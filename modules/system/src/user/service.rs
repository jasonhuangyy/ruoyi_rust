use super::model::{AddUserVo, AuthRoleVo, ChangeStatusVo, ListUserQuery, ResetPwdVo, UpdateAuthRoleVo, UpdateProfileVo, UpdatePwdVo, UpdateUserVo, UserListVo, UserProfileVo};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use chrono::{Local, NaiveDateTime};
use common::error::AppError;
use common::page::TableDataInfo;
use entity::{prelude::*, sys_user};
use rust_xlsxwriter::Workbook;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, IntoActiveModel, QueryFilter, TransactionTrait};
use sqlx::{Postgres, QueryBuilder};
use tracing::{error, info, instrument};
use uuid::Uuid;

pub async fn select_user_list(db: &DatabaseConnection, params: ListUserQuery) -> Result<TableDataInfo<UserListVo>, AppError> {
    info!(
        "[SERVICE] Entering select_user_list with params: {:?}",
        params
    );

    let db = db.get_postgres_connection_pool();

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.*, d.dept_name
         FROM sys_user u
         LEFT JOIN sys_dept d ON u.dept_id = d.dept_id
         WHERE u.del_flag = '0'",
    );

    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT COUNT(*)
         FROM sys_user u
         LEFT JOIN sys_dept d ON u.dept_id = d.dept_id
         WHERE u.del_flag = '0'",
    );

    if let Some(name) = params.user_name {
        if !name.trim().is_empty() {
            let condition = format!("%{}%", name);
            query_builder
                .push(" AND u.user_name LIKE ")
                .push_bind(condition.clone());
            count_builder
                .push(" AND u.user_name LIKE ")
                .push_bind(condition);
        }
    }
    if let Some(phone) = params.phonenumber {
        if !phone.trim().is_empty() {
            let condition = format!("%{}%", phone);
            query_builder
                .push(" AND u.phonenumber LIKE ")
                .push_bind(condition.clone());
            count_builder
                .push(" AND u.phonenumber LIKE ")
                .push_bind(condition);
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            query_builder
                .push(" AND u.status = ")
                .push_bind(status.clone());
            count_builder.push(" AND u.status = ").push_bind(status);
        }
    }

    if let Some(dept_id) = params.dept_id {
        query_builder
            .push(" AND (u.dept_id = ")
            .push_bind(dept_id)
            .push(" OR find_in_set(")
            .push_bind(dept_id)
            .push(", d.ancestors))");
        count_builder
            .push(" AND (u.dept_id = ")
            .push_bind(dept_id)
            .push(" OR find_in_set(")
            .push_bind(dept_id)
            .push(", d.ancestors))");
    }

    let total: (i64,) = count_builder.build_query_as().fetch_one(db).await?;

    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page_num - 1) * page_size;
    query_builder
        .push(" ORDER BY u.create_time DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);

    info!("[DB_QUERY] Executing query for user list...");

    #[derive(sqlx::FromRow, Debug)]
    struct UserWithDeptName {
        #[sqlx(flatten)]
        user: SysUserModel,
        dept_name: Option<String>,
    }

    let results: Vec<UserWithDeptName> = query_builder.build_query_as().fetch_all(db).await?;

    let user_list_vo: Vec<UserListVo> = results
        .into_iter()
        .map(|r| {
            let dept = r.dept_name.map(|name| SysDeptModel {
                dept_id: r.user.dept_id.unwrap_or(0),
                dept_name: Some(name),
                ..Default::default()
            });
            UserListVo { user: r.user, dept }
        })
        .collect();

    info!(
        "[DB_RESULT] Found {} users for the current page.",
        user_list_vo.len()
    );
    Ok(TableDataInfo::new(user_list_vo, total.0))
}
/// 新增用户，并处理其与角色的关联关系（事务性）
pub async fn add_user(db: &DatabaseConnection, vo: AddUserVo) -> Result<i64, AppError> {
    info!("[SERVICE] Entering add_user with vo: {:?}", vo);
    let tx = db.begin().await?;

    let role_ids = vo.role_ids.clone();
    // 密码哈希处理
    let password = vo.password.clone().ok_or(AppError::InvalidCredentials)?; // 假设密码是必须的
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::PasswordHashError(e.to_string()))?
        .to_string();

    // 1. 插入用户基本信息
    let mut model: sys_user::ActiveModel = vo.into();
    model.password = Set(Some(password_hash));
    let model = model.insert(&tx).await?;

    // let result = sqlx::query!(
    //     "INSERT INTO sys_user (dept_id, user_name, nick_name, password, phonenumber, email, sex, status, remark, create_by, create_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'admin', NOW())",
    //     vo.dept_id,
    //     vo.user_name,
    //     vo.nick_name,
    //     password_hash,
    //     vo.phonenumber,
    //     vo.email,
    //     vo.sex,
    //     vo.status,
    //     vo.remark
    // )
    // .execute(&mut *tx)
    // .await?;
    let user_id = model.user_id;
    info!("[TX] Inserted into sys_user, new user_id: {}", user_id);

    // 2. 插入用户和角色的关联信息
    if let Some(role_ids) = role_ids {
        if !role_ids.is_empty() {
            insert_user_role(&tx, user_id, &role_ids).await?;
        }
    }

    tx.commit().await?;
    info!(
        "[TX] Transaction committed successfully for user_id: {}",
        user_id
    );
    Ok(user_id)
}

/// 修改用户，并处理其与角色的关联关系（事务性）
pub async fn update_user(db: &DatabaseConnection, vo: UpdateUserVo) -> Result<(), AppError> {
    info!("[SERVICE] Entering update_user with vo: {:?}", vo);
    let tx = db.begin().await?;
    let exist = SysUser::find_by_id(vo.user_id).one(db).await?;
    if exist.is_none() {
        return Err(AppError::RecordNotFound);
    }

    // 1. 更新用户基本信息
    let mut model = exist.unwrap().into_active_model();
    model.dept_id = Set(vo.dept_id);
    model.nick_name = Set(vo.nick_name);
    model.phonenumber = Set(vo.phonenumber);
    model.email = Set(vo.email);
    model.sex = Set(vo.sex);
    model.status = Set(vo.status);
    model.remark = Set(vo.remark);
    model.update_by = Set(Some("admin".to_string()));
    model.update_time = Set(Some(Local::now().naive_utc()));
    model.update(&tx).await?;

    // let result = sqlx::query!(
    //     "UPDATE sys_user SET dept_id=?, nick_name=?, phonenumber=?, email=?, sex=?, status=?, remark=?, update_by='admin', update_time=NOW() WHERE user_id=?",
    //     vo.dept_id,
    //     vo.nick_name,
    //     vo.phonenumber,
    //     vo.email,
    //     vo.sex,
    //     vo.status,
    //     vo.remark,
    //     vo.user_id
    // )
    // .execute(&mut *tx)
    // .await?;
    info!("[TX] Updated sys_user for user_id: {}.", vo.user_id);

    // 2. 删除旧的用户角色关联
    SysUserRole::delete_many()
        .filter(SysUserRoleColumn::UserId.eq(vo.user_id))
        .exec(&tx)
        .await?;
    // sqlx::query!("DELETE FROM sys_user_role WHERE user_id = ?", vo.user_id)
    //     .execute(&mut *tx)
    //     .await?;
    // info!(
    //     "[TX] Deleted old role associations for user_id: {}.",
    //     vo.user_id
    // );

    // 3. 插入新的用户角色关联
    if let Some(role_ids) = vo.role_ids {
        if !role_ids.is_empty() {
            insert_user_role(&tx, vo.user_id, &role_ids).await?;
        }
    }

    tx.commit().await?;
    info!(
        "[TX] Transaction committed successfully for user_id: {}.",
        vo.user_id
    );
    Ok(())
}

/// 根据ID查询用户信息
pub async fn select_user_by_id(db: &DatabaseConnection, user_id: i64) -> Result<SysUserModel, AppError> {
    info!(
        "[SERVICE] Entering select_user_by_id with user_id: {}",
        user_id
    );
    // sqlx::query_as!(
    //     SysUserModel,
    //     "SELECT * FROM sys_user WHERE user_id = ?",
    //     user_id
    // )
    // .fetch_one(db)
    // .await
    // .map_err(AppError::from)

    SysUser::find_by_id(user_id)
        .one(db)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::RecordNotFound)
}

/// 根据用户名查询用户信息 (用于登录)
///
/// # Returns
///
/// 返回 `Ok(Some(SysUser))` 如果找到用户，
/// 返回 `Ok(None)` 如果没有找到，
/// 返回 `Err(AppError)` 如果发生数据库错误。
pub async fn select_user_by_username(db: &DatabaseConnection, user_name: &str) -> Result<Option<SysUserModel>, AppError> {
    info!(
        "[SERVICE] Entering select_user_by_username with user_name: '{}'",
        user_name
    );
    // 使用 fetch_optional，因为它在找不到记录时会安全地返回 Ok(None)，而不是返回 Err
    // let user = sqlx::query_as!(
    //     SysUserModel,
    //     "SELECT * FROM sys_user WHERE user_name = ? AND del_flag = '0'", // 增加 del_flag 判断
    //     user_name
    // )
    // .fetch_optional(db)
    // .await?; // `?` 会将 sqlx::Error 转换为 AppError

    let user = SysUser::find()
        .filter(SysUserColumn::UserName.eq(user_name))
        .filter(SysUserColumn::DelFlag.eq("0"))
        .one(db)
        .await?;

    if user.is_some() {
        info!("[DB_RESULT] Found user for user_name: '{}'", user_name);
    } else {
        info!("[DB_RESULT] No user found for user_name: '{}'", user_name);
    }

    Ok(user)
}

/// 根据用户ID查询其关联的角色ID列表
pub async fn select_role_ids_by_user_id(db: &DatabaseConnection, user_id: i64) -> Result<Vec<i64>, AppError> {
    info!(
        "[SERVICE] Entering select_role_ids_by_user_id for user_id: {}",
        user_id
    );
    sqlx::query_scalar("SELECT role_id FROM sys_user_role WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(db.get_postgres_connection_pool())
        .await
        .map_err(AppError::from)
}

/// 修改用户状态
pub async fn change_user_status(db: &DatabaseConnection, vo: ChangeStatusVo) -> Result<u64, AppError> {
    info!("[SERVICE] Entering change_user_status with vo: {:?}", vo);
    let result = SysUser::update_many()
        .col_expr(SysUserColumn::Status, vo.status.into())
        .filter(SysUserColumn::UserId.eq(vo.user_id))
        .exec(db)
        .await?;
    // let result = sqlx::query!(
    //     "UPDATE sys_user SET status = ? WHERE user_id = ?",
    //     vo.status,
    //     vo.user_id
    // )
    // .execute(db)
    // .await?;
    Ok(result.rows_affected)
}

/// 重置用户密码
pub async fn reset_user_pwd(db: &DatabaseConnection, vo: ResetPwdVo) -> Result<u64, AppError> {
    info!(
        "[SERVICE] Entering reset_user_pwd for user_id: {}",
        vo.user_id
    );
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(vo.password.as_bytes(), &salt)
        .map_err(|e| AppError::PasswordHashError(e.to_string()))?
        .to_string();
    // let result = sqlx::query!(
    //     "UPDATE sys_user SET password = ? WHERE user_id = ?",
    //     password_hash,
    //     vo.user_id
    // )
    // .execute(db)
    // .await?;
    let result = SysUser::update_many()
        .col_expr(SysUserColumn::Password, password_hash.into())
        .filter(SysUserColumn::UserId.eq(vo.user_id))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

/// 批量删除用户
pub async fn delete_user_by_ids(db: &DatabaseConnection, user_ids: &[i64]) -> Result<u64, AppError> {
    info!(
        "[SERVICE] Entering delete_user_by_ids with ids: {:?}",
        user_ids
    );

    let user_ids = user_ids.to_vec();
    let tx = db.begin().await?;

    let result = SysUser::update_many()
        .col_expr(SysUserColumn::DelFlag, "2".into())
        .filter(SysUserColumn::UserId.is_in(user_ids.clone()))
        .exec(&tx)
        .await?;

    // let params = user_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    // let sql_user = format!(
    //     "UPDATE sys_user SET del_flag = '2' WHERE user_id IN ({})",
    //     params
    // );
    // let mut query_user = sqlx::query(&sql_user);
    // for id in user_ids {
    //     query_user = query_user.bind(id);
    // }
    // let result = query_user.execute(&tx).await?;

    SysUserRole::delete_many()
        .filter(SysUserRoleColumn::UserId.is_in(user_ids))
        .exec(&tx)
        .await?;
    // let sql_role = format!("DELETE FROM sys_user_role WHERE user_id IN ({})", params);
    // let mut query_role = sqlx::query(&sql_role);
    // for id in user_ids {
    //     query_role = query_role.bind(id);
    // }
    // query_role.execute(&tx).await?;

    tx.commit().await?;
    Ok(result.rows_affected)
}

async fn insert_user_role(tx: &DatabaseTransaction, user_id: i64, role_ids: &[i64]) -> Result<(), AppError> {
    info!(
        "[TX_HELPER] Inserting {} role associations for user_id: {}",
        role_ids.len(),
        user_id
    );

    let user_role_models = role_ids
        .iter()
        .map(|role_id| {
            SysUserRoleModel {
                user_id,
                role_id: *role_id,
            }
            .into_active_model()
        })
        .collect::<Vec<_>>();

    SysUserRole::insert_many(user_role_models).exec(tx).await?;

    // let mut sql = "INSERT INTO sys_user_role (user_id, role_id) VALUES ".to_string();
    // sql.push_str(
    //     &role_ids
    //         .iter()
    //         .map(|role_id| format!("({}, {})", user_id, role_id))
    //         .collect::<Vec<_>>()
    //         .join(", "),
    // );
    // sqlx::query(&sql).execute(tx).await?;
    info!("[TX_HELPER] Successfully inserted role associations.");
    Ok(())
}

/// 根据用户ID查询其拥有的角色键（role_key）列表
///
/// # Arguments
/// * `db` - 数据库连接池
/// * `user_id` - 用户ID
///
/// # Returns
/// 一个包含角色键字符串的向量 `Vec<String>`
pub async fn get_user_roles(db: &DatabaseConnection, user_id: i64) -> Result<Vec<String>, AppError> {
    info!("[SERVICE] Entering get_user_roles for user_id: {}", user_id);
    let db = db.get_postgres_connection_pool();

    // RuoYi 的逻辑：如果是管理员(user_id=1)，直接返回 "admin"
    if user_id == 1 {
        info!("[SERVICE] User is admin (user_id=1), returning hardcoded role 'admin'.");
        return Ok(vec!["admin".to_string()]);
    }

    // 普通用户则从数据库查询
    let sql = r#"
        SELECT r.role_key
        FROM sys_role r
        LEFT JOIN sys_user_role ur ON r.role_id = ur.role_id
        WHERE r.status = '0' AND r.del_flag = '0' AND ur.user_id = ?
    "#;

    info!(
        "[DB_QUERY] Executing query for user roles with user_id: {}",
        user_id
    );
    // query_scalar 可以将查询结果的第一列直接收集到一个 Vec 中
    let roles: Vec<String> = sqlx::query_scalar(sql).bind(user_id).fetch_all(db).await?;

    info!(
        "[DB_RESULT] Found {} roles for user_id {}: {:?}",
        roles.len(),
        user_id,
        roles
    );
    Ok(roles)
}

/// 根据用户ID查询其拥有的所有菜单权限标识（perms）
///
/// # Arguments
/// * `db` - 数据库连接池
/// * `user_id` - 用户ID
///
/// # Returns
/// 一个包含权限标识字符串的向量 `Vec<String>`
pub async fn get_user_permissions(db: &DatabaseConnection, user_id: i64) -> Result<Vec<String>, AppError> {
    info!(
        "[SERVICE] Entering get_user_permissions for user_id: {}",
        user_id
    );
    info!("[SERVICE_PERM_DEBUG] 准备查询用户 {} 的权限...", user_id); // ★ 调试日志1
    let perms: Vec<String>;
    let db = db.get_postgres_connection_pool();

    // RuoYi 的逻辑：如果是管理员(user_id=1)，拥有所有权限
    if user_id == 1 {
        info!("[SERVICE_PERM_DEBUG] 用户是管理员 (user_id=1)，授予所有权限。"); // ★ 调试日志2
        info!("[SERVICE] User is admin (user_id=1), granting all permissions.");
        let sql = "SELECT DISTINCT perms FROM sys_menu WHERE perms IS NOT NULL AND perms != ''";
        perms = sqlx::query_scalar(sql).fetch_all(db).await?;
    } else {
        // 普通用户则通过角色关联查询
        let sql = r#"
            SELECT DISTINCT m.perms
            FROM sys_menu m
            LEFT JOIN sys_role_menu rm ON m.menu_id = rm.menu_id
            LEFT JOIN sys_user_role ur ON rm.role_id = ur.role_id
            LEFT JOIN sys_role r ON ur.role_id = r.role_id
            WHERE m.status = '0' AND r.status = '0'
              AND m.perms IS NOT NULL AND m.perms != ''
              AND ur.user_id = $1
        "#;
        info!(
            "[DB_QUERY] Executing query for user permissions with user_id: {}",
            user_id
        );
        perms = sqlx::query_scalar(sql).bind(user_id).fetch_all(db).await?;
    }
    info!(
        "[SERVICE_PERM_DEBUG] 为用户 {} 查询到的最终权限列表: {:?}",
        user_id, perms
    );
    info!(
        "[DB_RESULT] Found {} permissions for user_id {}: {:?}",
        perms.len(),
        user_id,
        perms
    );
    Ok(perms)
}

/// 根据用户ID查询其关联的岗位ID列表
pub async fn select_post_ids_by_user_id(db: &DatabaseConnection, user_id: i64) -> Result<Vec<i64>, AppError> {
    info!(
        "[SERVICE] Entering select_post_ids_by_user_id for user_id: {}",
        user_id
    );

    let db = db.get_postgres_connection_pool();
    sqlx::query_scalar("SELECT post_id FROM sys_user_post WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(db)
        .await
        .map_err(AppError::from)
}

/// 根据用户ID获取角色分配信息 (用于 authRole 页面)
#[instrument(skip(db))]
pub async fn get_auth_role(db: &DatabaseConnection, user_id: i64) -> Result<AuthRoleVo, AppError> {
    info!("[SERVICE] Entering get_auth_role for user_id: {}", user_id);
    let user = select_user_by_id(db, user_id).await?;

    let all_roles = SysRole::find()
        .filter(SysRoleColumn::Status.eq("0"))
        .filter(SysRoleColumn::DelFlag.eq("0"))
        .all(db)
        .await?;

    // let db = db.get_postgres_connection_pool();
    // let all_roles: Vec<SysRoleModel> = sqlx::query_as("SELECT * FROM sys_role WHERE status = '0' AND del_flag = '0'")
    //     .fetch_all(db)
    //     .await?;

    Ok(AuthRoleVo {
        user,
        roles: all_roles,
    })
}

/// 更新用户的角色分配 (事务性)
#[instrument(skip(db, vo))]
pub async fn update_auth_role(db: &DatabaseConnection, vo: UpdateAuthRoleVo) -> Result<(), AppError> {
    info!(
        "[SERVICE] Entering update_auth_role for user_id: {}",
        vo.user_id
    );
    let tx = db.begin().await?;

    // 1. 删除该用户所有的旧角色关联
    SysUserRole::delete_many()
        .filter(SysUserRoleColumn::UserId.eq(vo.user_id))
        .exec(&tx)
        .await?;
    // sqlx::query("DELETE FROM sys_user_role WHERE user_id = ?")
    //     .bind(vo.user_id)
    //     .execute(&tx)
    //     .await?;
    // info!("[TX] Deleted old roles for user_id: {}", vo.user_id);

    // 2. 解析前端传来的 role_ids 字符串
    let role_ids: Vec<i64> = vo
        .role_ids
        .split(',')
        .filter_map(|s| s.parse::<i64>().ok())
        .collect();

    // 3. 如果有新的角色ID，则批量插入
    if !role_ids.is_empty() {
        // 复用已有的插入辅助函数
        insert_user_role(&tx, vo.user_id, &role_ids).await?;
    }

    tx.commit().await?;
    info!(
        "[TX] Committed new role associations for user_id: {}",
        vo.user_id
    );
    Ok(())
}

/// 获取用户个人信息
pub async fn get_user_profile(db: &DatabaseConnection, user_id: i64) -> Result<UserProfileVo, AppError> {
    info!(
        "[SERVICE] Entering get_user_profile for user_id: {}",
        user_id
    );
    let user = select_user_by_id(db, user_id).await?;

    let db = db.get_postgres_connection_pool();
    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT r.role_name FROM sys_role r
         LEFT JOIN sys_user_role ur ON r.role_id = ur.role_id
         WHERE ur.user_id = ?",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;
    // 将角色名称用逗号连接成一个字符串
    let role_group = roles.join(",");

    let posts: Vec<String> = sqlx::query_scalar(
        "SELECT p.post_name FROM sys_post p
         LEFT JOIN sys_user_post up ON p.post_id = up.post_id
         WHERE up.user_id = ?",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;
    let post_group = posts.join(",");

    info!(
        "[DB_RESULT] Profile for user_id {}: roles='{}', posts='{}'",
        user_id, role_group, post_group
    );

    Ok(UserProfileVo {
        data: user,
        role_group,
        post_group,
    })
}

/// 更新当前登录用户的基本资料
#[instrument(skip(db, vo))]
pub async fn update_user_profile(db: &DatabaseConnection, user_id: i64, vo: UpdateProfileVo) -> Result<u64, AppError> {
    info!(
        "[SERVICE] Entering update_user_profile for user_id: {}",
        user_id
    );

    let result = SysUser::update_many()
        .filter(SysUserColumn::UserId.eq(user_id))
        .col_expr(SysUserColumn::NickName, vo.nick_name.into())
        .col_expr(SysUserColumn::Phonenumber, vo.phonenumber.into())
        .col_expr(SysUserColumn::Email, vo.email.into())
        .col_expr(SysUserColumn::Sex, vo.sex.into())
        .col_expr(SysUserColumn::UpdateTime, Local::now().naive_local().into())
        .exec(db)
        .await?;
    // let result = sqlx::query!(
    //     "UPDATE sys_user SET nick_name = $1, phonenumber = $2, email = $3, sex = $4, update_time = NOW() WHERE user_id = $5",
    //     vo.nick_name,
    //     vo.phonenumber,
    //     vo.email,
    //     vo.sex,
    //     user_id
    // )
    // .execute(db)
    // .await?;
    Ok(result.rows_affected)
}

/// 用户修改个人密码
pub async fn update_user_pwd(db: &DatabaseConnection, user_id: i64, vo: UpdatePwdVo) -> Result<u64, AppError> {
    info!(
        "[SERVICE] Entering update_user_pwd for user_id: {}",
        user_id
    );

    let current_user = select_user_by_id(db, user_id).await?;
    let password_from_db = current_user.password.ok_or_else(|| {
        error!("[PWD_UPDATE] User {} has no password set in DB.", user_id);
        AppError::ValidationFailed("用户密码未设置，无法修改".to_string())
    })?;

    let parsed_hash = PasswordHash::new(&password_from_db).map_err(|e| AppError::PasswordHashError(e.to_string()))?;

    if Argon2::default()
        .verify_password(vo.old_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        error!(
            "[PWD_UPDATE] Old password verification failed for user_id: {}",
            user_id
        );
        return Err(AppError::ValidationFailed("旧密码不正确".to_string()));
    }
    let salt = SaltString::generate(&mut OsRng);
    let new_password_hash = Argon2::default()
        .hash_password(vo.new_password.as_bytes(), &salt)
        .map_err(|e| AppError::PasswordHashError(e.to_string()))?
        .to_string();

    let result = SysUser::update_many()
        .filter(SysUserColumn::UserId.eq(user_id))
        .col_expr(SysUserColumn::Password, new_password_hash.into())
        .col_expr(
            SysUserColumn::PwdUpdateDate,
            Local::now().naive_local().into(),
        )
        .exec(db)
        .await?;
    // let result = sqlx::query!(
    //     "UPDATE sys_user SET password = ?, pwd_update_date = NOW() WHERE user_id = ?",
    //     new_password_hash,
    //     user_id
    // )
    // .execute(db)
    // .await?;

    info!(
        "[SERVICE] Successfully updated password for user_id: {}",
        user_id
    );
    Ok(result.rows_affected)
}

/// 更新当前登录用户的头像
#[instrument(skip(db, file_data))]
pub async fn update_user_avatar(db: &DatabaseConnection, user_id: i64, file_data: &[u8]) -> Result<String, AppError> {
    info!(
        "[SERVICE] Entering update_user_avatar for user_id: {}",
        user_id
    );

    // 逻辑类似于 file::service::save_file，但目录和数据库操作不同
    let avatar_dir = "uploads/avatar";
    tokio::fs::create_dir_all(avatar_dir).await.map_err(|e| {
        error!("创建头像目录失败: {:?}", e);
        AppError::ValidationFailed("服务器创建目录失败".to_string())
    })?;

    let file_name = format!("{}.png", Uuid::new_v4());
    let file_path = std::path::Path::new(avatar_dir).join(&file_name);
    tokio::fs::write(&file_path, file_data).await.map_err(|e| {
        error!("写入头像文件失败: {:?}", e);
        AppError::ValidationFailed("服务器保存文件失败".to_string())
    })?;

    let avatar_url = format!("/uploads/avatar/{}", file_name);

    SysUser::update_many()
        .filter(SysUserColumn::UserId.eq(user_id))
        .col_expr(SysUserColumn::Avatar, avatar_url.clone().into())
        .exec(db)
        .await?;
    // sqlx::query("UPDATE sys_user SET avatar = ? WHERE user_id = ?")
    //     .bind(&avatar_url)
    //     .bind(user_id)
    //     .execute(db.get_postgres_connection_pool())
    //     .await?;

    info!(
        "[SERVICE] User avatar updated for user_id: {}. New URL: {}",
        user_id, avatar_url
    );
    Ok(avatar_url)
}

#[instrument(skip(db, params))]
pub async fn export_user_list(db: &DatabaseConnection, params: ListUserQuery) -> Result<Vec<u8>, AppError> {
    info!(
        "[SERVICE] Starting user list export with params: {:?}",
        params
    );
    let db = db.get_postgres_connection_pool();
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.user_id, u.user_name, u.nick_name, u.email, u.phonenumber, u.sex, u.status, u.login_ip, u.login_date, u.create_time, d.dept_name
         FROM sys_user u
         LEFT JOIN sys_dept d ON u.dept_id = d.dept_id
         WHERE u.del_flag = '0'",
    );

    if let Some(name) = params.user_name {
        if !name.trim().is_empty() {
            query_builder
                .push(" AND u.user_name LIKE ")
                .push_bind(format!("%{}%", name));
        }
    }
    if let Some(phone) = params.phonenumber {
        if !phone.trim().is_empty() {
            query_builder
                .push(" AND u.phonenumber LIKE ")
                .push_bind(format!("%{}%", phone));
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            query_builder.push(" AND u.status = ").push_bind(status);
        }
    }
    if let Some(dept_id) = params.dept_id {
        query_builder
            .push(" AND (u.dept_id = ")
            .push_bind(dept_id)
            .push(" OR FIND_IN_SET(")
            .push_bind(dept_id)
            .push(", d.ancestors))");
    }

    query_builder.push(" ORDER BY u.create_time DESC");

    #[derive(sqlx::FromRow, Debug)]
    struct UserExportRow {
        user_id: i64,
        user_name: String,
        nick_name: String,
        email: Option<String>,
        phonenumber: Option<String>,
        sex: Option<String>,
        status: Option<String>,
        login_ip: Option<String>,
        login_date: Option<NaiveDateTime>,
        create_time: Option<NaiveDateTime>,
        dept_name: Option<String>,
    }

    let users: Vec<UserExportRow> = query_builder.build_query_as().fetch_all(db).await?;
    info!("[DB_RESULT] Fetched {} users for export.", users.len());

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = [
        "用户编号",
        "用户名称",
        "用户昵称",
        "部门",
        "手机号码",
        "邮箱",
        "性别",
        "帐号状态",
        "最后登录IP",
        "最后登录时间",
        "创建时间",
    ];
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }

    for (row_num, user) in users.iter().enumerate() {
        let row = (row_num + 1) as u32;
        worksheet.write(row, 0, user.user_id)?;
        worksheet.write(row, 1, &user.user_name)?;
        worksheet.write(row, 2, &user.nick_name)?;
        worksheet.write(row, 3, user.dept_name.as_deref().unwrap_or(""))?;
        worksheet.write(row, 4, user.phonenumber.as_deref().unwrap_or(""))?;
        worksheet.write(row, 5, user.email.as_deref().unwrap_or(""))?;
        worksheet.write(
            row,
            6,
            match user.sex.as_deref() {
                Some("0") => "男",
                Some("1") => "女",
                _ => "未知",
            },
        )?;
        worksheet.write(
            row,
            7,
            if user.status.as_deref() == Some("0") {
                "正常"
            } else {
                "停用"
            },
        )?;
        worksheet.write(row, 8, user.login_ip.as_deref().unwrap_or(""))?;
        worksheet.write(
            row,
            9,
            user.login_date.map_or("".to_string(), |t| {
                t.format("%Y-%m-%d %H:%M:%S").to_string()
            }),
        )?;
        worksheet.write(
            row,
            10,
            user.create_time.map_or("".to_string(), |t| {
                t.format("%Y-%m-%d %H:%M:%S").to_string()
            }),
        )?;
    }

    let buffer = workbook.save_to_buffer()?;
    info!(
        "[SERVICE] User export Excel buffer created successfully, size: {} bytes.",
        buffer.len()
    );

    Ok(buffer)
}
