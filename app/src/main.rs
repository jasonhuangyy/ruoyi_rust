use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::{extract::State, middleware, routing::get, Json, Router};
use common::error::AppError;
use common::models::online_model::SysUserOnline;
use framework::cache::AppCache;
use framework::jwt::{JwtConfig, JwtUtil};
use framework::{config::Settings, db, state::AppState};
use moka::future::Cache;
use monitor::job;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_cron_scheduler::JobScheduler;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), common::error::AppError> {
    dotenvy::dotenv().expect("Failed to load .env file");

    // 初始化日志
    framework::log::init_tracing();

    // 加载配置
    let settings = Settings::new()?;

    // 初始化数据库连接池
    // let db_pool = db::create_db_pool(&settings.database.url).await?;
    let db = db::new().await;

    // 从配置中创建 JWT 配置
    info!("[CONFIG] Loading JWT configuration from settings...");
    let jwt_config = JwtConfig {
        secret: settings.jwt.secret.clone(),             // 从 settings 中获取 secret
        issuer: settings.jwt.issuer.clone(),             // 从 settings 中获取 issuer
        expiration_hours: settings.jwt.expiration_hours, // 从 settings 中获取过期时间
    };
    info!(
        "[CONFIG] JWT configuration loaded successfully. Issuer: '{}'",
        jwt_config.issuer
    );

    // 创建 JwtUtil 实例
    let jwt_util = Arc::new(JwtUtil::new(jwt_config));

    // 初始化定时任务调度器
    info!("Initializing job scheduler...");
    let job_scheduler = JobScheduler::new().await.map_err(|e| {
        error!("Failed to create job scheduler: {}", e);
        AppError::JobSchedulerError(String::from("Failed to create job scheduler "))
    })?;
    let job_id_to_uuid_map = RwLock::new(HashMap::new());
    info!("Job scheduler created.");

    // 将连接池放入共享状态
    let app_state = Arc::new(AppState {
        db: db,
        jwt_util,
        job_scheduler,
        job_id_to_uuid_map,
        settings: settings.clone(),
        cache: AppCache::new(),
    });

    job::service::init_scheduler(app_state.clone()).await?;

    // 创建主路由
    let app = Router::new()
        // 使用 .nest() 方法将所有以 `/prod-api` 开头的请求路由到 system_api 中
        // 这样做的好处是，所有 system 模块的路由都自动带上了 `/prod-api` 前缀，
        // 与 RuoYi 前端配置的代理路径保持一致。
        // .route("/api/test-db", get(test_db_connection))
        .route(
            "/api/test-db",
            get(test_db_connection).with_state(app_state.clone()),
        )
        .nest(
            "/prod-api",
            system::router::api_router(app_state.clone()).merge(monitor::router::api_router(app_state.clone())),
        )
        .route("/health", axum::routing::get(health_check))
        // 为上传目录提供静态文件服务, 这个路由必须放在 /prod-api 之外，因为它是一个公共访问路径
        .nest_service("/uploads", ServeDir::new("uploads"))
        // 静态文件服务应该放在最后，作为回退
        .nest_service(
            "/",
            ServeDir::new("dist").fallback(tower_http::services::ServeFile::new("dist/index.html")), //.nest_service("/", ServeDir::new("static").fallback(ServeDir::new("static").append_index_html_on_directories(true)))
        )
        .layer(middleware::from_fn(debug_request_middleware))
        // 调试层
        // TraceLayer 必须放在最外层，才能捕获到所有请求
        .layer(TraceLayer::new_for_http());

    // 启动服务器
    let addr = format!("{}:{}", settings.server.host, settings.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid server address");

    info!("✅ Server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

// 数据库连接测试处理器

// 使用 State Extractor 从 Axum 中获取共享状态
async fn test_db_connection(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, AppError> {
    // 使用 sqlx::query! 宏执行一个简单的查询,这个宏会在编译时检查 SQL 语法和类型
    let result = sqlx::query("SELECT 1 as result")
        .fetch_one(&state.db)
        .await?;

    // 返回成功响应
    let response = serde_json::json!({
        "code": 200,
        "msg": "数据库连接成功",
        "data": { "result": result.result }
    });

    Ok(Json(response))
}

// 用于调试请求的中间件
async fn debug_request_middleware(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let (parts, body) = req.into_parts();

    // 将请求体读取为字节流,这里的 to_bytes 会消耗掉 body，所以需要重建它
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_e) => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // 从字节流中尝试解析为字符串并打印
    let body_str = String::from_utf8_lossy(&bytes);

    // 核心调试信息
    warn!("[DEBUG_MIDDLEWARE] ===> Request Received:");
    warn!("[DEBUG_MIDDLEWARE] Method: {}", parts.method);
    warn!("[DEBUG_MIDDLEWARE] URI: {}", parts.uri);
    warn!("[DEBUG_MIDDLEWARE] Headers: {:?}", parts.headers);
    warn!("[DEBUG_MIDDLEWARE] Body: {}", body_str);
    warn!("[DEBUG_MIDDLEWARE] <=== End of Request");

    // 重建请求，以便下游的处理器可以继续使用它
    let req = Request::from_parts(parts, Body::from(bytes));

    Ok(next.run(req).await)
}

#[test]
fn test_生成并验证密码() {
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
    // 生成  要哈希的明文密码
    let password_to_hash = b"admin123";

    // 生成一个随机的 salt
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 实例，使用默认参数
    let argon2 = Argon2::default();

    // 计算哈希,如果这里 panic，说明 argon2 库本身有问题
    let password_hash_obj = argon2
        .hash_password(password_to_hash, &salt)
        .expect("密码哈希生成失败!");

    // 获取最终要存入数据库的字符串
    let phc_string = password_hash_obj.to_string();

    println!("\n================== 密码生成结果 ==================");
    println!("请将下面这【一整行】完整的字符串复制到数据库 `sys_user` 表的 `password` 字段中：");
    println!("\n{}\n", phc_string);
    println!("==================================================\n");

    // 验证, 模拟从数据库读取刚生成的哈希字符串
    println!("正在模拟从数据库读取并验证...");

    // 尝试解析字符串,如果这里 panic，说明生成的字符串格式有问题，或者复制粘贴时损坏了。
    let parsed_hash = PasswordHash::new(&phc_string).expect("解析刚刚生成的哈希字符串失败! 这不应该发生。");
    println!("✅ 哈希字符串格式正确，解析成功。");

    // 验证密码是否匹配,如果这里出错，说明 argon2 的生成和验证逻辑不一致 (极不可能)
    assert!(
        Argon2::default()
            .verify_password(password_to_hash, &parsed_hash)
            .is_ok(),
        "验证失败! 明文密码与哈希值不匹配。"
    );
    println!("✅ 密码验证成功!");
    println!("\n测试通过! 可以安全地使用上面生成的哈希字符串。");
}
