use std::fmt;
use crate::response::AjaxResult;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use axum::extract::multipart::MultipartError;
use config::ConfigError;
use rust_xlsxwriter::XlsxError; 
use tracing::error;

/// 应用错误枚举，定义了不同类型的错误
#[derive(Debug)]
pub enum AppError {
    // 系统级/不可预见的错误 (应返回 500)
    DatabaseError(sqlx::Error),
    ConfigError(ConfigError),
    PasswordHashError(String),
    JwtError,
    JobSchedulerError(String),
    // 业务逻辑错误 (应返回 400/422)
    /// 硬件认证失败，消息由认证服务器提供
    HardwareAuthFailed(String),
    /// SSO认证失败，消息由内部逻辑生成
    SsoAuthFailed(String),
    /// 登录凭证无效（用户名、密码）
    InvalidCredentials,
    /// 验证码错误
    CaptchaError,
    /// 验证码过期
    CaptchaExpired,
    /// 记录未找到
    RecordNotFound,
    /// 通用业务验证失败 (例如，用户名已存在)
    ValidationFailed(String),

    // 认证/授权错误 (应返回 401/403)
    /// Token无效或过期
    TokenInvalid,
    /// 权限不足
    PermissionDenied,
    // 用于表示JSON解析或序列化错误
    JsonParseError(serde_json::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("[AppError] An application error occurred: {}", self);

        // 1. 根据错误类型，映射到 (HTTP状态码, 业务码, 消息)
        let (http_status, business_code, message) = match self {
            // 系统级错误 -> 500
            AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误".to_string()),
            AppError::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器配置错误".to_string()),
            AppError::PasswordHashError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 500, "密码处理异常".to_string()),
            AppError::JwtError => (StatusCode::INTERNAL_SERVER_ERROR, 500, "令牌处理异常".to_string()),
            AppError::JobSchedulerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, 500, format!("定时任务调度失败: {}", msg)),
            // 业务逻辑错误 -> 400 Bad Request
            // 前端将通过检查 HTTP 状态码是否为 400/422 来识别这些错误
            AppError::HardwareAuthFailed(msg) => (StatusCode::BAD_REQUEST, 500, msg),
            AppError::SsoAuthFailed(msg) => (StatusCode::BAD_REQUEST, 500, msg),
            AppError::InvalidCredentials => (StatusCode::BAD_REQUEST, 500, "用户名或密码错误".to_string()),
            AppError::CaptchaError => (StatusCode::BAD_REQUEST, 400, "验证码错误".to_string()),
            AppError::CaptchaExpired => (StatusCode::BAD_REQUEST, 500, "验证码已过期".to_string()),
            AppError::RecordNotFound => (StatusCode::BAD_REQUEST, 404, "请求的资源不存在".to_string()),
            AppError::ValidationFailed(msg) => (StatusCode::BAD_REQUEST, 400, msg),

            // 认证/授权错误
            AppError::TokenInvalid => (StatusCode::UNAUTHORIZED, 401, "令牌无效或已过期".to_string()),
            AppError::PermissionDenied => (StatusCode::FORBIDDEN, 403, "权限不足".to_string()),

            AppError::JsonParseError(e) => (StatusCode::BAD_REQUEST, 400, format!("JSON格式错误: {}", e)),
        };

        // 2. 构建统一的 AjaxResult 响应体
        let response_body = AjaxResult::<()>::error(business_code, &message);

        // 3. 组合最终的 HTTP 响应
        (http_status, Json(response_body)).into_response()
    }
}

// Display trait 用于日志记录
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DatabaseError(e) => write!(f, "Database Error: {}", e),
            AppError::ConfigError(e) => write!(f, "Configuration Error: {}", e),
            AppError::PasswordHashError(d) => write!(f, "Password Hashing Error: {}", d),
            AppError::JwtError => write!(f, "JWT Generation/Verification Error"),
            AppError::JobSchedulerError(m) => write!(f, "Job Scheduler Error: {}",m),
            AppError::HardwareAuthFailed(m) => write!(f, "Hardware Auth Failed: {}", m),
            AppError::SsoAuthFailed(m) => write!(f, "SSO Auth Failed: {}", m),
            AppError::InvalidCredentials => write!(f, "Invalid Credentials"),
            AppError::CaptchaError => write!(f, "Captcha Error"),
            AppError::CaptchaExpired => write!(f, "Captcha Expired"),
            AppError::RecordNotFound => write!(f, "Record Not Found"),
            AppError::ValidationFailed(m) => write!(f, "Validation Failed: {}", m),
            AppError::TokenInvalid => write!(f, "Token Invalid or Expired"),
            AppError::PermissionDenied => write!(f, "Permission Denied"),
            AppError::JsonParseError(e) => write!(f, "JSON Parse/Serialize Error: {}", e),
        }
    }
}

// 为 sqlx::Error 实现 From trait 保持不变
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err)
    }
}

impl From<ConfigError> for AppError {
    fn from(err: ConfigError) -> Self {
        AppError::ConfigError(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::JsonParseError(err)
    }
}


impl From<XlsxError> for AppError {
    fn from(err: XlsxError) -> Self {
        error!("[XLSX_ERROR] Failed to generate Excel file: {}", err);
        AppError::ValidationFailed("生成Excel文件失败".to_string())
    }
}

impl From<MultipartError> for AppError {
    fn from(err: MultipartError) -> Self {
        // 在日志中记录详细的技术错误
        error!("Multipart processing error: {}", err);
        // 返回一个对用户友好的业务错误
        AppError::ValidationFailed("文件上传失败，请检查文件格式或大小。".to_string())
    }
}
 