use super::handler;
use crate::operlog::extractor::BusinessType;
use crate::operlog::layer::LogLayer;
use axum::handler::Handler;
use axum::{
    routing::{delete, get, post},
    Router,
};
use framework::state::AppState;
use std::sync::Arc;

/// 登录日志路由
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // 查询列表  
        .route("/monitor/logininfor/list", get(handler::list))
        // 删除 
        .route(
            "/monitor/logininfor/:infoIds",
            delete(handler::delete.layer(LogLayer::new("登录日志", BusinessType::Delete))),
        )
        // 清空  
        .route(
            "/monitor/logininfor/clean",
            delete(handler::clean.layer(LogLayer::new("登录日志", BusinessType::Clean))),
        )
        // 解锁用户  
        .route(
            "/monitor/logininfor/unlock/:userName",
            post(handler::unlock.layer(LogLayer::new("登录日志", BusinessType::Update))), // RuoYi 中是POST方法
        )
}