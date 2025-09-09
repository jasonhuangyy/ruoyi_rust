use crate::{job, logininfor, online, operlog};
use axum::Router;
use framework::state::AppState;
use std::sync::Arc;

pub fn api_router(state: Arc<AppState>) -> Router {
    // 创建一个受保护的路由，因为所有监控功能都需要登录
    let protected_router = Router::new()
        .merge(operlog::router::router()) // 合并操作日志的路由
        .merge(logininfor::router::router()) 
        .merge(online::router::router())
        .merge(job::router::router())
        .with_state(state); // 将共享状态应用到所有路由

    protected_router
}