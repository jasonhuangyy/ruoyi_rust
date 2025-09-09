use super::handler;
use axum::{routing::post, Router};
use framework::state::AppState;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/system/file/upload", post(handler::upload))
}