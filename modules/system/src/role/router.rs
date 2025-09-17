use super::handler;
use axum::handler::Handler;
use axum::{
    routing::{get, post, put, },
    Router,
};
use framework::state::AppState;
use monitor::operlog::extractor::BusinessType;
use monitor::operlog::layer::LogLayer;
use std::sync::Arc;


pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/role/list", get(handler::list))
        .route("/system/role", post(handler::add))
        .route("/system/role", put(handler::update))
        .route("/system/role/changeStatus", put(handler::change_status))
        .route("/system/role/:roleId", get(handler::get_detail).delete(handler::delete))
        .route(
            "/system/role/export",
            post(handler::export.layer(LogLayer::new("角色管理", BusinessType::Export)))
        )
}