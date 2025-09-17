
use super::handler;
use axum::handler::Handler;
use axum::{
    routing::{delete, get, post},
    Router,
};
use framework::state::AppState;
use monitor::operlog::extractor::BusinessType;
use monitor::operlog::layer::LogLayer;
use std::sync::Arc;


pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/config/list", get(handler::list))
        .route("/system/config/refreshCache", delete(handler::refresh_cache)) // RuoYi 使用的是 DELETE 方法
        // 放在增删改查路由前面，防止路径冲突
        .route("/system/config/configKey/:configKey", get(handler::get_config_by_key))
        .route("/system/config/:configId", get(handler::get_info).delete(handler::remove))
        .route("/system/config", post(handler::add).put(handler::edit))
        .route(
            "/system/config/export", // 使用 post 方法，并为其添加操作日志
            post(handler::export.layer(LogLayer::new("参数管理", BusinessType::Export))),
        )
}