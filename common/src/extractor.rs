use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use tracing::error;

/// 自定义的 JSON 提取器，在反序列化失败时会打印详细日志
#[derive(Debug, Clone, Copy, Default)]
pub struct DebugJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for DebugJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();

        // 先将 body 读取为字节流
        let bytes = match Bytes::from_request(Request::from_parts(parts, body), state).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("[DebugJson] Failed to read request body bytes: {:?}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
            }
        };

        // 尝试反序列化
        let de: Result<T, serde_json::Error> = serde_json::from_slice(&bytes);

        match de {
            Ok(value) => Ok(Self(value)),
            Err(err) => {
                // 手动构建错误响应
                let body_string = String::from_utf8_lossy(&bytes);
                error!(
                    "[DebugJson] Failed to deserialize JSON body.\nError: {:?}\nOriginal Body: {}",
                    err,
                    body_string
                );

                // 构建一个 400 Bad Request 响应，并将 serde 的错误信息作为响应体。
                // 这模拟了 Axum 内置 Json 提取器的行为。
                let response = (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to parse JSON body: {}", err)
                ).into_response();

                Err(response) 
            }
        }
    }
}