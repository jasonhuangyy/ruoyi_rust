use super::handler;
use crate::dept;
use axum::handler::Handler;
use axum::routing::{get, post, put};
use axum::Router;
use framework::state::AppState;
use monitor::operlog::{extractor::BusinessType, layer::LogLayer};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // --- 个人中心 (Profile) ---
        .route(
            "/system/user/profile",
            get(handler::get_profile).put(handler::update_profile.layer(LogLayer::new("用户管理", BusinessType::Update))),
        )
        .route(
            "/system/user/profile/updatePwd",
            put(handler::update_pwd).layer(LogLayer::new("用户管理", BusinessType::Update)),
        )
        .route(
            "/system/user/profile/avatar",
            post(handler::update_avatar).layer(LogLayer::new("用户管理", BusinessType::Update)),
        )
        // --- 角色分配 (Auth Role) ---
        .route("/system/user/authRole/:userId", get(handler::get_auth_role))
        .route(
            "/system/user/authRole",
            put(handler::update_auth_role).layer(LogLayer::new("用户管理", BusinessType::Grant)),
        )
        .route("/system/user/deptTree", get(dept::handler::treeselect))
        .route("/system/user/list", get(handler::list))
        .route("/system/user/", get(handler::get_add_user_init_data))
        .route(
            "/system/user/:id",
            get(handler::get_detail).delete(handler::delete.layer(LogLayer::new("用户管理", BusinessType::Delete))),
        )
        .route(
            "/system/user",
            get(handler::get_add_user_init_data)
                .post(handler::add.layer(LogLayer::new("用户管理", BusinessType::Insert)))
                .put(handler::update.layer(LogLayer::new("用户管理", BusinessType::Update))),
        )
        .route(
            "/system/user/changeStatus",
            put(handler::change_status).layer(LogLayer::new("用户管理", BusinessType::Update)),
        )
        .route(
            "/system/user/resetPwd",
            put(handler::reset_pwd).layer(LogLayer::new("用户管理", BusinessType::Update)),
        )
        .route(
            "/system/user/export",
            post(handler::export.layer(LogLayer::new("用户管理", BusinessType::Export))),
        )
}
