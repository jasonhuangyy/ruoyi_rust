use super::handler;
use crate::operlog::extractor::BusinessType;
use crate::operlog::layer::LogLayer;
use axum::handler::Handler;
use axum::{
    routing::{delete, get},
    Router,
};
use framework::state::AppState;
use std::sync::Arc;
use axum::routing::post;

/// 操作日志路由
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // 查询列表 
        .route("/monitor/operlog/list", get(handler::list))
        // 删除 
        .route(
            "/monitor/operlog/:operIds",
            delete(handler::delete.layer(LogLayer::new("操作日志", BusinessType::Delete))),
        )
        // 清空 
        .route(
            "/monitor/operlog/clean",
            delete(handler::clean.layer(LogLayer::new("操作日志", BusinessType::Clean))),
        )
        .route(
            "/monitor/operlog/export",
            post(handler::export.layer(LogLayer::new("操作日志", BusinessType::Export)))
        )
}