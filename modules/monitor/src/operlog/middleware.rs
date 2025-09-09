use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Local;
use futures_util::future::BoxFuture;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tokio;
use tower::{Layer, Service};
use tracing::{error, info};

use super::{
    extractor::LogInfo,
    model::SysOperLog,
    service as operlog_service,
};
use framework::{jwt::ClaimsData, state::AppState};

/// 一个自定义的层，用于包裹所有受保护的路由，以实现操作日志记录
#[derive(Clone)]
pub struct OperLogLayer;

impl<S> Layer<S> for OperLogLayer {
    type Service = OperLogMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OperLogMiddleware { inner }
    }
}

/// 操作日志中间件服务
#[derive(Clone)]
pub struct OperLogMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for OperLogMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // --- 1. 准备 ---
            let start_time = Instant::now();
            let oper_time = Local::now().naive_local();

            // 从请求扩展中克隆出所需信息
            // auth 中间件必须先运行，否则这里会得到 None
            let claims = req.extensions().get::<ClaimsData>().cloned();
            let app_state = req.extensions().get::<Arc<AppState>>().cloned();

            let oper_url = req.uri().path().to_string();
            let request_method = req.method().to_string();
            let oper_ip = req.extensions().get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.ip().to_string())
                .unwrap_or_default();

            // 捕获请求体，以便记录 oper_param
            let (parts, body) = req.into_parts();
            let body_bytes = match to_bytes(body, usize::MAX).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("[OperLogMiddleware] Failed to read request body: {}", e);
                    let response = (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read request body: {}", e)).into_response();
                    return Ok(response);
                }
            };
            let oper_param = String::from_utf8_lossy(&body_bytes).to_string();
            // 重建请求，以便下游可以继续处理
            let req = Request::from_parts(parts, Body::from(body_bytes));

            // --- 2. 执行下游处理器 ---
            let response = inner.call(req).await?;

            info!("[OPER_LOG_MIDDLEWARE] <=== Downstream handler finished. Checking for LogInfo in response extensions...");
            // LogLayer 会在处理完请求后，将 LogInfo 放入响应的扩展中，作为需要记录日志的信号。
            let log_info = response.extensions().get::<Arc<LogInfo>>().cloned();

            // 只有当 LogInfo 存在时才记录
            if let (Some(log_info), Some(state)) = (log_info, app_state) {
                info!("[OPER_LOG_MIDDLEWARE] ✔✔✔ LogInfo FOUND for '{}'. Preparing to record log.", log_info.title);

                // --- 3. 记录日志 ---
                let cost_time = start_time.elapsed().as_millis() as i64;
                // 为了记录 json_result，需要消耗并重建响应体
                let (res_parts, res_body) = response.into_parts();
                let res_body_bytes = to_bytes(res_body, usize::MAX).await.unwrap_or_default();
                let json_result = String::from_utf8_lossy(&res_body_bytes).to_string();

                let (log_status, error_msg) = if res_parts.status.is_success() {
                    (0, None) // 0 代表成功
                } else {
                    // 如果请求失败，尝试从响应体中解析错误消息
                    let msg = serde_json::from_slice::<serde_json::Value>(&res_body_bytes)
                        .ok()
                        .and_then(|v| v.get("msg").and_then(|m| m.as_str().map(String::from)));
                    (1, msg) // 1 代表失败
                };

                // 构建日志实体
                let log = SysOperLog {
                    oper_id: 0,
                    title: Some(log_info.title.clone()),
                    business_type: Some(log_info.business_type.into()),
                    method: None, // RuoYi 这个字段通常是 "包名.类名.方法名", 在Rust中意义不大，可留空
                    request_method: Some(request_method),
                    operator_type: Some(1), // 1=后台用户, 2=手机端用户
                    oper_name: claims.map(|c| c.user_name),
                    dept_name: None, // 需要额外查询，暂时留空
                    oper_url: Some(oper_url),
                    oper_ip: Some(oper_ip),
                    oper_location: None, // IP地址定位，可后续实现
                    oper_param: Some(oper_param),
                    json_result: Some(json_result),
                    status: Some(log_status),
                    error_msg,
                    oper_time: Some(oper_time),
                    cost_time: Some(cost_time),
                };

                // 使用 tokio::spawn 将数据库写入操作放到后台执行，不阻塞主响应流程
                tokio::spawn(async move {
                    if let Err(e) = operlog_service::add_oper_log(&state.db_pool, log).await {
                        error!("[LOG_TASK] Failed to add operation log: {:?}", e);
                    }
                });

                // 将响应体重建并返回
                let response = Response::from_parts(res_parts, Body::from(res_body_bytes));
                return Ok(response);
            }else {
                info!("[OPER_LOG_MIDDLEWARE] ❌❌❌ LogInfo NOT FOUND. Skipping log recording for this request.");
            }

            // 如果没有 LogInfo，直接返回原始响应
            Ok(response)
        })
    }
}