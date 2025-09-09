use super::model::FileUploadResponse;
use super::service;
use axum::extract::multipart::MultipartError;
use axum::extract::{Extension, Multipart, State};
use axum::Json;
use common::error::AppError;
use common::response::AjaxResult;
use framework::jwt::ClaimsData;
use framework::state::AppState;
use std::sync::Arc;
use tracing::{error, info};

pub async fn upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<ClaimsData>,
    mut multipart: Multipart,
) -> Result<Json<AjaxResult<FileUploadResponse>>, AppError> {
    info!("[HANDLER] Entering file::upload for user '{}'", claims.user_name);

    let next_field_result = multipart.next_field().await
        .map_err(|e: MultipartError| {
            error!("解析 multipart/form-data 字段失败: {:?}", e);
            AppError::ValidationFailed("文件上传请求格式错误".to_string())
        })?;

    if let Some(field) = next_field_result {
        if field.name() == Some("file") {
            let file_name = field.file_name().unwrap_or("unknown_file").to_string();

            let data = field.bytes().await.map_err(|e: MultipartError| {
                error!("读取上传文件字节流失败 for '{}': {:?}", file_name, e);
                AppError::ValidationFailed("读取文件内容失败".to_string())
            })?;

            info!("Receiving file '{}' ({} bytes) from user '{}'", file_name, data.len(), claims.user_name);

            let file_info = service::save_file(&state.db_pool, &claims.user_name, &file_name, &data).await?;
            let response = FileUploadResponse {
                code: 200,
                msg: "上传成功".to_string(),
                data: file_info, // 将 service 返回的 file_info 放入 data 字段
            };
            return Ok(Json(AjaxResult::success(response)));
        }
    }

    // 如果循环结束了（或一开始就没字段）都还没返回，说明没有找到 "file" 字段
    Err(AppError::ValidationFailed("请求中未找到名为 'file' 的文件字段".to_string()))
}