use super::{
    model::{AddRoleVo, ChangeStatusVo, ListRoleQuery, UpdateRoleVo},
    service,
};
use axum::response::IntoResponse;
use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    Extension, Json,
};
use common::{auth::Permission, error::AppError, page::TableDataInfo, response::AjaxResult};
use entity::prelude::SysRoleModel;
use framework::jwt::ClaimsData;
use framework::state::AppState;
use ruoyi_macros::require_permission;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

/// 获取角色列表 (分页)
#[require_permission(Permission::RoleList)]
pub async fn list(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Query(params): Query<ListRoleQuery>) -> Result<Json<TableDataInfo<SysRoleModel>>, AppError> {
    info!("[HANDLER] Entering role::list with params: {:?}", params);
    let list_data = service::select_role_list(&state.db, params).await?;
    Ok(Json(list_data))
}

/// 获取角色详细信息
pub async fn get_detail(State(state): State<Arc<AppState>>, Path(role_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    info!(
        "[HANDLER] Entering role::get_detail with role_id: {}",
        role_id
    );

    // 1. 获取角色基本信息
    let role = service::select_role_by_id(&state.db, role_id).await?;
    // 2. 获取角色关联的菜单ID列表
    let checked_keys = service::select_menu_ids_by_role_id(&state.db, role_id).await?;

    let response = json!({
        "code": 200,
        "msg": "操作成功",
        // 将角色信息 SysRole 序列化后，作为 "data" 字段的值
        "data": role,
        // 将 checkedKeys 作为顶层字段
        "checkedKeys": checked_keys,
    });

    Ok(Json(response))
}

/// 新增角色
pub async fn add(State(state): State<Arc<AppState>>, Json(body): Json<AddRoleVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering role::add with body: {:?}", body);
    service::add_role(&state.db, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("新增成功")))
}

/// 修改角色
pub async fn update(State(state): State<Arc<AppState>>, Json(body): Json<UpdateRoleVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering role::update with body: {:?}", body);
    service::update_role(&state.db, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 删除角色 (逻辑删除)
/// 路径参数 `role_ids` 是一个用逗号分隔的字符串，例如 "1,2,3"
pub async fn delete(State(state): State<Arc<AppState>>, Path(role_ids_str): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering role::delete with ids: {}", role_ids_str);
    // 解析字符串为 i64 类型的 Vec
    let ids: Vec<i64> = role_ids_str
        .split(',')
        .filter_map(|s| s.parse::<i64>().ok())
        .collect();

    if ids.is_empty() {
        return Err(AppError::InvalidCredentials); // 可以定义一个更合适的错误类型
    }

    service::delete_role_by_ids(&state.db, ids).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 修改角色状态
pub async fn change_status(State(state): State<Arc<AppState>>, Json(body): Json<ChangeStatusVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering role::change_status with body: {:?}",
        body
    );
    service::change_role_status(&state.db, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("状态修改成功")))
}

#[require_permission("system:role:export")]
pub async fn export(
    State(state): State<Arc<AppState>>,
    Extension(_claims): Extension<ClaimsData>,
    Form(params): Form<ListRoleQuery>, // 前端使用 application/x-www-form-urlencoded, 所以用 Form
) -> Result<impl IntoResponse, AppError> {
    info!("[HANDLER] Entering role::export with params: {:?}", params);

    let excel_data = service::export_role_list(&state.db, params).await?;

    let filename = format!("role_{}.xlsx", chrono::Local::now().format("%Y%m%d%H%M%S"));

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
