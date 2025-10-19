use super::service;
use axum::{
    extract::{Path, State},
    Json,
};
use common::models::online_model::SysUserOnline;
use common::{error::AppError, page::TableDataInfo, response::AjaxResult};
use framework::state::AppState;
use std::sync::Arc;
use tracing::info;

/// 获取在线用户列表
/// RuoYi前端期望一个 TableDataInfo 结构，即使数据不是从数据库分页查询的。
pub async fn list(State(state): State<Arc<AppState>>) -> Result<Json<TableDataInfo<SysUserOnline>>, AppError> {
    info!("[HANDLER] Entering online::list");

    // 调用服务层从缓存获取列表
    let user_list = service::list_online_users(&state.cache.online_user_cache).await;

    // 将缓存列表包装成前端需要的 TableDataInfo 格式
    let total = user_list.len() as i64;
    let table_data = TableDataInfo::new(user_list, total);

    Ok(Json(table_data))
}

/// 强退用户
pub async fn force_logout(
    State(state): State<Arc<AppState>>,
    Path(token_id): Path<String>, // 从路径中获取 tokenId
) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering online::force_logout for tokenId: {}",
        token_id
    );

    // 调用服务层执行强退逻辑
    service::force_logout(&state, &token_id).await?;

    // 返回标准成功响应
    Ok(Json(AjaxResult::<()>::success_msg("强退成功")))
}
