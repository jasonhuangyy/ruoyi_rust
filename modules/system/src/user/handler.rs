use super::{
    model::{AddUserVo, ChangeStatusVo, ListUserQuery, ResetPwdVo, UpdateUserVo, UserDetailVo, UserListVo},
    service,
};
use crate::post;
use crate::role::service as role_service;
// 引入角色服务
use crate::user::model::{AddUserInitVo, AuthRoleVo, UpdateAuthRoleVo, UpdateAvatarVo, UpdateProfileVo, UpdatePwdVo, UserProfileVo};
use axum::extract::Multipart;
use axum::http::{header, HeaderMap, StatusCode};
use axum::{
    extract::{Form, Path, Query, State},
    response::IntoResponse, Extension,
    Json,
};
use common::extractor::DebugJson;
use common::{auth::Permission, error::AppError, page::TableDataInfo, response::AjaxResult};
use framework::jwt::ClaimsData;
use framework::state::AppState;
use ruoyi_macros::require_permission;
use std::sync::Arc;
use tracing::{info, instrument};

/// 获取当前登录用户的个人信息
pub async fn get_profile(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>) -> Result<Json<AjaxResult<UserProfileVo>>, AppError> {
    info!(
        "[HANDLER] Entering user::get_profile for user_id: {}",
        claims.user_id
    );
    let profile_data = service::get_user_profile(&state.db_pool, claims.user_id).await?;
    // 使用 AjaxResult::success 包装，它会自动将字段拍平
    Ok(Json(AjaxResult::success(profile_data)))
}

/// 用户修改个人密码
pub async fn update_pwd(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>, Json(body): Json<UpdatePwdVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering user::update_pwd for user_id: {}",
        claims.user_id
    );
    service::update_user_pwd(&state.db_pool, claims.user_id, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 获取用户列表 (分页)
// #[require_permission("system:user:query")]
// #[require_permission(Permission::UserQuery)]
#[require_permission(all = [Permission::UserList, "system:user:query"])]
// #[require_permission(any = [Permission::UserList, "system:user:query"])]
pub async fn list(State(state): State<Arc<AppState>>, Extension(_claims): Extension<ClaimsData>, Query(params): Query<ListUserQuery>) -> Result<Json<TableDataInfo<UserListVo>>, AppError> {
    info!("[HANDLER] Entering user::list with params: {:?}", params);
    let list_data = service::select_user_list(&state.db_pool, params).await?;
    Ok(Json(list_data))
}

/// RuoYi前端在点击"新增"按钮时，会发送一个不带ID的GET请求到 /system/user/
/// 这个接口需要返回所有可用的角色列表，以便在表单中渲染。
pub async fn get_add_user_init_data(State(state): State<Arc<AppState>>) -> Result<Json<AjaxResult<AddUserInitVo>>, AppError> {
    info!("[HANDLER] Entering user::get_add_user_init_data (for add user modal)");

    // 1. 查询所有可用的角色列表
    // 复用之前的逻辑，查询全量、非分页的角色列表
    let all_roles = role_service::select_role_list(
        &state.db_pool,
        // 传入一个默认的查询参数，表示查询所有
        crate::role::model::ListRoleQuery {
            ..Default::default()
        },
    )
    .await?
    .rows;
    let all_posts = post::service::select_post_all(&state.db_pool).await?;

    // 2. 组装成前端期望的 UserDetailVo 结构
    // `data` 字段为 null 或一个空的 SysUser 对象，`role_ids` 为空数组。
    let vo = AddUserInitVo {
        roles: all_roles,
        posts: all_posts, // 如果实现了岗位管理，在这里填充
    };

    Ok(Json(AjaxResult::success(vo)))
}

/// 获取用户详细信息
pub async fn get_detail(State(state): State<Arc<AppState>>, Path(user_id): Path<i64>) -> Result<Json<AjaxResult<UserDetailVo>>, AppError> {
    info!(
        "[HANDLER] Entering user::get_detail with user_id: {}",
        user_id
    );

    // 1. 查询用户基本信息
    // 2. 查询所有可用的角色列表,RuoYi前端需要所有角色用于选择，所以查询全量列表，不分页
    // 3. 查询当前用户已关联的角色ID

    // 4. 查询所有可用的岗位列表
    // 5. 查询当前用户已关联的岗位ID
    // 6. 组装成前端需要的 UserDetailVo 结构
    let user_data = service::select_user_by_id(&state.db_pool, user_id)
        .await
        .ok();
    let all_roles = role_service::select_role_list(&state.db_pool, Default::default())
        .await?
        .rows;
    let role_ids = service::select_role_ids_by_user_id(&state.db_pool, user_id).await?;

    let all_posts = post::service::select_post_all(&state.db_pool).await?;

    let post_ids = service::select_post_ids_by_user_id(&state.db_pool, user_id).await?;

    let vo = UserDetailVo {
        data: user_data,
        roles: all_roles,
        role_ids,
        posts: all_posts,
        post_ids,
    };
    // 直接返回这个复杂的VO，前端会把它包装在 `data` 字段里
    Ok(Json(AjaxResult::success(vo)))
}

#[instrument(skip(state, body))]
pub async fn add(State(state): State<Arc<AppState>>, DebugJson(body): DebugJson<AddUserVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER_ADD] Entering clean 'add' handler.");

    // 只执行核心业务
    service::add_user(&state.db_pool, body).await?;

    Ok(Json(AjaxResult::<()>::success_msg("新增成功")))
}

/// 修改用户
pub async fn update(State(state): State<Arc<AppState>>, Json(body): Json<UpdateUserVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering user::update with body: {:?}", body);
    service::update_user(&state.db_pool, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("修改成功")))
}

/// 删除用户
pub async fn delete(State(state): State<Arc<AppState>>, Path(user_ids_str): Path<String>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!("[HANDLER] Entering user::delete with ids: {}", user_ids_str);
    let ids: Vec<i64> = user_ids_str
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    if ids.is_empty() {
        return Err(AppError::InvalidCredentials);
    }
    service::delete_user_by_ids(&state.db_pool, &ids).await?;
    Ok(Json(AjaxResult::<()>::success_msg("删除成功")))
}

/// 修改用户状态
pub async fn change_status(State(state): State<Arc<AppState>>, Json(body): Json<ChangeStatusVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering user::change_status with body: {:?}",
        body
    );
    service::change_user_status(&state.db_pool, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("状态修改成功")))
}

/// 重置密码
pub async fn reset_pwd(State(state): State<Arc<AppState>>, Json(body): Json<ResetPwdVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering user::reset_pwd for user_id: {}",
        body.user_id
    );
    service::reset_user_pwd(&state.db_pool, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("密码重置成功")))
}

/// 获取指定用户的角色分配信息
#[instrument(skip(state))]
pub async fn get_auth_role(State(state): State<Arc<AppState>>, Path(user_id): Path<i64>) -> Result<Json<AuthRoleVo>, AppError> {
    info!("[HANDLER] Entering get_auth_role for user_id: {}", user_id);
    let vo = service::get_auth_role(&state.db_pool, user_id).await?;
    // 注意：此接口直接返回业务VO，不包装在AjaxResult中，以匹配前端行为
    Ok(Json(vo))
}

/// 更新指定用户的角色分配
#[instrument(skip(state, body))]
pub async fn update_auth_role(State(state): State<Arc<AppState>>, Json(body): Json<UpdateAuthRoleVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering update_auth_role for user_id: {}",
        body.user_id
    );
    service::update_auth_role(&state.db_pool, body).await?;
    Ok(Json(AjaxResult::<()>::success_msg("授权成功")))
}

/// 更新当前登录用户的基本资料
#[instrument(skip(state, claims, body))]
pub async fn update_profile(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>, Json(body): Json<UpdateProfileVo>) -> Result<Json<AjaxResult<()>>, AppError> {
    info!(
        "[HANDLER] Entering update_profile for current user_id: {}",
        claims.user_id
    );
    service::update_user_profile(&state.db_pool, claims.user_id, body).await?;
    Ok(Json(AjaxResult::<String>::success_msg("修改成功")))
}
/// 更新当前登录用户的头像
#[instrument(skip(state, claims, multipart))]
pub async fn update_avatar(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>, mut multipart: Multipart) -> Result<Json<UpdateAvatarVo>, AppError> {
    info!(
        "[HANDLER] Entering update_avatar for current user_id: {}",
        claims.user_id
    );

    // 从 multipart 中提取文件
    if let Some(field) = multipart.next_field().await? {
        if field.name() == Some("avatarfile") {
            let data = field.bytes().await?;
            let img_url = service::update_user_avatar(&state.db_pool, claims.user_id, &data).await?;

            // 返回前端特定的 JSON 格式
            let vo = UpdateAvatarVo {
                img_url,
                msg: "上传成功".to_string(),
            };
            return Ok(Json(vo));
        }
    }

    Err(AppError::ValidationFailed(
        "请求中未找到名为 'avatarfile' 的文件字段".to_string(),
    ))
}


#[require_permission("system:user:export")] // RuoYi 原始权限标识
pub async fn export(State(state): State<Arc<AppState>>,    Extension(_claims): Extension<ClaimsData>,    Form(params): Form<ListUserQuery>, // 复用列表查询的参数结构体
) -> Result<impl IntoResponse, AppError> {
    info!("[HANDLER] Entering user::export with params: {:?}", params);

    let excel_data = service::export_user_list(&state.db_pool, params).await?;

    let filename = format!("user_{}.xlsx", chrono::Local::now().format("%Y%m%d%H%M%S"));

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
