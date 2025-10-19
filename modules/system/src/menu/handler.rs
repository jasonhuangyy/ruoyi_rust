use super::{
    model::MenuTreeVo, // 从 system/model.rs 引入 RouterVo, MetaVo
    service,
};
use crate::menu::model::{AddMenuVo, MenuTreeSelectVo, UpdateMenuVo};
use crate::model::{MetaVo, RouterVo};
use crate::role;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{
    extract::{Extension, State},
    Json,
};
use common::{error::AppError, response::AjaxResult};
use entity::prelude::SysMenuModel;
use framework::{jwt::ClaimsData, state::AppState};
use serde_json::json;
use std::sync::Arc;
use tracing::info;

pub async fn get_routers(
    State(state): State<Arc<AppState>>,
    // 从 auth 中间件获取当前登录用户的信息
    Extension(claims): Extension<ClaimsData>,
) -> Result<Json<AjaxResult<Vec<RouterVo>>>, AppError> {
    // 调用服务层逻辑，获取菜单树
    let menu_tree = service::select_menu_tree_by_user_id(&state.db, claims.user_id).await?;

    // 将 MenuTreeVo 转换为前端需要的 RouterVo 结构
    let router_vos = build_routers(menu_tree);

    Ok(Json(AjaxResult::success(router_vos)))
}

/// 获取菜单列表
pub async fn list(
    State(state): State<Arc<AppState>>,
    // Query(params): Query<MenuListParams>, // 后续可以添加查询参数
) -> Result<Json<AjaxResult<Vec<MenuTreeVo>>>, AppError> {
    // 菜单管理界面通常需要树形结构来展示
    let menu_list = service::select_menu_list(&state.db).await?;
    // 调用我们已有的 build_menu_tree 函数来构建树
    let menu_tree = service::build_menu_tree(menu_list);
    Ok(Json(AjaxResult::success(menu_tree)))
}

/// 获取菜单详细信息
pub async fn get_detail(State(state): State<Arc<AppState>>, Path(menu_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    // 1. 返回值类型改为更通用的 impl IntoResponse
    let menu = service::select_menu_by_id(&state.db, menu_id).await?;

    // 2. 手动构建前端需要的、带有 "data" 键的 JSON 结构
    let response = json!({
        "code": 200,
        "msg": "操作成功",
        "data": menu // 将查询到的 menu 对象作为 "data" 字段的值
    });

    // 3. 将构建好的 JSON 返回
    Ok(Json(response))
}

/// 新增菜单
pub async fn add(State(state): State<Arc<AppState>>, Json(body): Json<AddMenuVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::add_menu(&state.db, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("新增成功")))
}

/// 修改菜单
pub async fn update(State(state): State<Arc<AppState>>, Json(body): Json<UpdateMenuVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::update_menu(&state.db, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 删除菜单
pub async fn delete(State(state): State<Arc<AppState>>, Path(menu_id): Path<i64>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::delete_menu_by_id(&state.db, menu_id).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 辅助函数：将后端的 MenuTreeVo 转换为前端路由所需的 RouterVo
fn build_routers(menu_trees: Vec<MenuTreeVo>) -> Vec<RouterVo> {
    menu_trees
        .into_iter()
        .map(|menu_tree| {
            let menu = &menu_tree.menu;

            // 初始构建 router 对象
            let mut router = RouterVo {
                hidden: menu.visible.as_deref().unwrap_or("1") == "1",
                name: get_route_name(menu),
                path: get_route_path(menu),
                component: get_component(menu), // 初始 component 从数据库获取
                redirect: None,
                always_show: None,
                meta: MetaVo {
                    title: menu.menu_name.clone(),
                    icon: menu.icon.clone().unwrap_or_default(),
                    no_cache: menu.is_cache.unwrap_or_default(),
                    link: if menu.is_frame.unwrap_or(false) {
                        menu.path.clone()
                    } else {
                        None
                    },
                },
                children: None,
            };

            let has_children = !menu_tree.children.is_empty();

            if has_children {
                // 1. 如果有子菜单，递归构建
                router.children = Some(build_routers(menu_tree.children));
                // 确保父级 component 是 Layout
                if menu.parent_id.unwrap_or(0) == 0 {
                    router.component = "Layout".to_string();
                }
            }

            // 2.RuoYi 逻辑：处理一级菜单 (类型C)
            // 如果一个菜单是顶级的(parent_id=0)，类型是C，并且它没有子菜单，
            // 它需要被包装在一个Layout中，并拥有一个path为'index'的子路由。
            if menu.parent_id.unwrap_or(0) == 0 && menu.menu_type.as_deref() == Some("C") && !has_children {
                // a. 父路由（外壳）的 component 必须是 Layout
                router.component = "Layout".to_string();

                // b. 创建一个代表实际页面的子路由
                let child_router = RouterVo {
                    // 1: 子路由的 path 硬编码为 "index"
                    path: "index".to_string(),
                    // 2: 子路由的 name 根据新 path "index" 生成，得到 "Index"
                    name: get_route_name(&SysMenuModel {
                        path: Some("index".to_string()),
                        ..Default::default()
                    }),
                    component: get_component(menu), // 真实组件路径
                    meta: router.meta.clone(),      // meta 信息继承自逻辑父级
                    // 其他字段为默认值
                    hidden: false,
                    redirect: None,
                    always_show: None,
                    children: None,
                };

                // c. 3: 父路由的 redirect 指向子路由的完整路径
                router.redirect = Some(format!("{}/index", router.path));
                router.children = Some(vec![child_router]);
            }
            // 3. 处理目录 (类型M)
            else if menu.menu_type.as_deref() == Some("M") {
                router.redirect = Some("noRedirect".to_string());
                router.always_show = Some(true);
                // 顶级目录的 component 也应该是 Layout
                if menu.parent_id.unwrap_or(0) == 0 {
                    router.component = "Layout".to_string();
                }
            }

            router
        })
        .collect()
}

/// 获取路由名称
fn get_route_name(menu: &SysMenuModel) -> String {
    // 动态路由的 path 通常是 ':id' 格式，需要转换成 NameId
    let mut path = menu.path.clone().unwrap_or_default();
    if path.starts_with(':') {
        path = path.replacen(':', "", 1);
    }
    // 首字母大写
    let mut c = path.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// 获取路由地址
fn get_route_path(menu: &SysMenuModel) -> String {
    let mut path = menu.path.clone().unwrap_or_default();
    // 如果是外链
    if menu.is_frame.unwrap_or(false) == false {
        return path;
    }
    // 如果是顶级目录
    if menu.parent_id.unwrap_or(0) == 0 && menu.menu_type.as_deref() == Some("M") {
        // 在前面补上 '/'
        if !path.starts_with('/') {
            path = format!("/{}", path);
        }
    }
    // 2. 如果是顶级菜单 (parent_id=0)，它的路径必须以 "/" 开头
    if menu.parent_id.unwrap_or(0) == 0 {
        if !path.starts_with('/') {
            path = format!("/{}", path);
        }
    }
    path
}

/// 获取组件信息
fn get_component(menu: &SysMenuModel) -> String {
    // let mut component = "Layout".to_string(); // 默认为 Layout
    // if let Some(comp_str) = &menu.component {
    //     if !comp_str.is_empty() {
    //         component = comp_str.clone();
    //     } else if menu.parent_id.unwrap_or(0) == 0 &&  menu.menu_type.as_deref() == Some("M") {
    //         // 如果是顶级目录且 component 为空，则为 ParentView
    //         component = "ParentView".to_string();
    //     }
    // }
    ////////////
    // 默认返回值现在是空字符串，而不是 "Layout"
    let mut component = String::new();

    if let Some(comp_str) = &menu.component {
        if !comp_str.is_empty() {
            return comp_str.clone();
        }
    }

    // 如果 component 字段为空，根据规则赋予默认值
    if menu.parent_id.unwrap_or(0) == 0 && menu.menu_type.as_deref() == Some("M") {
        // 顶级目录，使用 ParentView
        component = "ParentView".to_string();
    } else {
        // 对于其他情况（如顶级菜单C，或子菜单C），component 字段为空时，就让它为空
        // 这样 build_routers 函数就能知道需要特殊处理
    }
    component
}

/// 获取菜单下拉树列表
/// 这个接口被用于“角色管理-新增/修改”和“菜单管理-新增/修改”中的“上级菜单”选择器
pub async fn treeselect(State(state): State<Arc<AppState>>) -> Result<Json<AjaxResult<Vec<MenuTreeSelectVo>>>, AppError> {
    info!("[HANDLER] Entering menu::treeselect");
    // 1. 从数据库查询所有状态正常的菜单
    let menus = service::select_menu_list_for_treeselect(&state.db).await?;
    // 2. 将扁平列表构建成树形结构
    let menu_tree = service::build_menu_treeselect(menus);
    // 3. 返回成功响应
    Ok(Json(AjaxResult::success(menu_tree)))
}

/// 根据角色ID查询菜单下拉树列表
///
/// # 架构设计说明
///
/// 这个接口是为适配现有 RuoYi-Vue 前端而创建的。
/// 在理想的架构中，`menu` 模块应该只提供纯粹的菜单数据，不应关心“角色”这个概念。
/// 获取“某个角色对应的菜单树和已选中的键”这个业务逻辑，应该由 `role` 模块的处理器来完成，
/// 它在内部调用 `menu` 模块的服务来获取数据，然后进行组装。
///
/// 然而，现有前端在点击“修改角色”时，会分别调用两个接口：
/// 1. `GET /system/role/{roleId}`: 获取角色基本信息。
/// 2. `GET /system/menu/roleMenuTreeselect/{roleId}`: 获取菜单树和已选中的键。
///
/// 为了在不修改前端代码的前提下让功能正常工作，我们在此处创建了这个专门的处理器。
/// 它通过跨模块调用 `role::service` 来获取 `checkedKeys`，这是一种技术上的妥协。
///
/// 未来重构方向:如果有机会优化前端，应将上述两个请求合并为一个，
/// 由 `role` 模块的某个接口（如增强版的 `get_detail`）一次性返回所有需要的数据。
/// 届时，本接口 (`role_menu_treeselect`) 即可被移除。
///
pub async fn role_menu_treeselect(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<i64>, // 从路径中获取 role_id
) -> Result<impl IntoResponse, AppError> {
    info!(
        "[HANDLER] Entering menu::role_menu_treeselect for role_id: {}",
        role_id
    );

    // 1. 获取所有状态正常的菜单，用于构建完整的菜单树
    let all_menus = service::select_menu_list_for_treeselect(&state.db).await?;
    let menu_tree = service::build_menu_treeselect(all_menus);

    // 2. 跨模块调用 role::service 来获取指定角色已关联的菜单ID
    let checked_keys = role::service::select_menu_ids_by_role_id(&state.db, role_id).await?;

    // 3. 手动构建前端期望的 JSON 响应体
    let response = json!({
        "code": 200,
        "msg": "操作成功",
        "menus": menu_tree,
        "checkedKeys": checked_keys,
    });

    info!(
        "[HANDLER] Responding to menu::role_menu_treeselect with {} root menus and {} checked keys.",
        menu_tree.len(),
        checked_keys.len()
    );

    Ok(Json(response))
}
