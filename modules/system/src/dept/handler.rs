use super::{
    model::{AddDeptVo, ListDeptQuery, SysDept, UpdateDeptVo},
    service,
};
use crate::dept::model::DeptTreeSelectVo;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use common::{error::AppError, response::AjaxResult};
use framework::state::AppState;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

/// 获取部门列表 
///
/// # 修复说明
/// 此接口之前返回一个原始的 JSON 数组 `[ {..}, {..} ]`。
/// RuoYi 前端 `getList` 方法期望收到一个 `{ code, msg, data }` 结构，
/// 并从 `response.data` 中获取数组。
/// 返回值类型从 `Json<Vec<SysDept>>` 改为 `Json<AjaxResult<Vec<SysDept>>>`，
/// 并使用 `AjaxResult::success()` 对结果进行包装。
pub async fn list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListDeptQuery>,
) -> Result<Json<AjaxResult<Vec<SysDept>>>, AppError> {
    info!("[HANDLER] Entering dept::list with params: {:?}", params);

    // 调用 service 获取扁平列表，这部分逻辑不变
    let dept_list = service::select_dept_list(
        &state.db_pool,
        params.dept_name.as_deref(),
        params.status.as_deref(),
    ).await?;

    // 使用 AjaxResult::success() 包装返回的数据
    info!("[HANDLER] dept::list returning {} departments, wrapped in AjaxResult.", dept_list.len());
    Ok(Json(AjaxResult::success(dept_list)))
}


/// 获取部门详细信息
pub async fn get_detail(
    State(state): State<Arc<AppState>>,
    Path(dept_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> { // 1. 返回值类型改为更通用的 impl IntoResponse
    let dept = service::select_dept_by_id(&state.db_pool, dept_id).await?;

    // 2. 手动构建前端需要的、带有 "data" 键的 JSON 结构
    let response = json!({
        "code": 200,
        "msg": "操作成功",
        "data": dept // 将查询到的 dept 对象作为 "data" 字段的值
    });

    // 3. 将构建好的 JSON 返回
    Ok(Json(response))
}

/// 新增部门
pub async fn add(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddDeptVo>,
) -> Result<Json<AjaxResult<()>>, AppError> {
    service::add_dept(&state.db_pool, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("新增成功")))
}

/// 修改部门
pub async fn update(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateDeptVo>,
) -> Result<Json<AjaxResult<()>>, AppError> {
    service::update_dept(&state.db_pool, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 删除部门
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(dept_id): Path<i64>,
) -> Result<Json<AjaxResult<()>>, AppError> {
    service::delete_dept_by_id(&state.db_pool, dept_id).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 获取部门下拉树列表
///
/// # 修复说明
/// 此接口被多个地方（如用户管理、部门管理本身）调用，用于渲染部门选择树。
/// 之前的实现返回裸数组 `[...]`，导致用户管理页面解析JSON失败。
/// 同样，我们将其返回值用 `AjaxResult` 包装，以提供统一和健壮的接口。
pub async fn treeselect(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AjaxResult<Vec<DeptTreeSelectVo>>>, AppError> {
    info!("[HANDLER] Entering dept::treeselect (AjaxResult Wrapped Version)");
    // 服务层逻辑保持不变
    let depts = service::select_dept_list_for_treeselect(&state.db_pool).await?;
    let dept_tree = service::build_dept_treeselect(depts);

    // 使用 AjaxResult::success() 包装返回的树形结构
    info!("[HANDLER] dept::treeselect returning {} root nodes, wrapped in AjaxResult.", dept_tree.len());
    Ok(Json(AjaxResult::success(dept_tree)))
}

/// 获取部门列表（排除指定ID及其子节点）
///
/// 此接口专门用于“修改部门”弹窗中的“上级部门”下拉树，
/// 以防止将部门的上级设置为其自身或其子部门。
pub async fn list_exclude_child(
    State(state): State<Arc<AppState>>,
    Path(dept_id): Path<i64>, // 从路径中获取要排除的部门ID
) -> Result<Json<AjaxResult<Vec<SysDept>>>, AppError> {
    info!("[HANDLER] Entering dept::list_exclude_child for dept_id: {}", dept_id);

    // 调用新的 service 函数
    let dept_list = service::select_dept_list_exclude_child(&state.db_pool, dept_id).await?;

    // 使用 AjaxResult::success() 包装返回的数据
    info!("[HANDLER] dept::list_exclude_child returning {} departments.", dept_list.len());
    Ok(Json(AjaxResult::success(dept_list)))
}
