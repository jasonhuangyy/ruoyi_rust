use super::model::{ListOperLogQuery, SysOperLog};
use common::error::AppError;
use common::page::TableDataInfo;
use rust_xlsxwriter::{Workbook, XlsxError};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use tracing::{info, instrument};

/// 新增一条操作日志记录
///
/// # 异步说明
/// 这个函数将被设计为在后台任务中调用 (`tokio::spawn`)，
/// 以避免阻塞主请求的响应。
pub async fn add_oper_log(db: &MySqlPool, log: SysOperLog) -> Result<(), AppError> {
    info!("[SERVICE] Preparing to add operation log: {:?}", log.title);

    // --- 数据库插入逻辑将在这里实现 ---
    sqlx::query!(
        r#"
        INSERT INTO sys_oper_log (title, business_type, method, request_method, operator_type, oper_name, dept_name, oper_url, oper_ip, oper_location, oper_param, json_result, status, error_msg, oper_time, cost_time)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        log.title,
        log.business_type,
        log.method,
        log.request_method,
        log.operator_type,
        log.oper_name,
        log.dept_name,
        log.oper_url,
        log.oper_ip,
        log.oper_location,
        log.oper_param,
        log.json_result,
        log.status,
        log.error_msg,
        log.oper_time, // `chrono::NaiveDateTime`可以直接被sqlx使用
        log.cost_time,
    )
        .execute(db)
        .await?;

    info!("[SERVICE] Operation log added successfully.");
    Ok(())
}


/// 查询操作日志列表（分页）
pub async fn select_oper_log_list(
    db: &MySqlPool,
    params: ListOperLogQuery,
) -> Result<TableDataInfo<SysOperLog>, AppError> {
    info!("[SERVICE] Entering select_oper_log_list with params: {:?}", params);

    let mut sql = "SELECT * FROM sys_oper_log WHERE 1=1".to_string();

    if let Some(title) = params.title {
        if !title.trim().is_empty() {
            sql.push_str(&format!(" AND title LIKE '%{}%'", title));
        }
    }
    if let Some(oper_name) = params.oper_name {
        if !oper_name.trim().is_empty() {
            sql.push_str(&format!(" AND oper_name LIKE '%{}%'", oper_name));
        }
    }
    if let Some(business_type) = params.business_type {
        sql.push_str(&format!(" AND business_type = {}", business_type));
    }
    if let Some(status) = params.status {
        sql.push_str(&format!(" AND status = {}", status));
    }
    if let Some(begin_time) = params.begin_time {
        if !begin_time.trim().is_empty() {
            sql.push_str(&format!(" AND date_format(oper_time,'%y%m%d') >= date_format('{}','%y%m%d')", begin_time));
        }
    }
    if let Some(end_time) = params.end_time {
        if !end_time.trim().is_empty() {
            sql.push_str(&format!(" AND date_format(oper_time,'%y%m%d') <= date_format('{}','%y%m%d')", end_time));
        }
    }

    let count_sql = format!("SELECT COUNT(*) as count FROM ({}) temp_table", sql);
    let total: i64 = sqlx::query(&count_sql).fetch_one(db).await?.get("count");

    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page_num - 1) * page_size;

    // RuoYi 默认按操作时间倒序排列
    sql.push_str(&format!(" ORDER BY oper_time DESC LIMIT {} OFFSET {}", page_size, offset));

    info!("[DB_QUERY] Executing query for operlog list: {}", sql);
    let rows: Vec<SysOperLog> = sqlx::query_as(&sql).fetch_all(db).await?;
    info!("[DB_RESULT] Found {} operlogs for the current page.", rows.len());

    Ok(TableDataInfo::new(rows, total))
}

/// 批量删除操作日志
pub async fn delete_oper_log_by_ids(db: &MySqlPool, oper_ids: &[i64]) -> Result<u64, AppError> {
    info!("[SERVICE] Entering delete_oper_log_by_ids with ids: {:?}", oper_ids);
    let params = oper_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM sys_oper_log WHERE oper_id IN ({})", params);

    let mut query = sqlx::query(&sql);
    for id in oper_ids {
        query = query.bind(id);
    }

    let result = query.execute(db).await?;
    info!("[DB_RESULT] Deleted {} operlogs.", result.rows_affected());
    Ok(result.rows_affected())
}

/// 清空所有操作日志
pub async fn clean_oper_log(db: &MySqlPool) -> Result<u64, AppError> {
    info!("[SERVICE] Entering clean_oper_log");
    let result = sqlx::query("TRUNCATE TABLE sys_oper_log").execute(db).await?;
    info!("[DB_RESULT] Truncated sys_oper_log table.");
    Ok(result.rows_affected())
}

#[instrument(skip(db, params))]
pub async fn export_oper_log_list(
    db: &MySqlPool,
    params: ListOperLogQuery,
) -> Result<Vec<u8>, AppError> {
    info!("[SERVICE] Starting oper log list export with params: {:?}", params);

    let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new("SELECT * FROM sys_oper_log WHERE 1=1");

    if let Some(title) = params.title {
        if !title.trim().is_empty() {
            query_builder.push(" AND title LIKE ").push_bind(format!("%{}%", title));
        }
    }
    if let Some(oper_name) = params.oper_name {
        if !oper_name.trim().is_empty() {
            query_builder.push(" AND oper_name LIKE ").push_bind(format!("%{}%", oper_name));
        }
    }
    if let Some(business_type) = params.business_type {
        query_builder.push(" AND business_type = ").push_bind(business_type);
    }
    if let Some(status) = params.status {
        query_builder.push(" AND status = ").push_bind(status);
    }
    if let Some(begin_time) = params.begin_time {
        if !begin_time.trim().is_empty() {
            query_builder.push(" AND date_format(oper_time,'%y%m%d') >= date_format(").push_bind(begin_time).push(",'%y%m%d')");
        }
    }
    if let Some(end_time) = params.end_time {
        if !end_time.trim().is_empty() {
            query_builder.push(" AND date_format(oper_time,'%y%m%d') <= date_format(").push_bind(end_time).push(",'%y%m%d')");
        }
    }
    query_builder.push(" ORDER BY oper_time DESC");

    let oper_logs: Vec<SysOperLog> = query_builder.build_query_as().fetch_all(db).await?;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = ["日志主键", "系统模块", "操作类型", "请求方式", "操作人员", "主机", "操作地点", "操作状态", "操作日期", "消耗时间(ms)"];
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }

    for (row_num, log) in oper_logs.iter().enumerate() {
        let row = (row_num + 1) as u32;

        let business_type_str = match log.business_type {
            Some(0) => "其它",
            Some(1) => "新增",
            Some(2) => "修改",
            Some(3) => "删除",
            Some(4) => "授权",
            Some(5) => "导出",
            Some(6) => "导入",
            Some(7) => "强退",
            Some(8) => "生成代码",
            Some(9) => "清空数据",
            _ => "未知",
        };

        let status_str = if log.status == Some(0) { "正常" } else { "异常" };

        worksheet.write(row, 0, log.oper_id)?;
        worksheet.write(row, 1, log.title.as_deref().unwrap_or(""))?;
        worksheet.write(row, 2, business_type_str)?;
        worksheet.write(row, 3, log.request_method.as_deref().unwrap_or(""))?;
        worksheet.write(row, 4, log.oper_name.as_deref().unwrap_or(""))?;
        worksheet.write(row, 5, log.oper_ip.as_deref().unwrap_or(""))?;
        worksheet.write(row, 6, log.oper_location.as_deref().unwrap_or(""))?;
        worksheet.write(row, 7, status_str)?;
        worksheet.write(row, 8, log.oper_time.map_or("".to_string(), |t| t.format("%Y-%m-%d %H:%M:%S").to_string()))?;
        worksheet.write(row, 9, log.cost_time.unwrap_or(0))?;
    }

    let buffer = workbook.save_to_buffer()?;
    info!("[SERVICE] Excel buffer created successfully, size: {} bytes.", buffer.len());

    Ok(buffer)
}