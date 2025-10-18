//! 岗位管理模块的处理器层

use super::{
    model::{AddPostVo, ListPostQuery, PostOptionVo, SysPost, UpdatePostVo},
    service,
};
use axum::http::{header, HeaderMap, StatusCode};
use axum::{
    extract::{Form, Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use common::{auth::Permission, error::AppError, page::TableDataInfo, response::AjaxResult};
use framework::{jwt::ClaimsData, state::AppState};
use ruoyi_macros::require_permission;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

/// 获取岗位列表 (分页)
#[require_permission(Permission::PostList)]
pub async fn list(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Query(params): Query<ListPostQuery>) -> Result<Json<TableDataInfo<SysPost>>, AppError> {
    info!("[HANDLER] Entering post::list with params: {:?}", params);
    let list_data = service::select_post_list(&state.db_pool, params).await?;
    Ok(Json(list_data))
}

/// 获取岗位详细信息
#[require_permission(Permission::PostQuery)]
pub async fn get_info(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Path(post_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    let post = service::select_post_by_id(&state.db_pool, post_id).await?;
    let response = json!({
        "code": 200,
        "msg": "操作成功",
        "data": post
    });
    Ok(Json(response))
}

/// 新增岗位
#[require_permission(Permission::PostAdd)]
pub async fn add(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>, Json(data): Json<AddPostVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::add_post(&state.db_pool, data, &claims.user_name).await?;
    Ok(Json(AjaxResult::<()>::success_msg("新增成功")))
}

/// 修改岗位
#[require_permission(Permission::PostEdit)]
pub async fn edit(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>, Json(data): Json<UpdatePostVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    service::update_post(&state.db_pool, data, &claims.user_name).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 删除岗位
#[require_permission(Permission::PostRemove)]
pub async fn remove(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Path(ids): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    let post_ids: Vec<i64> = ids.split(',').filter_map(|s| s.parse().ok()).collect();
    service::delete_post_by_ids(&state.db_pool, &post_ids).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 获取岗位选择下拉框
pub async fn optionselect(State(state): State<Arc<AppState>>) -> Result<Json<AjaxResult<Vec<PostOptionVo>>>, AppError> {
    info!("[HANDLER] Entering post::optionselect");
    let options = service::select_all_posts_options(&state.db_pool).await?;
    Ok(Json(AjaxResult::success(options)))
}

/// 导出岗位列表
#[require_permission(Permission::PostRemove)] // RuoYi 中导出和删除权限相同
pub async fn export(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Form(params): Form<ListPostQuery>) -> Result<impl IntoResponse, AppError> {
    info!("[HANDLER] Entering post::export with params: {:?}", params);

    // 1. 调用服务层生成 Excel 字节流
    let excel_data = service::export_post_list(&state.db_pool, params).await?;

    // 2. 构造文件名
    let filename = format!("post_{}.xlsx", chrono::Local::now().format("%Y%m%d%H%M%S"));

    // 3. 创建一个 HeaderMap 来存储我们的响应头
    let mut headers = HeaderMap::new();

    // 4. 创建一个拥有所有权的 String 来存储 Content-Disposition 的值
    let disposition = format!("attachment; filename=\"{}\"", filename);

    // 5. 将 Header 插入到 HeaderMap 中
    // .parse().unwrap() 是安全的，因为我们提供的字符串是有效的 Header 值
    headers.insert(
        header::CONTENT_TYPE,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            .parse()
            .unwrap(),
    );
    headers.insert(header::CONTENT_DISPOSITION, disposition.parse().unwrap());

    // 6. 构建新的响应元组，这次使用 HeaderMap 而不是数组
    // (StatusCode, HeaderMap, Body) 也是一个实现了 IntoResponse 的有效元组
    let response = (StatusCode::OK, headers, excel_data);

    Ok(response)
}
