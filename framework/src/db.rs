use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::time::Duration;
use tracing::info;

pub async fn create_db_pool(database_url: &str) -> Result<MySqlPool, sqlx::Error> {
    info!("正在创建数据库连接池...");

    let pool = MySqlPoolOptions::new()
        .max_connections(10) // 设置最大连接数
        .min_connections(2)  // 设置最小连接数
        .idle_timeout(Duration::from_secs(8)) // 空闲连接超时时间
        .connect(database_url) // 直接使用 .connect() 方法
        .await?;

    info!("✅ 数据库连接池创建成功。");
    Ok(pool)
}