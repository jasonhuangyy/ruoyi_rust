use super::handler;
use crate::operlog::{extractor::BusinessType, layer::LogLayer};
use axum::handler::Handler;
use axum::{
    routing::{delete, get},
    Router,
};
use framework::state::AppState;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // 获取在线用户列表，这是一个查询操作，通常不需要记录操作日志
        .route("/monitor/online/list", get(handler::list))
        // 强退用户，这是一个修改/删除性质的操作，需要记录操作日志
        .route(
            "/monitor/online/:tokenId",
            delete(handler::force_logout.layer(LogLayer::new("在线用户", BusinessType::Force))),
        )
}