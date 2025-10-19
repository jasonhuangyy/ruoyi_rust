use super::{model::ListLogininforQuery, service};
use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use common::{error::AppError, page::TableDataInfo, response::AjaxResult};
use entity::prelude::SysLogininforModel;
use framework::jwt::ClaimsData;
use framework::state::AppState;
use ruoyi_macros::require_permission;
use std::sync::Arc;
use tracing::info;

/// 获取登录日志列表 (分页)
pub async fn list(State(state): State<Arc<AppState>>, Query(params): Query<ListLogininforQuery>) -> Result<Json<TableDataInfo<SysLogininforModel>>, AppError> {
    info!(
        "[HANDLER] Entering logininfor::list with params: {:?}",
        params
    );
    let list_data = service::select_logininfor_list(&state.db, params).await?;
    Ok(Json(list_data))
}

/// 删除登录日志
pub async fn delete(State(state): State<Arc<AppState>>, Path(info_ids_str): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering logininfor::delete with ids: {}",
        info_ids_str
    );
    let ids: Vec<i64> = info_ids_str
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    if ids.is_empty() {
        return Err(AppError::InvalidCredentials); // 可以定义一个更合适的错误类型
    }
    service::delete_logininfor_by_ids(&state.db, &ids).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 清空登录日志
pub async fn clean(State(state): State<Arc<AppState>>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering logininfor::clean");
    service::clean_logininfor(&state.db).await?;
    Ok(Json(AjaxResult::<()>::success_msg("清空成功")))
}

/// 解锁用户
/// 注意：RuoYi 的这个功能放在登录日志里，但实际上是修改 sys_user 表的状态。
/// 暂不实现此功能，但保留接口定义以备将来扩展。
pub async fn unlock(Path(user_name): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering logininfor::unlock for user: {}",
        user_name
    );
    // TODO: 实现解锁用户的业务逻辑
    // 1. 调用 user::service 更新用户状态
    // 2. 如果有缓存，需要清理用户缓存
    Ok(Json(AjaxResult::<()>::success_msg("解锁功能待实现")))
}

#[require_permission("monitor:logininfor:export")]
pub async fn export(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Form(params): Form<ListLogininforQuery>) -> Result<impl IntoResponse, AppError> {
    info!(
        "[HANDLER] Entering logininfor::export with params: {:?}",
        params
    );

    let excel_data = service::export_logininfor_list(&state.db, params).await?;

    let filename = format!(
        "logininfor_{}.xlsx",
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
