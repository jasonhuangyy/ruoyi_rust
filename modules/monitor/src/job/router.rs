use super::handler;
use crate::operlog::{extractor::BusinessType, layer::LogLayer};
use axum::handler::Handler;
use axum::{
    routing::{get, post, put},
    Router,
};
use framework::state::AppState;
use std::sync::Arc;

/// 定时任务路由
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/monitor/job/list", get(handler::list))
        .route(
            "/monitor/job/:jobId",
            get(handler::get_detail)
                .delete(handler::delete)
                .layer(LogLayer::new("定时任务", BusinessType::Delete)),
        )
        .route(
            "/monitor/job",
            post(handler::add)
                .layer(LogLayer::new("定时任务", BusinessType::Insert))
                .put(handler::update)
                .layer(LogLayer::new("定时任务", BusinessType::Update)),
        )
        .route(
            "/monitor/job/changeStatus",
            put(handler::change_status).layer(LogLayer::new("定时任务", BusinessType::Update)),
        )
        .route(
            "/monitor/job/run/:jobId",
            put(handler::run_once).layer(LogLayer::new("定时任务", BusinessType::Other)),
        )
        .route(
            "/monitor/job/export",
            post(handler::export.layer(LogLayer::new("定时任务", BusinessType::Export))),
        )
}
