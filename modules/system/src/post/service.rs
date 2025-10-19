//! 岗位管理模块的服务层

use super::model::{AddPostVo, ListPostQuery, PostOptionVo, SysPost, UpdatePostVo};
use common::{error::AppError, page::TableDataInfo};
use rust_xlsxwriter::Workbook;
use sea_orm::DatabaseConnection;
use sqlx::{PgPool, Postgres, QueryBuilder};
use tracing::{error, info, instrument};

/// 分页查询岗位列表
#[instrument(skip(db))]
pub async fn select_post_list(db: &DatabaseConnection, params: ListPostQuery) -> Result<TableDataInfo<SysPost>, AppError> {
    let db = db.get_postgres_connection_pool();
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_post WHERE 1=1");
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT count(*) FROM sys_post WHERE 1=1");

    if let Some(post_code) = params.post_code {
        if !post_code.trim().is_empty() {
            // 尝试将输入的 post_code 解析为数字 (i64)
            let post_id_attempt = post_code.parse::<i64>();

            if let Ok(id) = post_id_attempt {
                // 如果能成功解析为数字，就认为用户可能想按 岗位编号 或 岗位编码 搜索
                query_builder
                    .push(" AND (post_code = ")
                    .push_bind(post_code.clone())
                    .push(" OR post_id = ")
                    .push_bind(id)
                    .push(")");
                count_builder
                    .push(" AND (post_code = ")
                    .push_bind(post_code.clone())
                    .push(" OR post_id = ")
                    .push_bind(id)
                    .push(")");
            } else {
                // 如果不能解析为数字，那肯定是按 岗位编码 搜索
                query_builder
                    .push(" AND post_code = ")
                    .push_bind(post_code.clone());
                count_builder.push(" AND post_code = ").push_bind(post_code);
            }
        }
    }

    if let Some(post_name) = params.post_name {
        if !post_name.trim().is_empty() {
            let condition = format!("%{}%", post_name);
            query_builder
                .push(" AND post_name LIKE ")
                .push_bind(condition.clone());
            count_builder
                .push(" AND post_name LIKE ")
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

    let total: (i64,) = count_builder
        .build_query_as()
        .fetch_one(db)
        .await
        .map_err(|e| {
            error!("[DB_ERROR] Failed to count posts: {}", e);
            AppError::from(e)
        })?;

    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    query_builder
        .push(" ORDER BY post_sort ASC, create_time DESC LIMIT ")
        .push_bind((page_num - 1) * page_size)
        .push(", ")
        .push_bind(page_size);

    let rows = query_builder
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(|e| {
            error!("[DB_ERROR] Failed to fetch post list: {}", e);
            AppError::from(e)
        })?;

    Ok(TableDataInfo::new(rows, total.0))
}

/// 根据ID查询岗位详情
#[instrument(skip(db))]
pub async fn select_post_by_id(db: &PgPool, post_id: i64) -> Result<SysPost, AppError> {
    sqlx::query_as("SELECT * FROM sys_post WHERE post_id = ?")
        .bind(post_id)
        .fetch_one(db)
        .await
        .map_err(AppError::from)
}

/// 检查岗位名称是否唯一
async fn check_post_name_unique(db: &PgPool, post_name: &str, post_id: Option<i64>) -> Result<(), AppError> {
    let mut query = QueryBuilder::new("SELECT post_id FROM sys_post WHERE post_name = ");
    query.push_bind(post_name);
    if let Some(id) = post_id {
        query.push(" AND post_id <> ").push_bind(id);
    }
    let result: Option<(i64,)> = query.build_query_as().fetch_optional(db).await?;
    if result.is_some() {
        Err(AppError::ValidationFailed("岗位名称已存在".to_string()))
    } else {
        Ok(())
    }
}

/// 检查岗位编码是否唯一
async fn check_post_code_unique(db: &PgPool, post_code: &str, post_id: Option<i64>) -> Result<(), AppError> {
    let mut query = QueryBuilder::new("SELECT post_id FROM sys_post WHERE post_code = ");
    query.push_bind(post_code);
    if let Some(id) = post_id {
        query.push(" AND post_id <> ").push_bind(id);
    }
    let result: Option<(i64,)> = query.build_query_as().fetch_optional(db).await?;
    if result.is_some() {
        Err(AppError::ValidationFailed("岗位编码已存在".to_string()))
    } else {
        Ok(())
    }
}

/// 新增岗位
#[instrument(skip(db, data, operator))]
pub async fn add_post(db: &PgPool, data: AddPostVo, operator: &str) -> Result<u64, AppError> {
    check_post_name_unique(db, &data.post_name, None).await?;
    check_post_code_unique(db, &data.post_code, None).await?;

    let result = sqlx::query("INSERT INTO sys_post (post_code, post_name, post_sort, status, remark, create_by, create_time) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(data.post_code)
        .bind(data.post_name)
        .bind(data.post_sort)
        .bind(data.status)
        .bind(data.remark)
        .bind(operator)
        .bind(chrono::Local::now().naive_local())
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

/// 修改岗位
#[instrument(skip(db, data, operator))]
pub async fn update_post(db: &PgPool, data: UpdatePostVo, operator: &str) -> Result<u64, AppError> {
    check_post_name_unique(db, &data.post_name, Some(data.post_id)).await?;
    check_post_code_unique(db, &data.post_code, Some(data.post_id)).await?;

    let result = sqlx::query("UPDATE sys_post SET post_code=?, post_name=?, post_sort=?, status=?, remark=?, update_by=?, update_time=? WHERE post_id = ?")
        .bind(data.post_code)
        .bind(data.post_name)
        .bind(data.post_sort)
        .bind(data.status)
        .bind(data.remark)
        .bind(operator)
        .bind(chrono::Local::now().naive_local())
        .bind(data.post_id)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

/// 批量删除岗位 (物理删除)
#[instrument(skip(db))]
pub async fn delete_post_by_ids(db: &PgPool, post_ids: &[i64]) -> Result<u64, AppError> {
    if post_ids.is_empty() {
        return Ok(0);
    }

    // RuoYi Java后台会检查岗位是否已分配给用户，这里为了简化，我们先直接删除。
    // 生产环境中应增加此校验。

    let mut query_builder = QueryBuilder::new("DELETE FROM sys_post WHERE post_id IN (");
    let mut separated = query_builder.separated(",");
    for id in post_ids {
        separated.push_bind(*id);
    }
    separated.push_unseparated(")");

    let result = query_builder.build().execute(db).await?;
    Ok(result.rows_affected())
}

/// 获取所有启用的岗位作为下拉选项
#[instrument(skip(db))]
pub async fn select_all_posts_options(db: &PgPool) -> Result<Vec<PostOptionVo>, AppError> {
    info!("[SERVICE] Fetching all enabled posts for dropdown options.");
    let posts = sqlx::query_as("SELECT post_id, post_name FROM sys_post WHERE status = '0' ORDER BY post_sort ASC")
        .fetch_all(db)
        .await?;
    Ok(posts)
}

/// 查询所有状态正常的岗位 (用于下拉框)
#[instrument(skip(db))]
pub async fn select_post_all(db: &PgPool) -> Result<Vec<SysPost>, AppError> {
    info!("[SERVICE] Fetching all enabled posts.");
    let posts = sqlx::query_as("SELECT * FROM sys_post WHERE status = '0' ORDER BY post_sort ASC")
        .fetch_all(db)
        .await?;
    Ok(posts)
}

/// 导出岗位列表为 Excel 文件
///
/// # Arguments
/// * `db` - 数据库连接池
/// * `params` - 查询参数，与列表查询一致
///
/// # Returns
/// 成功时返回包含 Excel 文件内容的 `Vec<u8>`，失败时返回 `AppError`
#[instrument(skip(db, params))]
pub async fn export_post_list(db: &PgPool, params: ListPostQuery) -> Result<Vec<u8>, AppError> {
    info!(
        "[SERVICE] Starting post list export with params: {:?}",
        params
    );

    // 1. 查询所有符合条件的数据（注意：导出不分页）
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_post WHERE 1=1");
    // (这里的查询条件构建逻辑与 select_post_list 完全相同，只是没有分页)
    if let Some(post_code) = params.post_code {
        if !post_code.trim().is_empty() {
            query_builder
                .push(" AND post_code LIKE ")
                .push_bind(format!("%{}%", post_code));
        }
    }
    if let Some(post_name) = params.post_name {
        if !post_name.trim().is_empty() {
            query_builder
                .push(" AND post_name LIKE ")
                .push_bind(format!("%{}%", post_name));
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            query_builder.push(" AND status = ").push_bind(status);
        }
    }
    query_builder.push(" ORDER BY post_sort ASC, create_time DESC");

    let posts: Vec<SysPost> = query_builder.build_query_as().fetch_all(db).await?;
    info!("[DB_RESULT] Fetched {} posts for export.", posts.len());

    // 2. 创建 Excel 工作簿和工作表
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // 3. 写入表头
    let headers = [
        "岗位编号",
        "岗位编码",
        "岗位名称",
        "岗位排序",
        "状态",
        "创建时间",
    ];
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }

    // 4. 写入数据行
    for (row_num, post) in posts.iter().enumerate() {
        println!("{:?}", post);
        let row = (row_num + 1) as u32;
        worksheet.write(row, 0, post.post_id)?;
        worksheet.write(row, 1, &post.post_code)?;
        worksheet.write(row, 2, &post.post_name)?;
        worksheet.write(row, 3, post.post_sort)?;
        worksheet.write(
            row,
            4,
            if post.status == "0" {
                "正常"
            } else {
                "停用"
            },
        )?;
        worksheet.write(
            row,
            5,
            post.create_time.map_or("".to_string(), |t| {
                t.format("%Y-%m-%d %H:%M:%S").to_string()
            }),
        )?;
    }

    // 5. 将工作簿保存到内存中的字节缓冲区  使用 .map_err() 手动转换错误
    let buffer = workbook.save_to_buffer()?;

    info!(
        "[SERVICE] Excel buffer created successfully, size: {} bytes.",
        buffer.len()
    );

    Ok(buffer)
}
