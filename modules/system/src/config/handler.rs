use super::{
    model::{AddConfigVo, ListConfigQuery, SysConfig, UpdateConfigVo},
    service,
};
use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode}, // 导入 HeaderMap 和 StatusCode
    response::IntoResponse,
    Extension,
    Json,
};
use common::{auth::Permission, error::AppError, page::TableDataInfo, response::AjaxResult};
use framework::{jwt::ClaimsData, state::AppState};
use ruoyi_macros::require_permission;
use serde_json::json;
use std::sync::Arc;

/// 获取参数配置列表 (分页)
// #[require_permission("system:config:list")]
#[require_permission(Permission::ConfigList)]
pub async fn list(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Query(params): Query<ListConfigQuery>) -> Result<Json<TableDataInfo<SysConfig>>, AppError> {
    let list_data = service::select_config_list(&state.db, params).await?;

    Ok(Json(list_data))
}

/// 获取参数配置详细信息
#[require_permission("system:config:query")]
pub async fn get_info(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Path(config_id): Path<i32>) -> Result<impl IntoResponse, AppError> {
    let config = service::select_config_by_id(&state.db, config_id).await?;
    let response = json!({
        "code": 200,
        "msg": "操作成功",
        "data": config
    });
    Ok(Json(response)) // 用 Json() 包装
}

/// 根据参数键名获取参数值
pub async fn get_config_by_key(State(state): State<Arc<AppState>>, Path(config_key): Path<String>) -> Result<impl IntoResponse, AppError> {
    let config_value = service::select_config_value_by_key(&state, &config_key).await?;
    let response = json!({
        "code": 200,
        "msg": config_value
    });

    Ok(Json(response))
}

/// 新增参数配置
#[require_permission("system:config:add")]
pub async fn add(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>, Json(data): Json<AddConfigVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::add_config(&state, data, &claims.user_name).await?;
    Ok(Json(AjaxResult::<&str>::success_msg("新增成功")))
}

/// 修改参数配置
#[require_permission("system:config:edit")]
pub async fn edit(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>, Json(data): Json<UpdateConfigVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::update_config(&state, data, &claims.user_name).await?;
    Ok(Json(AjaxResult::<&str>::success_msg("修改成功")))
}

/// 删除参数配置
#[require_permission("system:config:remove")]
pub async fn remove(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Path(ids): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    let config_ids: Vec<i32> = ids.split(',').filter_map(|s| s.parse().ok()).collect();
    service::delete_config_by_ids(&state, &config_ids).await?;
    Ok(Json(AjaxResult::<&str>::success_msg("删除成功")))
}

/// 刷新参数缓存
#[require_permission("system:config:remove")]
pub async fn refresh_cache(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::refresh_cache(&state).await?;
    Ok(Json(AjaxResult::<&str>::success_msg("刷新成功")))
}
#[require_permission("system:config:export")] // 建议添加权限，根据RuoYi习惯应该是这个
pub async fn export(
    State(state): State<Arc<AppState>>,
    Extension(_claims): Extension<ClaimsData>,
    Form(params): Form<ListConfigQuery>, // 使用 Form 提取器来接收 x-www-form-urlencoded 数据
) -> Result<impl IntoResponse, AppError> {
    let excel_data = service::export_config_list(&state.db, params).await?;

    let filename = format!(
        "config_{}.xlsx",
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
