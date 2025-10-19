use super::model::{ListJobLogQuery, SysJobLog};
use common::{error::AppError, page::TableDataInfo};
use rust_xlsxwriter::Workbook;
use sea_orm::DatabaseConnection;
use sqlx::{Postgres, QueryBuilder};
use tracing::{info, instrument};

#[instrument(skip(db, ids))]
pub async fn delete_job_log_by_ids(db: &DatabaseConnection, ids: &[i64]) -> Result<u64, AppError> {
    info!("[SERVICE] Deleting job logs with IDs: {:?}", ids);

    if ids.is_empty() {
        return Ok(0);
    }

    let mut query_builder = QueryBuilder::new("DELETE FROM sys_job_log WHERE job_log_id IN (");
    let mut separated = query_builder.separated(",");
    for id in ids {
        separated.push_bind(*id);
    }
    separated.push_unseparated(")");

    let result = query_builder
        .build()
        .execute(db.get_postgres_connection_pool())
        .await?;
    let affected_rows = result.rows_affected();
    info!("[DB_RESULT] Deleted {} job logs.", affected_rows);

    Ok(affected_rows)
}

#[instrument(skip(db))]
pub async fn clean_job_log(db: &DatabaseConnection) -> Result<(), AppError> {
    info!("[SERVICE] Cleaning all job logs.");

    sqlx::query("TRUNCATE TABLE sys_job_log")
        .execute(db.get_postgres_connection_pool())
        .await?;

    info!("[DB_RESULT] sys_job_log table has been truncated.");
    Ok(())
}

pub async fn select_job_log_list(db: &DatabaseConnection, params: ListJobLogQuery) -> Result<TableDataInfo<SysJobLog>, AppError> {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_job_log WHERE 1=1");
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM sys_job_log WHERE 1=1");
    let db = db.get_postgres_connection_pool();

    if let Some(name) = params.job_name {
        if !name.trim().is_empty() {
            let condition = format!("%{}%", name);
            query_builder
                .push(" AND job_name LIKE ")
                .push_bind(condition.clone());
            count_builder
                .push(" AND job_name LIKE ")
                .push_bind(condition);
        }
    }
    if let Some(group) = params.job_group {
        if !group.trim().is_empty() {
            query_builder
                .push(" AND job_group = ")
                .push_bind(group.clone());
            count_builder.push(" AND job_group = ").push_bind(group);
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            query_builder
                .push(" AND status = ")
                .push_bind(status.clone());
            count_builder.push(" AND status = ").push_bind(status);
        }
    }
    // 时间范围查询 占位

    let total: (i64,) = count_builder.build_query_as().fetch_one(db).await?;

    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    query_builder
        .push(" ORDER BY create_time DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page_num - 1) * page_size);

    let rows = query_builder.build_query_as().fetch_all(db).await?;

    Ok(TableDataInfo::new(rows, total.0))
}

#[instrument(skip(db, params))]
pub async fn export_job_log_list(db: &DatabaseConnection, params: ListJobLogQuery) -> Result<Vec<u8>, AppError> {
    info!(
        "[SERVICE] Starting job log list export with params: {:?}",
        params
    );

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_job_log WHERE 1=1");

    if let Some(name) = params.job_name {
        if !name.trim().is_empty() {
            query_builder
                .push(" AND job_name LIKE ")
                .push_bind(format!("%{}%", name));
        }
    }
    query_builder.push(" ORDER BY create_time DESC");

    let logs: Vec<SysJobLog> = query_builder
        .build_query_as()
        .fetch_all(db.get_postgres_connection_pool())
        .await?;
    info!("[DB_RESULT] Fetched {} job logs for export.", logs.len());

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = [
        "日志ID",
        "任务名称",
        "任务组名",
        "调用目标字符串",
        "日志信息",
        "执行状态",
        "执行时间",
    ];
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }

    for (row_num, log) in logs.iter().enumerate() {
        let row = (row_num + 1) as u32;
        let status_str = if log.status.as_deref() == Some("0") {
            "正常"
        } else {
            "失败"
        };

        worksheet.write(row, 0, log.job_log_id)?;
        worksheet.write(row, 1, &log.job_name)?;
        worksheet.write(row, 2, &log.job_group)?;
        worksheet.write(row, 3, &log.invoke_target)?;
        worksheet.write(row, 4, log.job_message.as_deref().unwrap_or(""))?;
        worksheet.write(row, 5, status_str)?;
        worksheet.write(
            row,
            6,
            log.create_time.map_or("".to_string(), |t| {
                t.format("%Y-%m-%d %H:%M:%S").to_string()
            }),
        )?;
    }

    let buffer = workbook.save_to_buffer()?;
    info!(
        "[SERVICE] Excel buffer for job logs created, size: {} bytes.",
        buffer.len()
    );

    Ok(buffer)
}
