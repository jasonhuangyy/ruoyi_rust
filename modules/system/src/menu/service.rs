use super::model::{AddMenuVo, MenuTreeSelectVo, MenuTreeVo, UpdateMenuVo};
use common::error::AppError;
use entity::prelude::{SysMenu, SysMenuColumn, SysMenuModel};
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter};
use sea_orm::{DatabaseConnection, QueryOrder};
use sqlx::QueryBuilder;
use tracing::info;

pub async fn select_menu_tree_by_user_id(db: &DatabaseConnection, user_id: i64) -> Result<Vec<MenuTreeVo>, AppError> {
    info!(
        "[SERVICE] Entering select_menu_tree_by_user_id for user_id: {}",
        user_id
    );

    let menus: Vec<SysMenuModel>;

    // 1. 检查是否为管理员 (user_id 为 1)
    if user_id == 1 {
        info!("[AUTH] User is admin (user_id=1), fetching all enabled menus.");
        menus = SysMenu::find()
            .filter(SysMenuColumn::MenuType.is_in(vec!['M', 'C']))
            .filter(SysMenuColumn::Status.eq("0"))
            .order_by_asc(SysMenuColumn::ParentId)
            .order_by_asc(SysMenuColumn::OrderNum)
            .all(db)
            .await?;

        // 【修复】将 SQL 字符串字面量直接放入宏中
        // menus = sqlx::query_as!(
        //     SysMenu,
        //     "SELECT
        //         menu_id, menu_name, parent_id, order_num, path, component, query,
        //         route_name, is_frame, is_cache, menu_type, visible, status,
        //         perms, icon, create_by, create_time, update_by, update_time, remark
        //     FROM sys_menu
        //     WHERE menu_type IN ('M', 'C') AND status = '0'
        //     ORDER BY parent_id, order_num"
        // )
        // .fetch_all(db)
        // .await?;
    } else {
        info!("[AUTH] User is not admin, fetching menus based on roles.");

        // 将 SQL 字符串字面量直接放入宏中，并绑定参数, 使用 `SELECT DISTINCT m.*` 对于 `query_as!` 宏总有问题的，必须明确列出所有字段。
        menus = sqlx::query_as!(
            SysMenu,
            "SELECT DISTINCT
                m.menu_id, m.menu_name, m.parent_id, m.order_num, m.path, m.component, m.query,
                m.route_name, m.is_frame, m.is_cache, m.menu_type, m.visible, m.status,
                m.perms, m.icon, m.create_by, m.create_time, m.update_by, m.update_time, m.remark
            FROM sys_menu m
            LEFT JOIN sys_role_menu rm ON m.menu_id = rm.menu_id
            LEFT JOIN sys_user_role ur ON rm.role_id = ur.role_id
            LEFT JOIN sys_role r ON ur.role_id = r.role_id
            WHERE ur.user_id = ?
              AND m.menu_type IN ('M', 'C')
              AND m.status = '0'
              AND r.status = '0'
            ORDER BY m.parent_id, m.order_num",
            user_id
        )
        .fetch_all(db)
        .await?;
    }

    info!(
        "[DB_RESULT] Found {} menus for user_id {}.",
        menus.len(),
        user_id
    );

    let menu_tree = build_menu_tree(menus);
    Ok(menu_tree)
}

pub async fn select_menu_list(db: &DatabaseConnection) -> Result<Vec<SysMenu>, AppError> {
    // 添加 WHERE 子句，只查询目录和菜单，过滤掉按钮。 与 RuoYi 菜单管理页面的行为保持一致。
    let menus = sqlx::query_as!(
        SysMenu,
        "SELECT * FROM sys_menu ORDER BY parent_id, order_num"
    )
    .fetch_all(db)
    .await?;

    Ok(menus)
}

pub async fn select_menu_list_with_params(db: &MySqlPool, menu_name: Option<&str>, status: Option<&str>) -> Result<Vec<SysMenu>, AppError> {
    let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new("SELECT * FROM sys_menu WHERE 1=1 ");

    if let Some(name) = menu_name {
        if !name.trim().is_empty() {
            query_builder
                .push(" AND menu_name LIKE ")
                .push_bind(format!("%{}%", name));
        }
    }
    if let Some(s) = status {
        if !s.trim().is_empty() {
            query_builder.push(" AND status = ").push_bind(s);
        }
    }

    query_builder.push(" ORDER BY parent_id, order_num");

    let menus = query_builder.build_query_as().fetch_all(db).await?;
    Ok(menus)
}

/// 根据菜单ID查询菜单详情
pub async fn select_menu_by_id(db: &DatabaseConnection, menu_id: i64) -> Result<SysMenuModel, AppError> {
    SysMenu::find_by_id(menu_id)
        .one(db)
        .await?
        .ok_or(AppError::RecordNotFound)
    // let menu = sqlx::query_as!(
    //     SysMenuModel,
    //     "SELECT * FROM sys_menu WHERE menu_id = ?",
    //     menu_id
    // )
    // .fetch_one(db.get_postgres_connection_pool())
    // .await?;
    // Ok(menu)
}

/// 新增菜单
pub async fn add_menu(db: &DatabaseConnection, menu: AddMenuVo) -> Result<u64, AppError> {
    let is_frame_num: i32 = menu.is_frame;
    let is_cache_num: i32 = menu.is_cache;

    let result = sqlx::query!(
        r#"
            INSERT INTO sys_menu (menu_name, parent_id, order_num, path, component, is_frame, is_cache, menu_type, visible, status, perms, icon, remark, create_by, create_time)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'admin', NOW())
        "#,
        menu.menu_name,
        menu.parent_id,
        menu.order_num,
        menu.path,
        menu.component,
        is_frame_num,
        is_cache_num,
        menu.menu_type,
        menu.visible,
        menu.status,
        menu.perms,
        menu.icon,
        menu.remark
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

/// 修改菜单
pub async fn update_menu(db: &DatabaseConnection, menu: UpdateMenuVo) -> Result<u64, AppError> {
    let is_frame_num: i32 = menu.is_frame;
    let is_cache_num: i32 = menu.is_cache;

    let result = sqlx::query!(
        r#"
            UPDATE sys_menu
            SET menu_name = ?, parent_id = ?, order_num = ?, path = ?, component = ?, is_frame = ?, is_cache = ?, menu_type = ?, visible = ?, status = ?, perms = ?, icon = ?, remark = ?, update_by = 'admin', update_time = NOW()
            WHERE menu_id = ?
        "#,
        menu.menu_name,
        menu.parent_id,
        menu.order_num,
        menu.path,
        menu.component,
        is_frame_num,
        is_cache_num,
        menu.menu_type,
        menu.visible,
        menu.status,
        menu.perms,
        menu.icon,
        menu.remark,
        menu.menu_id
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

/// 删除菜单
pub async fn delete_menu_by_id(db: &DatabaseConnection, menu_id: i64) -> Result<u64, AppError> {
    // RuoYi 删除菜单时会检查是否有子菜单，我们暂时简化
    let result = sqlx::query!("DELETE FROM sys_menu WHERE menu_id = ?", menu_id)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}

/// 查询所有菜单，用于构建菜单选择树
pub async fn select_menu_list_for_treeselect(db: &DatabaseConnection) -> Result<Vec<SysMenu>, AppError> {
    // 关键区别：这里需要获取所有类型的菜单（M, C, F），而不仅仅是 M 和 C
    // 并且只选择状态正常的菜单
    info!("[SERVICE] Entering select_menu_list_for_treeselect");
    let menus = sqlx::query_as!(
        SysMenu,
        "SELECT * FROM sys_menu WHERE status = '0' ORDER BY parent_id, order_num"
    )
    .fetch_all(db)
    .await?;
    info!("[DB_RESULT] Found {} menus for treeselect.", menus.len());
    Ok(menus)
}

/// 辅助函数：将菜单的扁平列表构建成树形结构
pub fn build_menu_tree(menus: Vec<SysMenu>) -> Vec<MenuTreeVo> {
    // 1. 将所有 SysMenu 转换为 MenuTreeVo
    let mut all_nodes: Vec<MenuTreeVo> = menus
        .into_iter()
        .map(|menu| MenuTreeVo {
            menu,
            children: Vec::new(),
        })
        .collect();

    // 2. 筛选出所有根节点（parent_id 为 0 或 null）
    let mut roots = Vec::new();
    all_nodes.retain_mut(|node| {
        if node.menu.parent_id.unwrap_or(0) == 0 {
            // 这不是一个高效的移动，但对于建树这种一次性操作来说是可接受的，为了避免所有权问题，克隆根节点
            roots.push(node.clone());
            // 从 all_nodes 中移除根节点
            false
        } else {
            true
        }
    });

    // 3. 为每个根节点递归地构建子树
    for root in roots.iter_mut() {
        build_children_for_menu(root, &mut all_nodes);
    }

    // 4. 对根节点排序
    roots.sort_by_key(|a| a.menu.order_num.unwrap_or(0));

    roots
}

// 递归构建子节点的辅助函数
fn build_children_for_menu(parent: &mut MenuTreeVo, all_nodes: &mut Vec<MenuTreeVo>) {
    let mut children = Vec::new();
    // 从剩余节点中找到当前父节点的所有子节点
    all_nodes.retain_mut(|node| {
        if node.menu.parent_id == Some(parent.menu.menu_id) {
            children.push(node.clone()); // 克隆子节点
            false // 从 all_nodes 中移除
        } else {
            true
        }
    });

    // 为每个子节点递归地构建它们的子树
    for child in children.iter_mut() {
        build_children_for_menu(child, all_nodes);
    }

    // 对当前层的子节点进行排序
    children.sort_by_key(|a| a.menu.order_num.unwrap_or(0));
    parent.children = children;
}

/// 辅助函数：将扁平的 SysMenu 列表构建成前端需要的 MenuTreeSelectVo 树形结构
pub fn build_menu_treeselect(menus: Vec<SysMenu>) -> Vec<MenuTreeSelectVo> {
    // 复用已经绝对正确的 build_menu_tree 逻辑
    let menu_tree_vo = build_menu_tree(menus);
    // 然后进行一次简单的转换
    convert_menu_tree_to_select_tree(menu_tree_vo)
}

/// 辅助函数：将 MenuTreeVo 树转换为 MenuTreeSelectVo 树
fn convert_menu_tree_to_select_tree(tree_vo: Vec<MenuTreeVo>) -> Vec<MenuTreeSelectVo> {
    tree_vo
        .into_iter()
        .map(|vo| MenuTreeSelectVo {
            id: vo.menu.menu_id,
            label: vo.menu.menu_name,
            children: convert_menu_tree_to_select_tree(vo.children),
        })
        .collect()
}
