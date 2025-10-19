use crate::error::AppError;
use sea_orm::DatabaseConnection;
use strum::{AsRefStr, EnumString};
use tracing::info;
/// # 系统权限枚举
///
/// 定义了系统中所有可用的API权限点。
/// - `#[derive(AsRefStr)]`: 这个宏会自动为枚举实现 `AsRef<str>` trait，可以通过 `.as_ref()` 方法获取其字符串表示。
/// - `#[strum(serialize = "...")]`: 这个属性将每个枚举成员与其在数据库中对应的

#[derive(Debug, Clone, Copy, AsRefStr, EnumString)]
pub enum Permission {
    // 用户管理
    #[strum(serialize = "system:user:list")]
    UserList,
    #[strum(serialize = "system:user:query")]
    UserQuery,
    #[strum(serialize = "system:user:add")]
    UserAdd,
    #[strum(serialize = "system:user:edit")]
    UserEdit,
    #[strum(serialize = "system:user:remove")]
    UserRemove,
    #[strum(serialize = "system:user:resetPwd")]
    UserResetPwd,
    #[strum(serialize = "system:user:export")]
    UserExport,

    // 角色管理
    #[strum(serialize = "system:role:list")]
    RoleList,
    #[strum(serialize = "system:role:query")]
    RoleQuery,
    #[strum(serialize = "system:role:add")]
    RoleAdd,
    #[strum(serialize = "system:role:edit")]
    RoleEdit,
    #[strum(serialize = "system:role:remove")]
    RoleRemove,
    #[strum(serialize = "system:role:export")]
    RoleExport,

    // 菜单管理
    #[strum(serialize = "system:menu:list")]
    MenuList,
    #[strum(serialize = "system:menu:query")]
    MenuQuery,
    #[strum(serialize = "system:menu:add")]
    MenuAdd,
    #[strum(serialize = "system:menu:edit")]
    MenuEdit,
    #[strum(serialize = "system:menu:remove")]
    MenuRemove,

    // ... 在这里添加所有其他的权限 ...
    // 字典管理
    #[strum(serialize = "system:dict:list")]
    DictList,
    #[strum(serialize = "system:dict:export")]
    DictExport,

    // 参数配置
    #[strum(serialize = "system:config:list")]
    ConfigList,
    #[strum(serialize = "system:config:query")]
    ConfigQuery,
    #[strum(serialize = "system:config:add")]
    ConfigAdd,
    #[strum(serialize = "system:config:edit")]
    ConfigEdit,
    #[strum(serialize = "system:config:remove")]
    ConfigRemove,
    #[strum(serialize = "system:config:export")]
    ConfigExport,
    // --- 岗位管理 ---
    #[strum(serialize = "system:post:list")]
    PostList,
    #[strum(serialize = "system:post:query")]
    PostQuery,
    #[strum(serialize = "system:post:add")]
    PostAdd,
    #[strum(serialize = "system:post:edit")]
    PostEdit,
    #[strum(serialize = "system:post:remove")]
    PostRemove,
    // --- 操作日志 ---
    #[strum(serialize = "monitor:operlog:export")]
    OperlogExport,
    // --- 登录日志 ---
    #[strum(serialize = "monitor:logininfor:export")]
    LogininforExport,

    // --- 定時任務 ---
    #[strum(serialize = "monitor:job:remove")]
    JobRemove,
    #[strum(serialize = "monitor:job:export")]
    JobExport,
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
              AND ur.user_id = ?
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
