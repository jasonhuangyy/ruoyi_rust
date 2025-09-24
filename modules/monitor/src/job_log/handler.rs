use super::{
    model::{ListJobLogQuery, SysJobLog},
    service,
};
use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use common::{ response::AjaxResult};
use common::{error::AppError, page::TableDataInfo};
use framework::jwt::ClaimsData;
use framework::state::AppState;
use ruoyi_macros::require_permission;
use std::sync::Arc;
use tracing::info;

pub async fn list(State(state): State<Arc<AppState>>, Query(params): Query<ListJobLogQuery>) -> Result<Json<TableDataInfo<SysJobLog>>, AppError> {
    let list_data = service::select_job_log_list(&state.db_pool, params).await?;
    Ok(Json(list_data))
}

#[require_permission("monitor:job:remove")] // 沿用定时任务的删除权限
pub async fn remove(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Path(job_log_ids_str): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering jobLog::remove with ids: {}",
        job_log_ids_str
    );

    let ids: Vec<i64> = job_log_ids_str
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();

    if ids.is_empty() {
        return Err(AppError::ValidationFailed("未提供有效的日志ID".to_string()));
    }

    service::delete_job_log_by_ids(&state.db_pool, &ids).await?;

    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

#[require_permission("monitor:job:remove")] // 沿用定时任务的删除权限
pub async fn clean(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering jobLog::clean");

    service::clean_job_log(&state.db_pool).await?;

    Ok(Json(AjaxResult::<()>::success_msg("清空成功")))
}

#[require_permission("monitor:job:export")] // 沿用定时任务的导出权限
pub async fn export(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Form(params): Form<ListJobLogQuery>) -> Result<impl IntoResponse, AppError> {
    info!(
        "[HANDLER] Entering jobLog::export with params: {:?}",
        params
    );

    let excel_data = service::export_job_log_list(&state.db_pool, params).await?;

    let filename = format!(
        "job_log_{}.xlsx",
        chrono::Local::now().format("%Y%m%d%H%M%S")
    );

    let mut headers = HeaderMap::new();
    let disposition = format!("attachment; filename=\"{}\"", filename);
    headers.insert(
        header::CONTENT_TYPE,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            .parse()
            .unwrap(),
    );
    headers.insert(header::CONTENT_DISPOSITION, disposition.parse().unwrap());

    Ok((StatusCode::OK, headers, excel_data))
}
