use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::time::Duration;
use sqlx::Executor;
use tracing::info;

pub async fn create_db_pool(database_url: &str) -> Result<MySqlPool, sqlx::Error> {
    info!("正在创建数据库连接池...");

    let pool = MySqlPoolOptions::new()
        .max_connections(10) 
        .min_connections(2) 
        .idle_timeout(Duration::from_secs(8))
        .after_connect(|conn, _| Box::pin(async move {
            info!("为新数据库连接设置时区为 '+8:00'");
            conn.execute("SET time_zone = '+8:00';").await?;
            Ok(())
        }))
        .connect(database_url) 
        .await?;

    info!("✅ 数据库连接池创建成功。");
    Ok(pool)
}