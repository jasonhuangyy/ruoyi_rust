use std::sync::Arc;

use super::model::ListLogininforQuery;
use common::error::AppError;
use common::page::TableDataInfo;
use entity::prelude::SysLogininforModel;
use rust_xlsxwriter::Workbook;
use sea_orm::{ActiveModelTrait, DatabaseConnection, IntoActiveModel};
use sqlx::{Postgres, QueryBuilder};
use tracing::{info, instrument};

/// 新增一条登录日志记录
pub async fn add_logininfor(db: Arc<DatabaseConnection>, log: SysLogininforModel) -> Result<(), AppError> {
    info!(
        "[SERVICE] Preparing to add login information log for user: {:?}",
        log.user_name
    );

    // sqlx::query!(
    //     "INSERT INTO sys_logininfor (user_name, ipaddr, login_location, browser, os, status, msg, login_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    //     log.user_name,
    //     log.ipaddr,
    //     log.login_location,
    //     log.browser,
    //     log.os,
    //     log.status,
    //     log.msg,
    //     log.login_time,
    // )
    // .execute(db.get_postgres_connection_pool())
    // .await?;

    log.into_active_model().insert(db.as_ref()).await?;
    info!("[SERVICE] Login information log added successfully.");
    Ok(())
}

/// 查询登录日志列表（分页）
pub async fn select_logininfor_list(db: &DatabaseConnection, params: ListLogininforQuery) -> Result<TableDataInfo<SysLogininforModel>, AppError> {
    info!(
        "[SERVICE] Entering select_logininfor_list with params: {:?}",
        params
    );

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_logininfor WHERE 1=1");
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM sys_logininfor WHERE 1=1");

    if let Some(ipaddr) = params.ipaddr {
        if !ipaddr.trim().is_empty() {
            let condition = format!("%{}%", ipaddr);
            query_builder
                .push(" AND ipaddr LIKE ")
                .push_bind(condition.clone());
            count_builder.push(" AND ipaddr LIKE ").push_bind(condition);
        }
    }
    if let Some(user_name) = params.user_name {
        if !user_name.trim().is_empty() {
            let condition = format!("%{}%", user_name);
            query_builder
                .push(" AND user_name LIKE ")
                .push_bind(condition.clone());
            count_builder
                .push(" AND user_name LIKE ")
                .push_bind(condition);
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
    if let Some(begin_time) = params.begin_time {
        if !begin_time.trim().is_empty() {
            query_builder
                .push(" AND date_format(login_time,'%y%m%d') >= date_format(")
                .push_bind(begin_time.clone())
                .push(",'%y%m%d')");
            count_builder
                .push(" AND date_format(login_time,'%y%m%d') >= date_format(")
                .push_bind(begin_time)
                .push(",'%y%m%d')");
        }
    }
    if let Some(end_time) = params.end_time {
        if !end_time.trim().is_empty() {
            query_builder
                .push(" AND date_format(login_time,'%y%m%d') <= date_format(")
                .push_bind(end_time.clone())
                .push(",'%y%m%d')");
            count_builder
                .push(" AND date_format(login_time,'%y%m%d') <= date_format(")
                .push_bind(end_time)
                .push(",'%y%m%d')");
        }
    }

    let total: (i64,) = count_builder
        .build_query_as()
        .fetch_one(db.get_postgres_connection_pool())
        .await?;

    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page_num - 1) * page_size;

    query_builder
        .push(" ORDER BY login_time DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);

    info!("[DB_QUERY] Executing query for logininfor list (using QueryBuilder)");
    let rows: Vec<SysLogininforModel> = query_builder
        .build_query_as()
        .fetch_all(db.get_postgres_connection_pool())
        .await?;
    info!(
        "[DB_RESULT] Found {} logininfors for the current page.",
        rows.len()
    );

    Ok(TableDataInfo::new(rows, total.0))
}
/// 批量删除登录日志
pub async fn delete_logininfor_by_ids(db: &DatabaseConnection, info_ids: &[i64]) -> Result<u64, AppError> {
    info!(
        "[SERVICE] Entering delete_logininfor_by_ids with ids: {:?}",
        info_ids
    );
    let params = info_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM sys_logininfor WHERE info_id IN ({})", params);

    let mut query = sqlx::query(&sql);
    for id in info_ids {
        query = query.bind(id);
    }

    let result = query.execute(db.get_postgres_connection_pool()).await?;
    info!(
        "[DB_RESULT] Deleted {} logininfors.",
        result.rows_affected()
    );
    Ok(result.rows_affected())
}

/// 清空所有登录日志
pub async fn clean_logininfor(db: &DatabaseConnection) -> Result<u64, AppError> {
    info!("[SERVICE] Entering clean_logininfor");
    let result = sqlx::query("TRUNCATE TABLE sys_logininfor")
        .execute(db.get_postgres_connection_pool())
        .await?;
    info!("[DB_RESULT] Truncated sys_logininfor table.");
    Ok(result.rows_affected())
}

#[instrument(skip(db, params))]
pub async fn export_logininfor_list(db: &DatabaseConnection, params: ListLogininforQuery) -> Result<Vec<u8>, AppError> {
    info!(
        "[SERVICE] Starting logininfor list export with params: {:?}",
        params
    );

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_logininfor WHERE 1=1");

    if let Some(ipaddr) = params.ipaddr {
        if !ipaddr.trim().is_empty() {
            query_builder
                .push(" AND ipaddr LIKE ")
                .push_bind(format!("%{}%", ipaddr));
        }
    }
    if let Some(user_name) = params.user_name {
        if !user_name.trim().is_empty() {
            query_builder
                .push(" AND user_name LIKE ")
                .push_bind(format!("%{}%", user_name));
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            query_builder.push(" AND status = ").push_bind(status);
        }
    }
    if let Some(begin_time) = params.begin_time {
        if !begin_time.trim().is_empty() {
            query_builder
                .push(" AND date_format(login_time,'%y%m%d') >= date_format(")
                .push_bind(begin_time)
                .push(",'%y%m%d')");
        }
    }
    if let Some(end_time) = params.end_time {
        if !end_time.trim().is_empty() {
            query_builder
                .push(" AND date_format(login_time,'%y%m%d') <= date_format(")
                .push_bind(end_time)
                .push(",'%y%m%d')");
        }
    }
    query_builder.push(" ORDER BY login_time DESC");

    let login_logs: Vec<SysLogininforModel> = query_builder
        .build_query_as()
        .fetch_all(db.get_postgres_connection_pool())
        .await?;
    info!(
        "[DB_RESULT] Fetched {} login logs for export.",
        login_logs.len()
    );
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let headers = [
        "访问编号",
        "用户名称",
        "登录地址",
        "登录地点",
        "浏览器",
        "操作系统",
        "登录状态",
        "操作信息",
        "登录日期",
    ];
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }
    for (row_num, log) in login_logs.iter().enumerate() {
        let row = (row_num + 1) as u32;

        let status_str = if log.status.as_deref() == Some("0") {
            "成功"
        } else {
            "失败"
        };

        worksheet.write(row, 0, log.info_id)?;
        worksheet.write(row, 1, log.user_name.as_deref().unwrap_or(""))?;
        worksheet.write(row, 2, log.ipaddr.as_deref().unwrap_or(""))?;
        worksheet.write(row, 3, log.login_location.as_deref().unwrap_or(""))?;
        worksheet.write(row, 4, log.browser.as_deref().unwrap_or(""))?;
        worksheet.write(row, 5, log.os.as_deref().unwrap_or(""))?;
        worksheet.write(row, 6, status_str)?;
        worksheet.write(row, 7, log.msg.as_deref().unwrap_or(""))?;
        worksheet.write(
            row,
            8,
            log.login_time.map_or("".to_string(), |t| {
                t.format("%Y-%m-%d %H:%M:%S").to_string()
            }),
        )?;
    }
    let buffer = workbook.save_to_buffer()?;
    info!(
        "[SERVICE] Excel buffer created successfully, size: {} bytes.",
        buffer.len()
    );

    Ok(buffer)
}
