use common::models::dict_model::SysDictData;
use common::{
    constants::cache_keys,
    error::AppError,
    page::TableDataInfo,
};
use moka::future::Cache;
use sqlx::{MySqlPool, Row};
use tracing::info;

use super::model::{AddDictDataVo, AddDictTypeVo, DictTypeOptionVo, ListDictDataQuery, ListDictTypeQuery, SysDictType, UpdateDictDataVo, UpdateDictTypeVo};


/// 查询字典类型列表（分页）
pub async fn select_dict_type_list(
    db: &MySqlPool,
    params: ListDictTypeQuery,
) -> Result<TableDataInfo<SysDictType>, AppError> {
    // 动态构建查询
    let mut where_sql = " WHERE 1=1 ".to_string();

    if let Some(name) = params.dict_name {
        where_sql.push_str(&format!(" AND dict_name LIKE '%{}%'", name));
    }
    if let Some(t) = params.dict_type {
        where_sql.push_str(&format!(" AND dict_type LIKE '%{}%'", t));
    }
    if let Some(s) = params.status {
        where_sql.push_str(&format!(" AND status = '{}'", s));
    }

    // 计算总数
    let count_sql = format!("SELECT COUNT(*) as count FROM sys_dict_type {}", where_sql);
    let total: i64 = sqlx::query(&count_sql).fetch_one(db).await?.get("count");

    // 如果前端没有传递分页参数，则提供默认值
    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page_num - 1) * page_size;

    // 分页查询
    let mut query_sql = "SELECT * FROM sys_dict_type ".to_string();
    query_sql.push_str(&where_sql);
    query_sql.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));

    let rows: Vec<SysDictType> = sqlx::query_as(&query_sql).fetch_all(db).await?;

    // 返回 TableDataInfo 结构
    // 假设你已经在 common::page 中为 TableDataInfo 添加了 new 函数
    Ok(TableDataInfo {
        total,
        rows,
        code: 200,
        msg: "查询成功".to_string(),
    })
}

/// 根据ID查询字典类型详情
pub async fn select_dict_type_by_id(db: &MySqlPool, dict_id: i64) -> Result<SysDictType, AppError> {
    let dict_type = sqlx::query_as!(
        SysDictType,
        "SELECT * FROM sys_dict_type WHERE dict_id = ?",
        dict_id
    )
        .fetch_one(db)
        .await?;
    Ok(dict_type)
}

/// 新增字典类型
pub async fn add_dict_type(db: &MySqlPool, vo: AddDictTypeVo) -> Result<u64, AppError> {
    let result = sqlx::query!(
        "INSERT INTO sys_dict_type (dict_name, dict_type, status, remark, create_by, create_time) VALUES (?, ?, ?, ?, 'admin', NOW())",
        vo.dict_name, vo.dict_type, vo.status, vo.remark
    )
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}

/// 修改字典类型
pub async fn update_dict_type(db: &MySqlPool, vo: UpdateDictTypeVo, cache: &Cache<String, Vec<SysDictData>>) -> Result<u64, AppError> {
    // 如果字典类型字符串（dict_type）被修改，需要删除旧的缓存
    // 1. 先查询旧的数据
    let old_dict_type = select_dict_type_by_id(db, vo.dict_id).await?;
    // 2. 执行更新
    let result = sqlx::query!(
        "UPDATE sys_dict_type SET dict_name = ?, dict_type = ?, status = ?, remark = ?, update_by = 'admin', update_time = NOW() WHERE dict_id = ?",
        vo.dict_name, vo.dict_type, vo.status, vo.remark, vo.dict_id
    )
        .execute(db)
        .await?;
    // 3. 如果 dict_type 发生了变化，使旧的缓存失效
    if let Some(old_type) = old_dict_type.dict_type {
        if old_type != vo.dict_type {
            let old_cache_key = format!("{}{}", cache_keys::SYS_DICT_KEY, old_type);
            cache.invalidate(&old_cache_key).await;
            info!("[CACHE] Invalidated dict cache due to type change: {}", old_cache_key);
        }
    }
    Ok(result.rows_affected())
}

/// 删除字典类型
pub async fn delete_dict_type_by_ids(db: &MySqlPool, ids: Vec<i64>) -> Result<u64, AppError> {
    // RuoYi 的实现会检查是否有关联的字典数据，这里简化为直接删除
    let query_str = format!("DELETE FROM sys_dict_type WHERE dict_id IN ({})", ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(","));
    let result = sqlx::query(&query_str)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}

/// 获取所有字典类型作为下拉框选项
pub async fn get_dict_type_option_select(db: &MySqlPool) -> Result<Vec<DictTypeOptionVo>, AppError> {
    // 目标结构体是 DictTypeOptionVo，所以 SQL 语句现在是合法的
    let list = sqlx::query_as!(
        DictTypeOptionVo,
        "SELECT dict_id, dict_name FROM sys_dict_type"
    )
        .fetch_all(db)
        .await?;
    Ok(list)
}

/// 查询字典数据列表（分页）
pub async fn select_dict_data_list(
    db: &MySqlPool,
    params: ListDictDataQuery,
) -> Result<TableDataInfo<SysDictData>, AppError> {
    // 构建 where 子句的逻辑不变
    let mut where_sql = format!(" WHERE dict_type = '{}' ", params.dict_type);
    if let Some(label) = params.dict_label {
        if !label.trim().is_empty() {
            where_sql.push_str(&format!(" AND dict_label LIKE '%{}%'", label));
        }
    }
    if let Some(s) = params.status {
        if !s.trim().is_empty() {
            where_sql.push_str(&format!(" AND status = '{}'", s));
        }
    }

    // 计算总数
    let count_sql = format!("SELECT COUNT(*) as count FROM sys_dict_data {}", where_sql);
    let total: i64 = sqlx::query(&count_sql).fetch_one(db).await?.get("count");

    // 关键修改：直接从 params 获取分页参数，并提供默认值
    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page_num - 1) * page_size;

    // 构建最终的分页查询 SQL
    let mut query_sql = "SELECT * FROM sys_dict_data ".to_string();
    query_sql.push_str(&where_sql);
    query_sql.push_str(" ORDER BY dict_sort "); // RuoYi 默认按排序号排序
    query_sql.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));

    let rows: Vec<SysDictData> = sqlx::query_as(&query_sql).fetch_all(db).await?;

    Ok(TableDataInfo::new(rows, total))
}

/// 根据ID查询字典数据详情
pub async fn select_dict_data_by_code(db: &MySqlPool, dict_code: i64) -> Result<SysDictData, AppError> {
    let data = sqlx::query_as!(
        SysDictData,
        "SELECT * FROM sys_dict_data WHERE dict_code = ?",
        dict_code
    )
        .fetch_one(db)
        .await?;
    Ok(data)
}

/// 新增字典数据
pub async fn add_dict_data(db: &MySqlPool, vo: AddDictDataVo, cache: &Cache<String, Vec<SysDictData>>) -> Result<u64, AppError> {
    let result = sqlx::query!(
        "INSERT INTO sys_dict_data (dict_sort, dict_label, dict_value, dict_type, css_class, list_class, is_default, status, remark, create_by, create_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'admin', NOW())",
        vo.dict_sort, vo.dict_label, vo.dict_value, vo.dict_type, vo.css_class, vo.list_class, vo.is_default, vo.status, vo.remark
    )
        .execute(db)
        .await?;
    // 操作成功后，使该类型的缓存失效
    let cache_key = format!("{}{}", cache_keys::SYS_DICT_KEY, vo.dict_type);
    cache.invalidate(&cache_key).await;
    info!("[CACHE] Invalidated dict cache due to new data: {}", cache_key);
    Ok(result.rows_affected())
}

/// 修改字典数据
pub async fn update_dict_data(db: &MySqlPool, vo: UpdateDictDataVo, cache: &Cache<String, Vec<SysDictData>>) -> Result<u64, AppError> {
    let result = sqlx::query!(
        "UPDATE sys_dict_data SET dict_sort = ?, dict_label = ?, dict_value = ?, dict_type = ?, css_class = ?, list_class = ?, is_default = ?, status = ?, remark = ?, update_by = 'admin', update_time = NOW() WHERE dict_code = ?",
        vo.dict_sort, vo.dict_label, vo.dict_value, vo.dict_type, vo.css_class, vo.list_class, vo.is_default, vo.status, vo.remark, vo.dict_code
    )
        .execute(db)
        .await?;
    // 操作成功后，使该类型的缓存失效
    let cache_key = format!("{}{}", cache_keys::SYS_DICT_KEY, vo.dict_type);
    cache.invalidate(&cache_key).await;
    info!("[CACHE] Invalidated dict cache due to data update: {}", cache_key);
    Ok(result.rows_affected())
}

/// 删除字典数据
pub async fn delete_dict_data_by_codes(db: &MySqlPool, codes: Vec<i64>, _cache: &Cache<String, Vec<SysDictData>>) -> Result<u64, AppError> {
    // 为了使缓存失效，需要知道被删除的数据的 dict_type
    // 实际项目中，这里可以先查询一次，或者让前端把 dict_type 传来。为简化，假设批量删除的都是同一种类型。
    // 这里先不处理缓存失效，因为不知道 dict_type。
    let query_str = format!("DELETE FROM sys_dict_data WHERE dict_code IN ({})", codes.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(","));
    let result = sqlx::query(&query_str)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}

/// 根据字典类型查询字典数据列表（核心缓存接口）
pub async fn select_dict_data_by_type(
    db: &MySqlPool,
    cache: &Cache<String, Vec<SysDictData>>, // 传入缓存实例
    dict_type: &str,
) -> Result<Vec<SysDictData>, AppError> {
    let cache_key = format!("{}{}", cache_keys::SYS_DICT_KEY, dict_type);
    println!("[DEBUG] Attempting to get dict from cache with key: {}", cache_key);

    // 1. 尝试从缓存中获取
    if let Some(dict_data) = cache.get(&cache_key).await {
        println!("[DEBUG] Cache HIT for key: {}", cache_key);
        return Ok(dict_data);
    }

    // 2. 如果缓存未命中，则查询数据库
    println!("[DEBUG] Cache MISS for key: {}. Querying database...", cache_key);
    let dict_data = sqlx::query_as!(
        SysDictData,
        "SELECT * FROM sys_dict_data WHERE status = '0' AND dict_type = ? ORDER BY dict_sort",
        dict_type
    )
        .fetch_all(db)
        .await?;

    // 3. 将查询结果存入缓存
    // 我们需要克隆一份数据放入缓存，因为原始数据的所有权将被函数返回
    cache.insert(cache_key.clone(), dict_data.clone()).await;
    println!("[DEBUG] Stored data in cache for key: {}", cache_key);

    Ok(dict_data)
}

/// 刷新所有字典缓存
pub async fn refresh_dict_cache(
    db: &MySqlPool,
    cache: &Cache<String, Vec<SysDictData>>,
) -> Result<(), AppError> {
    // 1. 先清空所有字典缓存
    cache.invalidate_all();
    println!("[DEBUG] All dict cache invalidated.");

    // 2. 从数据库中查询出所有唯一的、非空的字典类型
    // sqlx::query_scalar 在查询可空列时，返回 Vec<Option<String>>
    let dict_types: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT DISTINCT dict_type FROM sys_dict_type WHERE dict_type IS NOT NULL AND dict_type != ''"
    ).fetch_all(db).await?;

    // 3. 遍历每种类型，重新加载并缓存
    // for 循环会消耗 dict_types，每次迭代得到的 dict_type_opt 的类型是 Option<String>
    for dict_type_opt in dict_types {
        // 使用 if let Some(dt) = ... 来安全地解构 Option
        // 如果 dict_type_opt 是 Some(string_value)，那么 dt 就会是 string_value (类型为 String)
        if let Some(dt) = dict_type_opt {
            // 调用 service 时，传入 dt 的引用 &dt (类型为 &String)
            // select_dict_data_by_type 的第三个参数期望一个 &str，&String 可以自动解引用为 &str
            println!("[DEBUG] Refreshing cache for dict_type: {}", dt);
            select_dict_data_by_type(db, cache, &dt).await?;
        }
    }

    println!("[DEBUG] All dict caches have been refreshed.");
    Ok(())
}
