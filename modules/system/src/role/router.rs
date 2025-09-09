use super::handler;
use axum::{
    routing::{get, post, put, },
    Router,
};
use framework::state::AppState;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/role/list", get(handler::list))
        .route("/system/role", post(handler::add))
        .route("/system/role", put(handler::update))
        .route("/system/role/changeStatus", put(handler::change_status))
        .route("/system/role/:roleId", get(handler::get_detail).delete(handler::delete))
}