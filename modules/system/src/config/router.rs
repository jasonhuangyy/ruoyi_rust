use super::handler;
use axum::{
    routing::{delete, get, post, },
    Router,
};
use framework::state::AppState;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/config/list", get(handler::list))
        .route("/system/config/refreshCache", delete(handler::refresh_cache)) // RuoYi 使用的是 DELETE 方法
        // 放在增删改查路由前面，防止路径冲突
        .route("/system/config/configKey/:configKey", get(handler::get_config_by_key))
        .route("/system/config/:configId", get(handler::get_info).delete(handler::remove))
        .route("/system/config", post(handler::add).put(handler::edit))
}