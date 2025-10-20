use crate::role::model::{AddRoleVo, ChangeStatusVo, ListRoleQuery, UpdateRoleVo};
use common::{error::AppError, page::TableDataInfo};
use entity::prelude::{SysRole, SysRoleColumn, SysRoleMenu, SysRoleMenuColumn, SysRoleMenuModel, SysRoleModel};
use rust_xlsxwriter::Workbook;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, TransactionTrait,
};
use sqlx::{Postgres, QueryBuilder};
use tracing::{error, info, instrument};

/// 查询角色列表（分页）
pub async fn select_role_list(db: &DatabaseConnection, params: ListRoleQuery) -> Result<TableDataInfo<SysRoleModel>, AppError> {
    info!(
        "[SERVICE] Entering select_role_list with params: {:?}",
        params
    );
    let db = db.get_postgres_connection_pool();

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_role WHERE del_flag = '0'");
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM sys_role WHERE del_flag = '0'");

    if let Some(name) = params.role_name {
        if !name.trim().is_empty() {
            let condition = format!("%{}%", name);
            query_builder
                .push(" AND role_name LIKE ")
                .push_bind(condition.clone());
            count_builder
                .push(" AND role_name LIKE ")
                .push_bind(condition);
        }
    }
    if let Some(key) = params.role_key {
        if !key.trim().is_empty() {
            let condition = format!("%{}%", key);
            query_builder
                .push(" AND role_key LIKE ")
                .push_bind(condition.clone());
            count_builder
                .push(" AND role_key LIKE ")
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
                .push(" AND date_format(create_time,'%y%m%d') >= date_format(")
                .push_bind(begin_time.clone())
                .push(",'%y%m%d')");
            count_builder
                .push(" AND date_format(create_time,'%y%m%d') >= date_format(")
                .push_bind(begin_time)
                .push(",'%y%m%d')");
        }
    }
    if let Some(end_time) = params.end_time {
        if !end_time.trim().is_empty() {
            query_builder
                .push(" AND date_format(create_time,'%y%m%d') <= date_format(")
                .push_bind(end_time.clone())
                .push(",'%y%m%d')");
            count_builder
                .push(" AND date_format(create_time,'%y%m%d') <= date_format(")
                .push_bind(end_time)
                .push(",'%y%m%d')");
        }
    }

    let total: (i64,) = count_builder.build_query_as().fetch_one(db).await?;

    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page_num - 1) * page_size;

    query_builder
        .push(" ORDER BY role_sort LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);

    info!("[DB_QUERY] Executing safe, parameterized query for role list.");
    let rows: Vec<SysRoleModel> = query_builder.build_query_as().fetch_all(db).await?;
    info!(
        "[DB_RESULT] Found {} roles for the current page.",
        rows.len()
    );

    Ok(TableDataInfo::new(rows, total.0))
}

/// 根据角色ID查询角色详情
pub async fn select_role_by_id(db: &DatabaseConnection, role_id: i64) -> Result<SysRoleModel, AppError> {
    info!(
        "[SERVICE] Entering select_role_by_id with role_id: {}",
        role_id
    );

    let role = SysRole::find_by_id(role_id)
        .one(db)
        .await?
        .ok_or(AppError::RecordNotFound)?;
    info!("[DB_RESULT] Found role: {:?}", role);
    Ok(role)
}

/// 根据角色ID查询其关联的菜单ID列表
pub async fn select_menu_ids_by_role_id(db: &DatabaseConnection, role_id: i64) -> Result<Vec<i64>, AppError> {
    info!(
        "[SERVICE] Entering select_menu_ids_by_role_id with role_id: {}",
        role_id
    );
    // `query_scalar` 可以直接将单列查询结果收集到 Vec 中
    let menu_ids: Vec<i64> = sqlx::query_scalar("SELECT menu_id FROM sys_role_menu WHERE role_id = $1")
        .bind(role_id)
        .fetch_all(db.get_postgres_connection_pool())
        .await?;
    info!(
        "[DB_RESULT] Found {} menu_ids for role_id {}: {:?}",
        menu_ids.len(),
        role_id,
        menu_ids
    );
    Ok(menu_ids)
}

/// 新增角色，并处理其与菜单的关联关系（事务性）
pub async fn add_role(db: &DatabaseConnection, vo: AddRoleVo) -> Result<i64, AppError> {
    info!("[SERVICE] Entering add_role with vo: {:?}", vo);
    let tx = db
        .begin()
        .await
        .map_err(|arg0: sea_orm::DbErr| AppError::SeaOrmDbError(arg0))?;
    let mut model = SysRoleModel {
        role_name: vo.role_name,
        role_key: vo.role_key,
        role_sort: vo.role_sort,
        status: vo.status,
        remark: vo.remark,
        create_by: Some("admin".to_string()),
        ..Default::default()
    }
    .into_active_model();

    model.role_id = NotSet;
    model.create_time = NotSet;
    model.update_time = NotSet;

    let result = model.insert(db).await?;

    // info!("[TX] Transaction started for adding a new role.");
    // let result = sqlx::query!(
    //     "INSERT INTO sys_role (role_name, role_key, role_sort, status, remark, create_by, create_time) VALUES (?, ?, ?, ?, ?, 'admin', NOW())",
    //     vo.role_name,
    //     vo.role_key,
    //     vo.role_sort,
    //     vo.status,
    //     vo.remark
    // )
    // .execute(&mut *tx)
    // .await?;

    let role_id = result.role_id;
    info!("[TX] Inserted into sys_role, new role_id: {}", role_id);
    if let Some(menu_ids) = vo.menu_ids {
        if !menu_ids.is_empty() {
            insert_role_menu(&tx, role_id, &menu_ids).await?;
        }
    }
    tx.commit().await.map_err(|e| AppError::SeaOrmDbError(e))?;
    info!(
        "[TX] Transaction committed successfully for role_id: {}",
        role_id
    );

    Ok(result.role_id)
}

/// 修改角色，并处理其与菜单的关联关系（事务性）
pub async fn update_role(db: &DatabaseConnection, vo: UpdateRoleVo) -> Result<i64, AppError> {
    info!("[SERVICE] Entering update_role with vo: {:?}", vo);
    let tx = db.begin().await?;
    info!(
        "[TX] Transaction started for updating role_id: {}",
        vo.role_id
    );

    let exist = SysRole::find_by_id(vo.role_id)
        .one(db)
        .await?
        .ok_or(AppError::RecordNotFound)?;
    let mut active_model = exist.into_active_model();

    active_model.role_name = Set(vo.role_name);
    active_model.role_key = Set(vo.role_key);
    active_model.role_sort = Set(vo.role_sort);
    active_model.status = Set(vo.status);
    active_model.remark = Set(vo.remark);

    // let result = sqlx::query!(
    //     "UPDATE sys_role SET role_name = ?, role_key = ?, role_sort = ?, status = ?, remark = ?, update_by = 'admin', update_time = NOW() WHERE role_id = ?",
    //     vo.role_name,
    //     vo.role_key,
    //     vo.role_sort,
    //     vo.status,
    //     vo.remark,
    //     vo.role_id
    // )
    // .execute(&mut *tx)
    // .await?;
    let result = active_model.update(&tx).await?;
    info!("[TX] Updated sys_role for role_id: {}.", vo.role_id);

    SysRole::delete_by_id(vo.role_id).exec(&tx).await?;

    // sqlx::query!("DELETE FROM sys_role_menu WHERE role_id = ?", vo.role_id)
    //     .execute(&mut *tx)
    //     .await?;
    // info!(
    //     "[TX] Deleted old menu associations for role_id: {}.",
    //     vo.role_id
    // );

    if let Some(menu_ids) = vo.menu_ids {
        if !menu_ids.is_empty() {
            insert_role_menu(&tx, vo.role_id, &menu_ids).await?;
        }
    }

    tx.commit().await?;
    info!(
        "[TX] Transaction committed successfully for role_id: {}.",
        vo.role_id
    );
    Ok(result.role_id)
}

/// 批量删除角色（逻辑删除）
pub async fn delete_role_by_ids(db: &DatabaseConnection, role_ids: Vec<i64>) -> Result<u64, AppError> {
    info!(
        "[SERVICE] Entering delete_role_by_ids with ids: {:?}",
        role_ids
    );
    let tx = db.begin().await?;
    info!("[TX] Transaction started for deleting roles.");

    let result = SysRole::update_many()
        .col_expr(SysRoleColumn::DelFlag, "2".into())
        .filter(SysRoleColumn::RoleId.is_in(role_ids.clone()))
        .exec(&tx)
        .await?;

    // let params = role_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    // let sql_role = format!(
    //     "UPDATE sys_role SET del_flag = '2' WHERE role_id IN ({})",
    //     params
    // );
    // let mut query_role = sqlx::query(&sql_role);
    // for id in &role_ids {
    //     query_role = query_role.bind(id);
    // }
    // let result = query_role.execute(&mut *tx).await?;
    // info!("[TX] Logically deleted roles from sys_role: {:?}", role_ids);

    SysRoleMenu::delete_many()
        .filter(SysRoleMenuColumn::RoleId.is_in(role_ids.clone()))
        .exec(&tx)
        .await?;

    // let sql_menu = format!("DELETE FROM sys_role_menu WHERE role_id IN ({})", params);
    // let mut query_menu = sqlx::query(&sql_menu);
    // for id in &role_ids {
    //     query_menu = query_menu.bind(id);
    // }
    // query_menu.execute(&mut *tx).await?;
    info!(
        "[TX] Deleted menu associations from sys_role_menu for roles: {:?}",
        role_ids
    );

    tx.commit().await?;
    info!("[TX] Transaction committed for deleting roles.");
    Ok(result.rows_affected)
}

/// 修改角色状态
pub async fn change_role_status(db: &DatabaseConnection, vo: ChangeStatusVo) -> Result<u64, AppError> {
    let result = SysRole::update_many()
        .col_expr(SysRoleColumn::Status, vo.status.clone().into())
        .filter(SysRoleColumn::RoleId.eq(vo.role_id))
        .exec(db)
        .await?;

    // info!("[SERVICE] Entering change_role_status with vo: {:?}", vo);
    // let result = sqlx::query("UPDATE sys_role SET status = $1 WHERE role_id = $2")
    //     .bind(vo.status)
    //     .bind(vo.role_id)
    //     .execute(db.get_postgres_connection_pool())
    //     .await?;
    // info!(
    //     "[DB_RESULT] Changed status for role_id: {} to {}",
    //     vo.role_id, vo.status
    // );
    Ok(result.rows_affected)
}

/// 辅助函数：在事务中插入角色与菜单的关联记录
async fn insert_role_menu(tx: &DatabaseTransaction, role_id: i64, menu_ids: &[i64]) -> Result<(), AppError> {
    info!(
        "[TX_HELPER] Inserting {} menu associations for role_id: {}",
        menu_ids.len(),
        role_id
    );

    let models = menu_ids.into_iter().map(|menu_id| {
        SysRoleMenuModel {
            role_id: role_id,
            menu_id: *menu_id,
        }
        .into_active_model()
    });

    SysRoleMenu::insert_many(models).exec(tx).await?;

    // 构建批量插入的SQL
    // let mut sql = "INSERT INTO sys_role_menu (role_id, menu_id) VALUES ".to_string();
    // let mut values = Vec::new();
    // for menu_id in menu_ids {
    //     values.push(format!("({}, {})", role_id, menu_id));
    // }
    // sql.push_str(&values.join(", "));

    // sqlx::query(&sql).execute(&mut **tx).await?;
    info!("[TX_HELPER] Successfully inserted menu associations.");
    Ok(())
}

/// 根据角色键(role_key)列表查询对应的角色ID列表。
pub async fn select_role_ids_by_keys(db: &DatabaseConnection, role_keys: &[String]) -> Result<Vec<i64>, AppError> {
    if role_keys.is_empty() {
        // return Ok(vec![]);
        let msg = "role_keys 字段为空,无法获取角色";
        error!(msg);
        return Err(AppError::ValidationFailed(msg.to_string()));
    }

    let role_ids: Vec<i64> = SysRole::find()
        .filter(SysRoleColumn::RoleKey.is_in(role_keys))
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.role_id)
        .collect();

    // let mut query_builder = QueryBuilder::new("SELECT role_id FROM sys_role WHERE role_key IN (");

    // let mut separated = query_builder.separated(", ");
    // for key in role_keys {
    //     separated.push_bind(key);
    // }
    // separated.push_unseparated(")");

    // let query = query_builder.build_query_scalar();
    // let role_ids = query.fetch_all(db).await?;

    info!(
        "[SERVICE_ROLE] Found {} role IDs for keys: {:?}",
        role_ids.len(),
        role_keys
    );
    if role_ids.is_empty() {
        let msg = format!(
            "role_keys 字段内容为[{}],系统中没找到角色,请在管理菜单中创建角色，[权限字符]要与function内容一致",
            role_keys.join(",")
        );
        error!(msg);
        return Err(AppError::ValidationFailed(msg));
    }
    Ok(role_ids)
}

/// 查询所有状态正常的角色列表 (不分页)
///
/// 此函数专为用户管理、角色分配等需要全量角色列表的场景设计。
#[instrument(skip(db))]
pub async fn select_all_active_roles(db: &DatabaseConnection) -> Result<Vec<SysRoleModel>, AppError> {
    info!("[SERVICE] Entering select_all_active_roles");

    let roles: Vec<SysRoleModel> = SysRole::find()
        .filter(SysRoleColumn::Status.eq("0"))
        .filter(SysRoleColumn::DelFlag.eq("0"))
        .order_by_asc(SysRoleColumn::RoleSort)
        .all(db)
        .await?;

    // let roles: Vec<SysRole> = sqlx::query_as("SELECT * FROM sys_role WHERE status = '0' AND del_flag = '0' ORDER BY role_sort")
    //     .fetch_all(db)
    //     .await?;

    info!("[DB_RESULT] Found {} active roles.", roles.len());
    Ok(roles)
}

#[instrument(skip(db, params))]
pub async fn export_role_list(db: &DatabaseConnection, params: ListRoleQuery) -> Result<Vec<u8>, AppError> {
    info!(
        "[SERVICE] Starting role list export with params: {:?}",
        params
    );

    let db = db.get_postgres_connection_pool();

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_role WHERE del_flag = '0'");

    if let Some(name) = params.role_name {
        if !name.trim().is_empty() {
            query_builder
                .push(" AND role_name LIKE ")
                .push_bind(format!("%{}%", name));
        }
    }
    if let Some(key) = params.role_key {
        if !key.trim().is_empty() {
            query_builder
                .push(" AND role_key LIKE ")
                .push_bind(format!("%{}%", key));
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            query_builder.push(" AND status = ").push_bind(status);
        }
    }
    if let Some(begin_time) = params.begin_time {
        if !begin_time.trim().is_empty() {
            // 将用户输入作为参数绑定，而不是直接拼接到字符串中
            query_builder
                .push(" AND date_format(create_time,'%y%m%d') >= date_format(")
                .push_bind(begin_time)
                .push(",'%y%m%d')");
        }
    }
    if let Some(end_time) = params.end_time {
        if !end_time.trim().is_empty() {
            query_builder
                .push(" AND date_format(create_time,'%y%m%d') <= date_format(")
                .push_bind(end_time)
                .push(",'%y%m%d')");
        }
    }

    query_builder.push(" ORDER BY role_sort");

    let roles: Vec<SysRoleModel> = query_builder.build_query_as().fetch_all(db).await?;
    info!("[DB_RESULT] Fetched {} roles for export.", roles.len());

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = [
        "角色编号",
        "角色名称",
        "权限字符",
        "显示顺序",
        "状态",
        "创建时间",
    ];
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }

    for (row_num, role) in roles.iter().enumerate() {
        let row = (row_num + 1) as u32;
        worksheet.write(row, 0, role.role_id)?;
        worksheet.write(row, 1, &role.role_name)?;
        worksheet.write(row, 2, &role.role_key)?;
        worksheet.write(row, 3, role.role_sort)?;
        worksheet.write(
            row,
            4,
            if role.status == "0" {
                "正常"
            } else {
                "停用"
            },
        )?;
        worksheet.write(
            row,
            5,
            role.create_time.map_or("".to_string(), |t| {
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
