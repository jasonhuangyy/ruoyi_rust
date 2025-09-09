use super::extractor::LogInfo;
use axum::http::Request;
use axum::response::Response;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::info;

/// LogLayer 用于将 LogInfo 注入到请求和响应的扩展中。
#[derive(Clone)]
pub struct LogLayer {
    log_info: Arc<LogInfo>,
}

impl LogLayer {
    /// 创建一个新的 LogLayer
    pub fn new(title: &str, business_type: super::extractor::BusinessType) -> Self {
        Self {
            log_info: Arc::new(LogInfo {
                title: title.to_string(),
                business_type,
            }),
        }
    }
}

/// 实现 tower::Layer trait，使 LogLayer 可以被应用到 Axum 的路由上。
impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LogService {
            inner,
            log_info: self.log_info.clone(),
        }
    }
}

/// LogService 是 LogLayer 应用后产生的实际服务。
#[derive(Clone)]
pub struct LogService<S> {
    inner: S,
    log_info: Arc<LogInfo>,
}

/// 实现 tower::Service trait，处理具体的请求。
impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for LogService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        info!(
            "[LOG_LAYER] ---> Entering LogLayer for '{}' (Type: {:?}). Injecting into request extensions.",
            self.log_info.title, self.log_info.business_type
        );
        // 请求时：注入 LogInfo 到请求扩展
        // 使外层的中间件（如 OperLogMiddleware）可以提前知道日志元数据
        req.extensions_mut().insert(self.log_info.clone());

        // 为了能修改响应，需要克隆 inner service
        let mut inner = self.inner.clone();
        let log_info_for_response = self.log_info.clone();

        Box::pin(async move {
            // 将请求传递给内部服务(inner)，比如 handler
            let mut response = inner.call(req).await?;
            info!(
                "[LOG_LAYER] <--- Exiting LogLayer for '{}'. Injecting into response extensions.",
                log_info_for_response.title
            );
            // 响应时：注入 LogInfo 到响应扩展 
            response.extensions_mut().insert(log_info_for_response);

            Ok(response)
        })
    }
}