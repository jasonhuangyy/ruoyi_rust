use crate::operlog::middleware::OperLogLayer;
use crate::{job, job_log, logininfor, online, operlog};
use axum::{middleware, Router};
use framework::middleware::auth;
use framework::state::AppState;
use std::sync::Arc;

pub fn api_router(state: Arc<AppState>) -> Router {
    let monitor_routes = Router::new()
        .merge(operlog::router::router())
        .merge(logininfor::router::router())
        .merge(online::router::router())
        .merge(job::router::router())
        .merge(job_log::router::router());

    let protected_api = Router::new()
        .merge(monitor_routes)
        .layer(OperLogLayer) 
        .layer(middleware::from_fn_with_state(state.clone(), auth)); 
 
    Router::new()
        .merge(protected_api)
        .with_state(state)
}