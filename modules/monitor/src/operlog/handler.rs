use super::{
    model::{ListOperLogQuery, SysOperLog},
    service,
};
use axum::{
    extract::{Form, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use common::{error::AppError, page::TableDataInfo, response::AjaxResult};
use framework::jwt::ClaimsData;
use framework::state::AppState;
use ruoyi_macros::require_permission;
use std::sync::Arc;
use tracing::info;

/// 获取操作日志列表 (分页)
pub async fn list(State(state): State<Arc<AppState>>, Query(params): Query<ListOperLogQuery>) -> Result<Json<TableDataInfo<SysOperLog>>, AppError> {
    info!("[HANDLER] Entering operlog::list with params: {:?}", params);
    let list_data = service::select_oper_log_list(&state.db, params).await?;
    Ok(Json(list_data))
}

/// 删除操作日志
pub async fn delete(State(state): State<Arc<AppState>>, axum::extract::Path(oper_ids_str): axum::extract::Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering operlog::delete with ids: {}",
        oper_ids_str
    );
    let ids: Vec<i64> = oper_ids_str
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    if ids.is_empty() {
        return Err(AppError::InvalidCredentials); // 可以定义一个更合适的错误类型
    }
    service::delete_oper_log_by_ids(&state.db, &ids).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 清空操作日志
pub async fn clean(State(state): State<Arc<AppState>>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering operlog::clean");
    service::clean_oper_log(&state.db).await?;
    Ok(Json(AjaxResult::<()>::success_msg("清空成功")))
}

#[require_permission("monitor:operlog:export")]
pub async fn export(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Form(params): Form<ListOperLogQuery>) -> Result<impl IntoResponse, AppError> {
    info!(
        "[HANDLER] Entering operlog::export with params: {:?}",
        params
    );

    let excel_data = service::export_oper_log_list(&state.db, params).await?;

    let filename = format!(
        "operlog_{}.xlsx",
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
