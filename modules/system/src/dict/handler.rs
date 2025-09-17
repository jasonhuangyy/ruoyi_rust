use super::{
    model::{ListDictTypeQuery, SysDictType},
    service,
};
use crate::dict::model::{AddDictDataVo, AddDictTypeVo, DictTypeOptionVo, ListDictDataQuery, UpdateDictDataVo, UpdateDictTypeVo};
use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json
};
use common::models::dict_model::SysDictData;
use common::{
    error::AppError,
    page::TableDataInfo,
    response::AjaxResult,
};
use framework::jwt::ClaimsData;
use framework::state::AppState;
use ruoyi_macros::require_permission;
use serde_json::json;
use std::sync::Arc;


/// 获取字典类型列表 (分页)
pub async fn list_types(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListDictTypeQuery>,
) -> Result<Json<TableDataInfo<SysDictType>>, AppError> {
    // RuoYi 的分页接口通常不使用 AjaxResult 包装，而是直接返回 TableDataInfo
    let list = service::select_dict_type_list(&state.db_pool, params).await?;
    Ok(Json(list))
}

/// 刷新字典缓存
pub async fn refresh_cache(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AjaxResult<()>>, AppError> {
    service::refresh_dict_cache(&state.db_pool, &state.dict_cache).await?;
    Ok(Json(AjaxResult::<()>::success_msg("刷新成功")))
}

/// 获取字典类型详情
pub async fn get_type_detail(
    State(state): State<Arc<AppState>>,
    Path(dict_id): Path<i64>
) -> Result<impl IntoResponse, AppError> { // 1. 返回值类型改为更通用的 impl IntoResponse
    let dict_type = service::select_dict_type_by_id(&state.db_pool, dict_id).await?;

    // 2. 手动构建前端需要的、带有 "data" 键的 JSON 结构
    let response = json!({
        "code": 200,
        "msg": "操作成功",
        "data": dict_type // 将查询到的 dict_type 对象作为 "data" 字段的值
    });

    // 3. 将构建好的 JSON 返回
    Ok(Json(response))
}

/// 新增字典类型
pub async fn add_type(State(state): State<Arc<AppState>>, Json(body): Json<AddDictTypeVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::add_dict_type(&state.db_pool, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("新增成功")))
}

/// 修改字典类型
pub async fn update_type(State(state): State<Arc<AppState>>, Json(body): Json<UpdateDictTypeVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::update_dict_type(&state.db_pool, body, &state.dict_cache).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 删除字典类型
pub async fn delete_type(State(state): State<Arc<AppState>>, Path(dict_ids): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    let ids: Vec<i64> = dict_ids.split(',').map(|s| s.parse().unwrap_or(0)).collect();
    service::delete_dict_type_by_ids(&state.db_pool, ids).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 获取字典类型下拉框列表
pub async fn get_type_option_select(State(state): State<Arc<AppState>>) -> Result<Json<AjaxResult<Vec<DictTypeOptionVo>>>, AppError> {
    let list = service::get_dict_type_option_select(&state.db_pool).await?;
    Ok(Json(AjaxResult::success(list)))
}


/// 获取字典数据列表 (分页)
pub async fn list_data(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListDictDataQuery>,
) -> Result<Json<TableDataInfo<SysDictData>>, AppError> {
    let list = service::select_dict_data_list(&state.db_pool, params).await?;
    Ok(Json(list))
}

/// 获取字典数据详情
pub async fn get_data_detail(
    State(state): State<Arc<AppState>>,
    Path(dict_code): Path<i64>
) -> Result<impl IntoResponse, AppError> {
    let data = service::select_dict_data_by_code(&state.db_pool, dict_code).await?; 
    let response = json!({
        "code": 200,
        "msg": "操作成功",
        "data": data
    });

    Ok(Json(response))
}

/// 新增字典数据
pub async fn add_data(State(state): State<Arc<AppState>>, Json(body): Json<AddDictDataVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::add_dict_data(&state.db_pool, body, &state.dict_cache).await?;
    Ok(Json(AjaxResult::<()>::success_msg("新增成功")))
}

/// 修改字典数据
pub async fn update_data(State(state): State<Arc<AppState>>, Json(body): Json<UpdateDictDataVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::update_dict_data(&state.db_pool, body, &state.dict_cache).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 删除字典数据
pub async fn delete_data(State(state): State<Arc<AppState>>, Path(dict_codes): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    let codes: Vec<i64> = dict_codes.split(',').map(|s| s.parse().unwrap_or(0)).collect();
    service::delete_dict_data_by_codes(&state.db_pool, codes, &state.dict_cache).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}


/// 根据字典类型获取字典数据列表 (核心对外接口)
pub async fn get_data_by_type(
    State(state): State<Arc<AppState>>,
    Path(dict_type): Path<String>, // 从路径中获取 dict_type
) -> Result<Json<AjaxResult<Vec<SysDictData>>>, AppError> {
    println!("[HANDLER] Received request for dict_type: {}", dict_type);
    let dict_data = service::select_dict_data_by_type(&state.db_pool, &state.dict_cache, &dict_type).await?;
    Ok(Json(AjaxResult::success(dict_data)))
}


#[require_permission("system:dict:export")] // 加上权限控制
pub async fn export_types(
    State(state): State<Arc<AppState>>,
    Extension(_claims): Extension<ClaimsData>,
    Form(params): Form<ListDictTypeQuery>, // 使用 Form 提取器
) -> Result<impl IntoResponse, AppError> {
    let excel_data = service::export_dict_type_list(&state.db_pool, params).await?;

    let filename = format!("dict_type_{}.xlsx", chrono::Local::now().format("%Y%m%d%H%M%S"));

    let mut headers = HeaderMap::new();
    let disposition = format!("attachment; filename=\"{}\"", filename);
    headers.insert(
        header::CONTENT_TYPE,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".parse().unwrap()
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        disposition.parse().unwrap()
    );

    Ok((StatusCode::OK, headers, excel_data))
}