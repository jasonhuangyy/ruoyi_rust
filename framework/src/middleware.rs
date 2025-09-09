use crate::state::AppState;
use axum::extract::State;
use axum::{
    http::Request,
    middleware::Next,
    response::Response,
};
use common::error::AppError;
use std::sync::Arc;
use tracing::{error, info, instrument};

#[instrument(skip_all)]  
pub async fn auth(
    State(state): State<Arc<AppState>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    info!("[AUTH] Auth middleware entered.");

    // 提取 Authorization header
    let auth_header = match req.headers().get("Authorization") {
        Some(header) => {
            info!("[AUTH] 'Authorization' header found.");
            header.to_str()
        }
        None => {
            error!("[AUTH] 'Authorization' header NOT FOUND. Rejecting request.");
            return Err(AppError::TokenInvalid);
        }
    };

    let auth_str = match auth_header {
        Ok(s) => {
            info!("[AUTH] Header converted to string successfully: '{}'", s);
            s
        },
        Err(_) => {
            error!("[AUTH] Header contains invalid UTF-8 characters. Rejecting request.");
            return Err(AppError::TokenInvalid);
        }
    };

    // 提取 Bearer Token
    let token = match auth_str.strip_prefix("Bearer ") {
        Some(t) => {
            info!("[AUTH] 'Bearer ' prefix stripped. Token: '{}'", t);
            t
        }
        None => {
            error!("[AUTH] 'Bearer ' prefix NOT FOUND. Rejecting request.");
            return Err(AppError::TokenInvalid);
        }
    };

    // 检查Token是否在黑名单中
    info!("[AUTH] Checking token against blacklist...");
    if state.token_blacklist_cache.contains_key(token) {
        error!("[AUTH] Token is BLACKLISTED. Rejecting request.");
        // 返回 TokenInvalid，前端会触发登出流程
        return Err(AppError::TokenInvalid);
    }
    info!("[AUTH] Token not in blacklist. Proceeding to verification.");

    // 验证 Token
    info!("[AUTH] Verifying token...");
    let claims_data = match state.jwt_util.verify(token) {
        Ok(claims) => {
            info!("[AUTH] Token verification successful for user: '{}'", claims.user_name);
            claims
        }
        Err(e) => {
            // 这里 e 是 AppError，打印它的 Display 实现
            error!("[AUTH] Token verification FAILED. Error: {}. Rejecting request.", e);
            return Err(AppError::TokenInvalid); // 直接返回，不再用之前的 `e`
        }
    };

    // 注入扩展并继续
    info!("[AUTH] Injecting ClaimsData and AppState into request extensions.");
    req.extensions_mut().insert(claims_data);
    req.extensions_mut().insert(state.clone()); // 注入 state

    info!("[AUTH] Passing request to the next layer.");
    Ok(next.run(req).await)
}


