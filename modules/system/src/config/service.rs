use super::model::{AddConfigVo, ListConfigQuery, UpdateConfigVo};
use common::{error::AppError, page::TableDataInfo};
use entity::prelude::*;
use framework::state::AppState;
use rust_xlsxwriter::Workbook;
use sea_orm::{DatabaseConnection, EntityTrait};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::sync::Arc;
use tracing::{error, info, instrument};

/// 分页查询参数配置列表
#[instrument(skip(db))]
pub async fn select_config_list(db: &DatabaseConnection, params: ListConfigQuery) -> Result<TableDataInfo<SysConfigModel>, AppError> {
    let db = db.get_postgres_connection_pool();
    // 构建基础查询
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("select config_id, config_name, config_key, config_value, config_type, create_by, create_time, update_by, update_time, remark from sys_config where 1=1");
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new("select count(*) from sys_config where 1=1");

    // 动态添加查询条件
    if let Some(config_name) = params.config_name {
        if !config_name.trim().is_empty() {
            let condition = format!("%{}%", config_name);
            // push_bind 会取得 condition 的所有权，所以生命周期问题解决
            query_builder
                .push(" and config_name like ")
                .push_bind(condition.clone());
            count_builder
                .push(" and config_name like ")
                .push_bind(condition);
        }
    }
    if let Some(config_key) = params.config_key {
        if !config_key.trim().is_empty() {
            let condition = format!("%{}%", config_key);
            query_builder
                .push(" and config_key like ")
                .push_bind(condition.clone());
            count_builder
                .push(" and config_key like ")
                .push_bind(condition);
        }
    }
    if let Some(config_type) = params.config_type {
        if !config_type.trim().is_empty() {
            // config_type 是从 params 移动过来的，可以直接绑定
            query_builder
                .push(" and config_type = ")
                .push_bind(config_type.clone());
            count_builder
                .push(" and config_type = ")
                .push_bind(config_type);
        }
    }
    if let Some(date_range) = params.date_range {
        if let Some(begin_time) = date_range.begin_time {
            if !begin_time.is_empty() {
                query_builder
                    .push(" and date_format(create_time,'%y%m%d') >= date_format(")
                    .push_bind(begin_time.clone())
                    .push(",'%y%m%d')");
                count_builder
                    .push(" and date_format(create_time,'%y%m%d') >= date_format(")
                    .push_bind(begin_time)
                    .push(",'%y%m%d')");
            }
        }
        if let Some(end_time) = date_range.end_time {
            if !end_time.is_empty() {
                query_builder
                    .push(" and date_format(create_time,'%y%m%d') <= date_format(")
                    .push_bind(end_time.clone())
                    .push(",'%y%m%d')");
                count_builder
                    .push(" and date_format(create_time,'%y%m%d') <= date_format(")
                    .push_bind(end_time)
                    .push(",'%y%m%d')");
            }
        }
    }

    // 执行总数查询
    let total: (i64,) = count_builder
        .build_query_as()
        .fetch_one(db)
        .await
        .map_err(|e| {
            error!("[DB_ERROR] Failed to count configs: {}", e);
            AppError::from(e)
        })?;

    // 分页处理
    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    query_builder
        .push(" order by create_time desc limit ")
        .push_bind((page_num - 1) * page_size)
        .push(", ")
        .push_bind(page_size);

    // 执行列表查询
    let rows = query_builder
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(|e| {
            error!("[DB_ERROR] Failed to fetch config list: {}", e);
            AppError::from(e)
        })?;

    Ok(TableDataInfo::new(rows, total.0))
}

/// 根据ID查询参数配置
#[instrument(skip(db))]
pub async fn select_config_by_id(db: &DatabaseConnection, config_id: i32) -> Result<SysConfigModel, AppError> {
    // sqlx::query_as::<_, SysConfig>("select * from sys_config where config_id = ?")
    //     .bind(config_id)
    //     .fetch_one(db)
    //     .await
    //     .map_err(AppError::from)
    let model: Option<SysConfigModel> = SysConfig::find_by_id(config_id).one(db).await?;
    model.ok_or_else(|| AppError::ValidationFailed(format!("参数 '{}' 不存在", config_id)))
}

/// 根据参数键名查询参数值 (带缓存)
#[instrument(skip(state))]
pub async fn select_config_value_by_key(state: &Arc<AppState>, config_key: &str) -> Result<String, AppError> {
    let db = state.db.get_postgres_connection_pool();
    if let Some(value) = state.cache.config_cache.get(config_key).await {
        info!("[CACHE_HIT] Config key '{}' found in cache.", config_key);
        return Ok(value);
    }

    info!(
        "[CACHE_MISS] Config key '{}' not found in cache. Querying database...",
        config_key
    );
    let config_value: (String,) = sqlx::query_as("select config_value from sys_config where config_key = ?")
        .bind(config_key)
        .fetch_one(db)
        .await
        .map_err(|e| {
            error!(
                "[DB_ERROR] Failed to fetch config by key '{}': {}",
                config_key, e
            );
            AppError::ValidationFailed(format!("参数 '{}' 不存在", config_key))
        })?;

    state
        .cache
        .config_cache
        .insert(config_key.to_string(), config_value.0.clone())
        .await;
    info!(
        "[CACHE_INSERT] Stored config key '{}' into cache.",
        config_key
    );

    Ok(config_value.0)
}

async fn check_config_key_unique(db: &DatabaseConnection, config_key: &str, config_id: Option<i32>) -> Result<(), AppError> {
    let mut query = QueryBuilder::new("select config_id from sys_config where config_key = ");
    query.push_bind(config_key.to_string());
    if let Some(id) = config_id {
        query.push(" and config_id <> ").push_bind(id);
    }
    let result: Option<(i32,)> = query
        .build_query_as()
        .fetch_optional(db.get_postgres_connection_pool())
        .await?;

    if result.is_some() {
        Err(AppError::ValidationFailed("参数键名已存在".to_string()))
    } else {
        Ok(())
    }
}

#[instrument(skip_all)]
pub async fn add_config(state: &Arc<AppState>, data: AddConfigVo, operator: &str) -> Result<u64, AppError> {
    let db = &state.db;
    check_config_key_unique(db, &data.config_key, None).await?;
    let result = sqlx::query("insert into sys_config (config_name, config_key, config_value, config_type, remark, create_by, create_time) values (?, ?, ?, ?, ?, ?, ?)")
        .bind(data.config_name)
        .bind(data.config_key.clone())
        .bind(data.config_value)
        .bind(data.config_type)
        .bind(data.remark)
        .bind(operator)
        .bind(chrono::Local::now().naive_local())
        .execute(db.get_postgres_connection_pool())
        .await?;
    state.cache.config_cache.invalidate(&data.config_key).await;
    info!(
        "[CACHE_INVALIDATE] Invalidated config cache for key: {}",
        data.config_key
    );
    Ok(result.rows_affected())
}

#[instrument(skip_all)]
pub async fn update_config(state: &Arc<AppState>, data: UpdateConfigVo, operator: &str) -> Result<u64, AppError> {
    let db = &state.db;
    check_config_key_unique(db, &data.config_key, Some(data.config_id)).await?;
    let result = sqlx::query("update sys_config set config_name=?, config_key=?, config_value=?, config_type=?, remark=?, update_by=?, update_time=? where config_id = ?")
        .bind(data.config_name)
        .bind(data.config_key.clone())
        .bind(data.config_value)
        .bind(data.config_type)
        .bind(data.remark)
        .bind(operator)
        .bind(chrono::Local::now().naive_local())
        .bind(data.config_id)
        .execute(db.get_postgres_connection_pool())
        .await?;
    state.cache.config_cache.invalidate(&data.config_key).await;
    info!(
        "[CACHE_INVALIDATE] Invalidated config cache for key: {}",
        data.config_key
    );
    Ok(result.rows_affected())
}

#[instrument(skip_all)]
pub async fn delete_config_by_ids(state: &Arc<AppState>, config_ids: &[i32]) -> Result<u64, AppError> {
    if config_ids.is_empty() {
        return Ok(0);
    }

    let db = &state.db;
    let mut tx = db.get_postgres_connection_pool().begin().await?;

    let mut check_builder = QueryBuilder::new("select count(*) from sys_config where config_type = 'Y' and config_id in (");
    let mut separated = check_builder.separated(",");
    for id in config_ids {
        separated.push_bind(*id);
    }
    separated.push_unseparated(")");
    let count: (i64,) = check_builder.build_query_as().fetch_one(&mut *tx).await?;
    if count.0 > 0 {
        return Err(AppError::ValidationFailed(
            "系统内置参数不能删除".to_string(),
        ));
    }

    let mut keys_builder = QueryBuilder::new("select config_key from sys_config where config_id in (");
    let mut separated_keys = keys_builder.separated(",");
    for id in config_ids {
        separated_keys.push_bind(*id);
    }
    separated_keys.push_unseparated(")");
    let keys_to_invalidate: Vec<String> = keys_builder
        .build_query_scalar()
        .fetch_all(&mut *tx)
        .await?;

    let mut delete_builder = QueryBuilder::new("delete from sys_config where config_id in (");
    let mut separated_delete = delete_builder.separated(",");
    for id in config_ids {
        separated_delete.push_bind(*id);
    }
    separated_delete.push_unseparated(")");
    let result = delete_builder.build().execute(&mut *tx).await?;

    tx.commit().await?;

    for key in keys_to_invalidate {
        state.cache.config_cache.invalidate(&key).await;
        info!(
            "[CACHE_INVALIDATE] Invalidated config cache for key: {}",
            key
        );
    }
    Ok(result.rows_affected())
}

#[instrument(skip(state))]
pub async fn refresh_cache(state: &Arc<AppState>) -> Result<(), AppError> {
    state.cache.config_cache.invalidate_all();
    info!("[CACHE_INVALIDATE] All config cache has been invalidated.");
    Ok(())
}

#[instrument(skip(db, params))]
pub async fn export_config_list(db: &DatabaseConnection, params: ListConfigQuery) -> Result<Vec<u8>, AppError> {
    info!(
        "[SERVICE] Starting config list export with params: {:?}",
        params
    );

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("select config_id, config_name, config_key, config_value, config_type, create_by, create_time,update_by, update_time, remark from sys_config where 1=1");

    if let Some(config_name) = params.config_name {
        if !config_name.trim().is_empty() {
            query_builder
                .push(" and config_name like ")
                .push_bind(format!("%{}%", config_name));
        }
    }
    if let Some(config_key) = params.config_key {
        if !config_key.trim().is_empty() {
            query_builder
                .push(" and config_key like ")
                .push_bind(format!("%{}%", config_key));
        }
    }
    if let Some(config_type) = params.config_type {
        if !config_type.trim().is_empty() {
            query_builder
                .push(" and config_type = ")
                .push_bind(config_type);
        }
    }
    if let Some(date_range) = params.date_range {
        if let Some(begin_time) = date_range.begin_time {
            if !begin_time.is_empty() {
                query_builder
                    .push(" and date_format(create_time,'%y%m%d') >= date_format(")
                    .push_bind(begin_time)
                    .push(",'%y%m%d')");
            }
        }
        if let Some(end_time) = date_range.end_time {
            if !end_time.is_empty() {
                query_builder
                    .push(" and date_format(create_time,'%y%m%d') <= date_format(")
                    .push_bind(end_time)
                    .push(",'%y%m%d')");
            }
        }
    }
    query_builder.push(" order by create_time desc");

    let configs: Vec<SysConfigModel> = query_builder
        .build_query_as()
        .fetch_all(db.get_postgres_connection_pool())
        .await?;
    info!("[DB_RESULT] Fetched {} configs for export.", configs.len());

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = [
        "参数主键",
        "参数名称",
        "参数键名",
        "参数键值",
        "系统内置",
        "创建时间",
        "备注",
    ];
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }

    for (row_num, config) in configs.iter().enumerate() {
        let row = (row_num + 1) as u32;
        worksheet.write(row, 0, config.config_id)?;
        worksheet.write(row, 1, config.config_name.as_deref().unwrap_or(""))?;
        worksheet.write(row, 2, config.config_key.as_deref().unwrap_or(""))?;
        worksheet.write(row, 3, config.config_value.as_deref().unwrap_or(""))?;
        worksheet.write(
            row,
            4,
            if config.config_type.as_deref() == Some("Y") {
                "是"
            } else {
                "否"
            },
        )?;
        worksheet.write(
            row,
            5,
            config.create_time.map_or("".to_string(), |t| {
                t.format("%Y-%m-%d %H:%M:%S").to_string()
            }),
        )?;
        worksheet.write(row, 6, config.remark.as_deref().unwrap_or(""))?;
    }

    let buffer = workbook.save_to_buffer()?;
    info!(
        "[SERVICE] Excel buffer created successfully, size: {} bytes.",
        buffer.len()
    );

    Ok(buffer)
}
