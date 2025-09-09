use crate::{handler::*, dept, dict, menu, role, user, config, file, post};
use axum::{middleware, routing::{get, post}, Router};
use framework::{middleware::auth, state::AppState};
use monitor::operlog::middleware::OperLogLayer;
use std::sync::Arc;

pub fn api_router(state: Arc<AppState>) -> Router {
    let protected_router = Router::new()
        .route("/getInfo", get(get_info))
        .route("/getRouters", get(menu::handler::get_routers))
        .merge(config::router::router())
        .merge(menu::router::router())
        .merge(dept::router::router())
        .merge(dict::router::router())
        .merge(role::router::router())
        .merge(user::router::router())
        .merge(file::router::router())
        .merge(post::router::router());

    let protected_api = Router::new()
        .merge(protected_router)
        .layer(OperLogLayer) // 这是外层，后于 auth 执行
        .layer(middleware::from_fn_with_state(state.clone(), auth)); // 这是内层，先于 OperLogLayer 执行

    Router::new()
        .route("/captchaImage", get(get_captcha_image))
        .route("/login", post(login).get(login))
        .route("/logout", post(logout))
        .merge(protected_api)
        .with_state(state)
}