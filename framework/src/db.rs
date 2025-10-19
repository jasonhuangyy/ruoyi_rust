use log::LevelFilter;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{str::FromStr, time::Duration};
use tracing::{debug, info};

pub async fn create_db_pool(database_url: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    info!("正在创建数据库连接池...");

    let pool = PgPoolOptions::new()
        .max_connections(10) // 设置最大连接数
        .min_connections(2) // 设置最小连接数
        .idle_timeout(Duration::from_secs(8)) // 空闲连接超时时间
        .connect(database_url) // 直接使用 .connect() 方法
        .await?;

    info!("✅ 数据库连接池创建成功。");
    Ok(pool)
}

#[derive(Deserialize, Debug, Clone)]
struct DbConfig {
    database_url: String,
    search_path: Option<String>,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub idle_timeout: Option<Duration>,
    pub sqlx_logging: Option<bool>,
    pub sqlx_logging_level: Option<String>,
}
pub async fn new() -> DatabaseConnection {
    // 从环境变量中读取数据库配置信息，如果读取失败，则输出错误信息
    let config = envy::from_env::<DbConfig>().expect("数据库连接配置错误");

    // 输出数据库连接配置信息，用于调试目的
    debug!("数据库连接配置: {:?}", config);

    // 根据环境变量中的配置或使用默认值来设置SQLx日志记录级别
    let level = config
        .sqlx_logging_level
        .and_then(|s| LevelFilter::from_str(s.as_str()).ok())
        .unwrap_or(LevelFilter::Info);

    // 创建数据库连接选项对象，并配置基本的连接参数
    let mut opts = ConnectOptions::new(config.database_url);

    // 配置连接池的最大连接数
    opts.max_connections(config.max_connections.unwrap_or(10))
        // 配置连接池的最小连接数
        .min_connections(config.min_connections.unwrap_or(2))
        // 设置连接超时时间
        .connect_timeout(Duration::from_secs(10))
        // 设置获取连接的超时时间
        .acquire_timeout(Duration::from_secs(10))
        // 设置空闲超时时间，超过该时间的连接将被关闭
        .idle_timeout(config.idle_timeout.unwrap_or(Duration::from_secs(60)))
        // 配置是否启用SQLx日志记录
        .sqlx_logging(config.sqlx_logging.unwrap_or(false))
        // 设置SQLx日志记录级别
        .sqlx_logging_level(level)
        // 配置慢查询日志记录设置
        .sqlx_slow_statements_logging_settings(log::LevelFilter::Info, Duration::from_secs(120));

    // 配置 postgres search_path
    if let Some(path) = config.search_path {
        opts.set_schema_search_path(path);
    }

    // 使用配置好的连接选项建立数据库连接，如果连接失败，则输出错误信息
    Database::connect(opts).await.expect("数据库连接失败")
}
