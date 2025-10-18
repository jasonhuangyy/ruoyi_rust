use super::{
    model::{AddJobVo, ChangeStatusVo, ListJobQuery, SysJob, UpdateJobVo},
    service,
};
use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use common::{error::AppError, page::TableDataInfo, response::AjaxResult};
use framework::jwt::ClaimsData;
use framework::state::AppState;
use ruoyi_macros::require_permission;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

/// 获取定时任务列表 (分页)
#[axum::debug_handler]
pub async fn list(State(state): State<Arc<AppState>>, Query(params): Query<ListJobQuery>) -> Result<Json<TableDataInfo<SysJob>>, AppError> {
    info!("[HANDLER] Entering job::list with params: {:?}", params);
    let list_data = service::select_job_list(&state.db, params).await?;
    Ok(Json(list_data))
}

/// 获取定时任务详细信息
#[axum::debug_handler]
pub async fn get_detail(State(state): State<Arc<AppState>>, Path(job_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    info!("[HANDLER] Entering job::get_detail with job_id: {}", job_id);
    let job = service::select_job_by_id(&state.db, job_id).await?;
    // RuoYi详情接口期望数据在 "data" 字段下
    Ok(Json(json!({
        "code": 200,
        "msg": "操作成功",
        "data": job
    })))
}

/// 新增定时任务
#[axum::debug_handler]
pub async fn add(State(state): State<Arc<AppState>>, Json(body): Json<AddJobVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering job::add with body: {:?}", body);
    service::add_job(state, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("新增成功")))
}

/// 修改定时任务
#[axum::debug_handler]
pub async fn update(State(state): State<Arc<AppState>>, Json(body): Json<UpdateJobVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering job::update with body: {:?}", body);
    service::update_job(state, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 删除定时任务
#[axum::debug_handler]
pub async fn delete(State(state): State<Arc<AppState>>, Path(job_ids_str): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering job::delete with ids: {}", job_ids_str);
    let ids: Vec<i64> = job_ids_str
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    if ids.is_empty() {
        return Err(AppError::InvalidCredentials);
    }
    service::delete_job_by_ids(state, &ids).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 修改任务状态
#[axum::debug_handler]
pub async fn change_status(State(state): State<Arc<AppState>>, Json(body): Json<ChangeStatusVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering job::change_status with body: {:?}",
        body
    );
    service::change_job_status(state, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("状态修改成功")))
}

/// 立即执行一次
#[axum::debug_handler]
pub async fn run_once(State(state): State<Arc<AppState>>, Path(job_id): Path<i64>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering job::run_once for job_id: {}", job_id);
    service::run_job_once(state, job_id).await?;
    Ok(Json(AjaxResult::<()>::success_msg("执行成功")))
}

#[require_permission("monitor:job:export")]
pub async fn export(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Form(params): Form<ListJobQuery>) -> Result<impl IntoResponse, AppError> {
    info!("[HANDLER] Entering job::export with params: {:?}", params);

    let excel_data = service::export_job_list(&state.db, params).await?;

    let filename = format!("job_{}.xlsx", chrono::Local::now().format("%Y%m%d%H%M%S"));

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
