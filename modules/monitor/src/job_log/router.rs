use super::handler;
use crate::operlog::extractor::BusinessType;
use crate::operlog::layer::LogLayer;
use axum::routing::{delete, post};
use axum::{routing::get, Router};
use framework::state::AppState;
use std::sync::Arc;

use axum::handler::Handler;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/monitor/jobLog/list", get(handler::list))
        .route(
            "/monitor/jobLog/clean", // 清空日志
            delete(handler::clean.layer(LogLayer::new("调度日志", BusinessType::Clean))),
        )
        .route(
            "/monitor/jobLog/:jobLogIds", // 批量删除日志
            delete(handler::remove.layer(LogLayer::new("调度日志", BusinessType::Delete))),
        )
        .route(
            "/monitor/jobLog/export",
            post(handler::export.layer(LogLayer::new("调度日志", BusinessType::Export))),
        )
}
