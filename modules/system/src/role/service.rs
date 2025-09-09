use crate::role::model::{AddRoleVo, ChangeStatusVo, ListRoleQuery, SysRole, UpdateRoleVo};
use common::{
    error::AppError,
    page::TableDataInfo,
};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row, Transaction};
use tracing::{error, info, instrument};

/// 查询角色列表（分页）
pub async fn select_role_list(
    db: &MySqlPool,
    params: ListRoleQuery,
) -> Result<TableDataInfo<SysRole>, AppError> {
    info!("[SERVICE] Entering select_role_list with params: {:?}", params);
    // -- 动态构建SQL查询 --
    let mut sql = "SELECT * FROM sys_role WHERE del_flag = '0'".to_string();

    if let Some(name) =params .role_name {
        if !name.trim().is_empty() {
            sql.push_str(&format!(" AND role_name LIKE '%{}%'", name));
        }
    }
    if let Some(key) =params.role_key {
        if !key.trim().is_empty() {
            sql.push_str(&format!(" AND role_key LIKE '%{}%'", key));
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            sql.push_str(&format!(" AND status = '{}'", status));
        }
    }
    if let Some(begin_time) = params.begin_time {
        if !begin_time.trim().is_empty() {
            sql.push_str(&format!(" AND date_format(create_time,'%y%m%d') >= date_format('{}','%y%m%d')", begin_time));
        }
    }
    if let Some(end_time) = params.end_time {
        if !end_time.trim().is_empty() {
            sql.push_str(&format!(" AND date_format(create_time,'%y%m%d') <= date_format('{}','%y%m%d')", end_time));
        }
    }

    // 计算总数:需要克隆一份sql字符串，因为它在后续会被修改
    let count_sql = format!("SELECT COUNT(*) as count FROM ({}) temp_table", sql);
    let total: i64 = sqlx::query(&count_sql).fetch_one(db).await?.get("count");

    // 处理分页参数
    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page_num - 1) * page_size;

    // 添加排序和分页
    sql.push_str(&format!(" ORDER BY role_sort LIMIT {} OFFSET {}", page_size, offset));

    info!("[DB_QUERY] Executing query for role list: {}", sql);
    let rows: Vec<SysRole> = sqlx::query_as(&sql).fetch_all(db).await?;
    info!("[DB_RESULT] Found {} roles for the current page.", rows.len());

    Ok(TableDataInfo::new(rows, total))
}

/// 根据角色ID查询角色详情
pub async fn select_role_by_id(db: &MySqlPool, role_id: i64) -> Result<SysRole, AppError> {
    info!("[SERVICE] Entering select_role_by_id with role_id: {}", role_id);
    let role = sqlx::query_as!(SysRole, "SELECT * FROM sys_role WHERE role_id = ?", role_id)
        .fetch_one(db)
        .await?;
    info!("[DB_RESULT] Found role: {:?}", role);
    Ok(role)
}

/// 根据角色ID查询其关联的菜单ID列表
pub async fn select_menu_ids_by_role_id(db: &MySqlPool, role_id: i64) -> Result<Vec<i64>, AppError> {
    info!("[SERVICE] Entering select_menu_ids_by_role_id with role_id: {}", role_id);
    // `query_scalar` 可以直接将单列查询结果收集到 Vec 中
    let menu_ids: Vec<i64> = sqlx::query_scalar("SELECT menu_id FROM sys_role_menu WHERE role_id = ?")
        .bind(role_id)
        .fetch_all(db)
        .await?;
    info!("[DB_RESULT] Found {} menu_ids for role_id {}: {:?}", menu_ids.len(), role_id, menu_ids);
    Ok(menu_ids)
}

/// 新增角色，并处理其与菜单的关联关系（事务性）
pub async fn add_role(db: &MySqlPool, vo: AddRoleVo) -> Result<u64, AppError> {
    info!("[SERVICE] Entering add_role with vo: {:?}", vo);
    // 开启数据库事务
    let mut tx = db.begin().await.map_err(AppError::DatabaseError)?;
    info!("[TX] Transaction started for adding a new role.");

    // 1. 插入角色基本信息
    let result = sqlx::query!(
        "INSERT INTO sys_role (role_name, role_key, role_sort, status, remark, create_by, create_time) VALUES (?, ?, ?, ?, ?, 'admin', NOW())",
        vo.role_name, vo.role_key, vo.role_sort, vo.status, vo.remark
    )
        .execute(&mut *tx) // 在事务上执行
        .await?;

    let role_id = result.last_insert_id() as i64;
    info!("[TX] Inserted into sys_role, new role_id: {}", role_id);

    // 2. 插入角色和菜单的关联信息
    if let Some(menu_ids) = vo.menu_ids {
        if !menu_ids.is_empty() {
            insert_role_menu(&mut tx, role_id, &menu_ids).await?;
        }
    }

    // 提交事务
    tx.commit().await.map_err(AppError::DatabaseError)?;
    info!("[TX] Transaction committed successfully for role_id: {}", role_id);

    Ok(result.rows_affected())
}

/// 修改角色，并处理其与菜单的关联关系（事务性）
pub async fn update_role(db: &MySqlPool, vo: UpdateRoleVo) -> Result<u64, AppError> {
    info!("[SERVICE] Entering update_role with vo: {:?}", vo);
    let mut tx = db.begin().await?;
    info!("[TX] Transaction started for updating role_id: {}", vo.role_id);

    // 1. 更新角色基本信息
    let result = sqlx::query!(
        "UPDATE sys_role SET role_name = ?, role_key = ?, role_sort = ?, status = ?, remark = ?, update_by = 'admin', update_time = NOW() WHERE role_id = ?",
        vo.role_name, vo.role_key, vo.role_sort, vo.status, vo.remark, vo.role_id
    )
        .execute(&mut *tx)
        .await?;
    info!("[TX] Updated sys_role for role_id: {}.", vo.role_id);

    // 2. 删除旧的角色菜单关联
    sqlx::query!("DELETE FROM sys_role_menu WHERE role_id = ?", vo.role_id)
        .execute(&mut *tx)
        .await?;
    info!("[TX] Deleted old menu associations for role_id: {}.", vo.role_id);

    // 3. 插入新的角色菜单关联
    if let Some(menu_ids) = vo.menu_ids {
        if !menu_ids.is_empty() {
            insert_role_menu(&mut tx, vo.role_id, &menu_ids).await?;
        }
    }

    tx.commit().await?;
    info!("[TX] Transaction committed successfully for role_id: {}.", vo.role_id);
    Ok(result.rows_affected())
}

/// 批量删除角色（逻辑删除）
pub async fn delete_role_by_ids(db: &MySqlPool, role_ids: Vec<i64>) -> Result<u64, AppError> {
    info!("[SERVICE] Entering delete_role_by_ids with ids: {:?}", role_ids);
    let mut tx = db.begin().await?;
    info!("[TX] Transaction started for deleting roles.");

    // 构建 IN 子句
    let params = role_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    // 1. 逻辑删除 sys_role 表中的记录
    let sql_role = format!("UPDATE sys_role SET del_flag = '2' WHERE role_id IN ({})", params);
    let mut query_role = sqlx::query(&sql_role);
    for id in &role_ids {
        query_role = query_role.bind(id);
    }
    let result = query_role.execute(&mut *tx).await?;
    info!("[TX] Logically deleted roles from sys_role: {:?}", role_ids);

    // 2. 物理删除 sys_role_menu 表中的关联
    let sql_menu = format!("DELETE FROM sys_role_menu WHERE role_id IN ({})", params);
    let mut query_menu = sqlx::query(&sql_menu);
    for id in &role_ids {
        query_menu = query_menu.bind(id);
    }
    query_menu.execute(&mut *tx).await?;
    info!("[TX] Deleted menu associations from sys_role_menu for roles: {:?}", role_ids);

    tx.commit().await?;
    info!("[TX] Transaction committed for deleting roles.");
    Ok(result.rows_affected())
}

/// 修改角色状态
pub async fn change_role_status(db: &MySqlPool, vo: ChangeStatusVo) -> Result<u64, AppError> {
    info!("[SERVICE] Entering change_role_status with vo: {:?}", vo);
    let result = sqlx::query!(
        "UPDATE sys_role SET status = ? WHERE role_id = ?",
        vo.status, vo.role_id
    )
        .execute(db)
        .await?;
    info!("[DB_RESULT] Changed status for role_id: {} to {}", vo.role_id, vo.status);
    Ok(result.rows_affected())
}

/// 辅助函数：在事务中插入角色与菜单的关联记录
async fn insert_role_menu(
    tx: &mut Transaction<'_, MySql>,
    role_id: i64,
    menu_ids: &[i64]
) -> Result<(), AppError> {
    info!("[TX_HELPER] Inserting {} menu associations for role_id: {}", menu_ids.len(), role_id);
    // 构建批量插入的SQL
    let mut sql = "INSERT INTO sys_role_menu (role_id, menu_id) VALUES ".to_string();
    let mut values = Vec::new();
    for menu_id in menu_ids {
        values.push(format!("({}, {})", role_id, menu_id));
    }
    sql.push_str(&values.join(", "));

    sqlx::query(&sql).execute(&mut **tx).await?;
    info!("[TX_HELPER] Successfully inserted menu associations.");
    Ok(())
}

/// 根据角色键(role_key)列表查询对应的角色ID列表。
pub async fn select_role_ids_by_keys(db: &MySqlPool, role_keys: &[String]) -> Result<Vec<i64>, AppError> {
    if role_keys.is_empty() {
        // return Ok(vec![]);
        let msg = "role_keys 字段为空,无法获取角色";
        error!(msg);
        return Err(AppError::ValidationFailed(msg.to_string()));
    }

    let mut query_builder = QueryBuilder::new("SELECT role_id FROM sys_role WHERE role_key IN (");

    let mut separated = query_builder.separated(", ");
    for key in role_keys {
        separated.push_bind(key);
    }
    separated.push_unseparated(")");

    let query = query_builder.build_query_scalar();
    let role_ids = query.fetch_all(db).await?;

    info!("[SERVICE_ROLE] Found {} role IDs for keys: {:?}", role_ids.len(), role_keys);
    if role_ids.is_empty(){
        let msg = format!("role_keys 字段内容为[{}],系统中没找到角色,请在管理菜单中创建角色，[权限字符]要与function内容一致",role_keys.join(","));
        error!(msg);
        return Err(AppError::ValidationFailed(msg));
    }
    Ok(role_ids)
}


/// 查询所有状态正常的角色列表 (不分页)
///
/// 此函数专为用户管理、角色分配等需要全量角色列表的场景设计。
#[instrument(skip(db))]
pub async fn select_all_active_roles(db: &MySqlPool) -> Result<Vec<SysRole>, AppError> {
    info!("[SERVICE] Entering select_all_active_roles");

    let roles: Vec<SysRole> = sqlx::query_as(
        "SELECT * FROM sys_role WHERE status = '0' AND del_flag = '0' ORDER BY role_sort"
    )
        .fetch_all(db)
        .await?;

    info!("[DB_RESULT] Found {} active roles.", roles.len());
    Ok(roles)
}