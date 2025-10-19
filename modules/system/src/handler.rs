use crate::model::{CaptchaVo, LoginRequest, LoginVo, UserDetailVo, UserInfoVo};
use crate::user;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::ConnectInfo;
use axum::response::IntoResponse;
use axum::{extract::State, Extension, Json};
use captcha::Captcha;
use chrono::Local;
use common::error::AppError;
use common::models::online_model::SysUserOnline;
use common::response::AjaxResult;
use entity::prelude::SysLogininforModel;
use framework::jwt::ClaimsData;
use framework::state::AppState;
use jwt_simple::prelude::*;
use monitor::logininfor;
use monitor::logininfor::model::SysLogininfor;
use sqlx::DatabaseConnnection;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info, instrument};
use uuid::Uuid;

#[derive(Serialize)]
pub struct LoginData {
    #[serde(rename = "token")]
    token: String,
}

/// 用于封装 getInfo 接口返回的数据结构
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoData {
    user: UserDetail,
    roles: Vec<String>,
    permissions: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetail {
    user_id: i64,
    user_name: String,
    nick_name: String,
    // ... 可以添加更多用户详情字段，如 avatar, email等
}

async fn record_login_log(db: &DatabaseConnnection, user_name: String, ipaddr: String, status: &'static str, msg: String) {
    let log = SysLogininforModel {
        info_id: 0,
        user_name: Some(user_name),
        ipaddr: Some(ipaddr),
        // 以下字段可以后续通过 User-Agent 解析库或 IP 地址库来填充
        login_location: None,
        browser: None,
        os: None,
        status: Some(status.to_string()),
        msg: Some(msg),
        login_time: Some(Local::now().naive_local()),
    };

    // 在一个独立的后台任务中执行数据库写入
    tokio::spawn(async move {
        if let Err(e) = logininfor::service::add_logininfor(db, log).await {
            // 这里的错误只会打印到服务器日志，不会影响主登录流程
            error!("[LOG_TASK] 记录登录日志失败: {:?}", e);
        } else {
            info!("[LOG_TASK] 登录日志记录成功。");
        }
    });
}

#[instrument(skip(state, payload))]
pub async fn login(State(state): State<Arc<AppState>>, Extension(addr): Extension<ConnectInfo<SocketAddr>>, Json(payload): Json<LoginRequest>) -> Result<Json<AjaxResult<LoginVo>>, AppError> {
    let ipaddr = addr.ip().to_string();

    let db = state.db.clone();

    let (user_name, password, _code, _uuid) = (
        payload.username,
        payload.password,
        payload.code,
        payload.uuid,
    );
    info!(
        "[LOGIN_HANDLER] 收到standard登录请求: user='{}', ip='{}'",
        user_name, ipaddr
    );

    // 1.1 验证码校验
    //#[cfg(not(debug_assertions))]
    if state.settings.security.captcha_enabled {
        match state.captcha_cache.get(&_uuid).await {
            Some(correct_code) if correct_code.to_lowercase() == _code.to_lowercase() => {
                state.captcha_cache.invalidate(&_uuid).await;
            }
            _ => {
                record_login_log(
                    state.db.clone(),
                    user_name.clone(),
                    addr.ip().to_string(),
                    "1",
                    "验证码错误或已过期".to_string(),
                )
                .await;
                return Err(AppError::CaptchaError);
            }
        }
    }
    // 用户名密码校验
    let db_user = match user::service::select_user_by_username(&state.db, &user_name).await? {
        Some(u) => u,
        None => {
            error!("[LOGIN_HANDLER] 用户 '{}' 不存在.", &user_name);
            record_login_log(db, user_name, ipaddr, "1", "用户不存在".to_string()).await;
            return Err(AppError::InvalidCredentials);
        }
    };

    // 使用 if let 解构 Option，避免在同步闭包中 await
    // let db_user_clone = db_user.clone();
    let password_from_db = if let Some(pwd) = db_user.password {
        pwd
    } else {
        // 如果密码是 None，记录日志并返回错误
        error!(
            "[LOGIN_HANDLER] 数据库中用户 '{}' 的密码字段为 NULL!",
            user_name
        );
        record_login_log(
            db.clone(),
            user_name.clone(),
            ipaddr.clone(),
            "1",
            "用户密码未设置".to_string(),
        )
        .await;
        return Err(AppError::InvalidCredentials);
    };

    let parsed_hash = PasswordHash::new(&password_from_db).map_err(|e| {
        error!(
            "[LOGIN_HANDLER] 密码哈希字符串解析失败! Error: {}. Hash: '{}'",
            e, password_from_db
        );
        tokio::spawn(record_login_log(
            db.clone(),
            user_name.clone(),
            ipaddr.clone(),
            "1",
            "服务端密码处理错误".to_string(),
        ));
        AppError::PasswordHashError(e.to_string())
    })?;

    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        record_login_log(
            state.db.clone(),
            user_name.clone(),
            addr.ip().to_string(),
            "1",
            "密码验证失败".to_string(),
        )
        .await;
        Err(AppError::InvalidCredentials)?
    }

    // --- 3. 生成 JWT ---
    info!("[LOGIN_HANDLER] 生成 JWT 开始...");
    let jti = Uuid::new_v4().to_string();
    info!("[LOGIN_HANDLER] Generated JTI for token: {}", jti);
    let claims_data = ClaimsData {
        user_id: db_user.user_id,
        user_name: db_user.user_name.clone(),
        jti: jti.clone(), // <-- 将 jti 放入 claims
    };
    let token = state.jwt_util.sign(claims_data)?;
    info!("[LOGIN_HANDLER] 生成 JWT 成功.");

    info!("[LOGIN_HANDLER] 准备记录在线用户信息...");
    let user_online = SysUserOnline {
        token_id: jti.clone(), // 前端需要的 tokenId 就是我们的 jti
        user_name: db_user.user_name.clone(),
        ipaddr: Some(ipaddr.clone()),
        login_location: Some("内网IP".to_string()), // TODO: 后续可通过IP地址库解析
        browser: Some("Unknown".to_string()),       // TODO: 后续可通过 User-Agent 解析
        os: Some("Unknown".to_string()),            // TODO: 后续可通过 User-Agent 解析
        login_time: Local::now(),                   // 使用 chrono::Local 获取当前时间
        token: token.clone(),                       // 存储完整token用于黑名单
    };

    // 使用 .insert() 将用户数据存入在线缓存。moka 会自动处理 TTL。
    state.online_user_cache.insert(jti, user_online).await;
    info!("[LOGIN_HANDLER] 在线用户信息已存入缓存.");

    // --- 4. 记录成功日志并返回 ---
    record_login_log(db, db_user.user_name, ipaddr, "0", "登录成功".to_string()).await;

    let vo = LoginVo { token };
    info!("[LOGIN_HANDLER] 登录流程全部完成，返回 Token.");
    Ok(Json(AjaxResult::success(vo)))
}

/// 处理获取当前登录用户信息的请求
// 使用 Extension 提取器，从请求扩展中获取由 auth 中间件注入的 ClaimsData
pub async fn get_info(State(state): State<Arc<AppState>>, Extension(claims): Extension<ClaimsData>) -> Result<impl IntoResponse, AppError> {
    info!(
        "[HANDLER] Entering get_info for user_id: {}",
        claims.user_id
    );

    // 1. 查询用户基本信息
    // 即使 claims 中有部分信息，也从数据库重新查询以获取最新数据（如昵称）
    // 这里复用 user 模块的 service，而不是自己写 SQL
    let user_detail = user::service::select_user_by_id(&state.db, claims.user_id).await?;
    info!(
        "[HANDLER] Fetched user details: nick_name='{}'",
        user_detail.nick_name
    );

    // 2. 查询用户的角色列表
    // 调用刚刚在 user::service 中创建的新函数
    let roles = user::service::get_user_roles(&state.db, claims.user_id).await?;
    info!("[HANDLER] Fetched user roles, count: {}", roles.len());

    // 3. 查询用户的权限列表
    // 调用刚刚在 user::service 中创建的新函数
    let permissions = user::service::get_user_permissions(&state.db, claims.user_id).await?;
    info!(
        "[HANDLER] Fetched user permissions, count: {}",
        permissions.len()
    );

    // 4. 封装成前端需要的业务数据 VO (View Object)
    let vo = UserInfoVo {
        user: UserDetailVo {
            user_id: user_detail.user_id,
            user_name: user_detail.user_name,
            nick_name: user_detail.nick_name,
            // 可以在这里添加更多字段，如 avatar, email...
        },
        roles,
        permissions,
    };

    Ok(Json(AjaxResult::success(vo)))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptchaImageResponse {
    img: String,
    uuid: String,
}

#[derive(Serialize)]
pub struct CaptchaData {
    uuid: String,
    img: String,
}

/// 处理获取验证码图片的请求
#[axum::debug_handler]
pub async fn get_captcha_image(State(state): State<Arc<AppState>>) -> Json<AjaxResult<CaptchaVo>> {
    let captcha_enabled = state.settings.security.captcha_enabled;

    let (uuid, img) = if captcha_enabled {
        // 1. 在所有 .await 调用之前，完成所有与非 `Send` 变量 `captcha` 相关的操作。

        let captcha_text: String;
        let captcha_img_base64: String;

        {
            // 将 `captcha` 的生命周期限制在这个代码块中。
            // 1. 使用 `captcha` 库生成一个数学验证码
            let mut captcha = Captcha::new();
            captcha
                .add_chars(4) // 验证码长度为4个字符
                // .apply_filter(Noise::new(0.2)) // 添加噪声干扰
                // .apply_filter(Wave::new(2.0, 10.0).horizontal()) // 添加水平扭曲
                // .apply_filter(Wave::new(2.0, 10.0).vertical()) // 添加垂直扭曲
                .set_color([20, 40, 80])
                .view(200, 70); // 设置字符颜色

            // 立即从 captcha 对象中提取出我们需要的所有数据。  这些数据 (String类型) 都是 `Send` 的。
            // 2. 从生成的验证码中获取文本和图片数据
            // `.chars()` 方法会根据配置（add_chars(4)）生成一个包含4个随机字符的 Vec<char>
            captcha_text = captcha.chars().iter().collect();
            // `.as_base64()` 方法是在 `Captcha` 对象上调用的，它会使用内部生成的字符来绘制图片。所以这行不需要改。
            captcha_img_base64 = captcha.as_base64().unwrap(); // 获取图片的 Base64 编码
        } // 在这里，非 `Send` 的 `captcha` 变量被销毁了。

        // 3. 生成一个唯一的 UUID 作为 key
        let uuid = Uuid::new_v4().to_string();

        // 4. 将 UUID 和验证码答案存入缓存
        // .insert() 是一个异步方法，需要 .await
        // 将答案转为小写存储，以便后续不区分大小写比较
        state
            .captcha_cache
            .insert(uuid.clone(), captcha_text.to_lowercase())
            .await;
        (uuid, captcha_img_base64)
    } else {
        ("".to_string(), "".to_string())
    };
    let vo = CaptchaVo {
        captcha_enabled,
        uuid,
        img,
    };
    // 使用统一的成功响应
    Json(AjaxResult::success(vo))
}

/// 用于封装通用成功响应的结构体
#[derive(Serialize)]
pub struct SuccessResponse {
    code: u16,
    msg: String,
}

/// 处理用户登出请求
pub async fn logout() -> Json<SuccessResponse> {
    // 对于JWT，服务端通常不需要做任何事。
    // 如果需要强制token失效，可以将其加入一个黑名单缓存（例如用moka缓存）。
    // 但对于基础实现，直接返回成功即可。
    Json(SuccessResponse {
        code: 200,
        msg: "注销成功".to_string(),
    })
}
