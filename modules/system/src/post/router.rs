use crate::post::handler;
use axum::handler::Handler;
use axum::{
    routing::{get, post},
    Router,
};
use framework::state::AppState;
use monitor::operlog::{extractor::BusinessType, layer::LogLayer};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // 查询类接口
        .route("/system/post/list", get(handler::list))
        .route("/system/post/optionselect", get(handler::optionselect))
        .route(
            "/system/post/:ids",
            get(handler::get_info).delete(handler::remove.layer(LogLayer::new("岗位管理", BusinessType::Delete))),
        )
        // 修改类接口 (应用操作日志中间件)
        .route(
            "/system/post",
            post(handler::add.layer(LogLayer::new("岗位管理", BusinessType::Insert))).put(handler::edit.layer(LogLayer::new("岗位管理", BusinessType::Update))),
        ) .route(
        "/system/post/export",
        post(handler::export.layer(LogLayer::new("岗位管理", BusinessType::Export)))
    )
}
