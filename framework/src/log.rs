use chrono::{Duration as ChronoDuration, Local};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::time;
use tracing::info;
use tracing_appender::rolling;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing() {
    // tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::INFO)
    //     .with_target(false)
    //     .init();

    // logs/ 目录下，每天生成一个新文件， order_system.2025-09-26.log
    let file_appender = rolling::daily("logs", "order_system.log");
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_writer)
        .with_ansi(false);

    let console_layer = fmt::layer().with_writer(std::io::stdout).pretty();

    // 默认 "info"，可通过环境变量 RUST_LOG=debug 来调整
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("Logger initialized. Starting application...");

    // 启动日志清理后台任务
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(24 * 60 * 60));
        // 初始化时清理一次，以防app重启后旧日志长时间不被清理
        cleanup_old_logs("logs", 30).await;
        loop {
            interval.tick().await;
            cleanup_old_logs("logs", 30).await;
        }
    });
}

async fn cleanup_old_logs(log_dir: &str, days_to_keep: i64) {
    let log_path = Path::new(log_dir);
    if !log_path.exists() {
        return;
    }

    let cutoff_date = (Local::now() - ChronoDuration::days(days_to_keep)).date_naive();
    info!(
        "[LogCleanup] Starting cleanup task. Deleting logs older than {}.",
        cutoff_date
    );

    match fs::read_dir(log_path) {
        Ok(entries) => {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    // 从文件名中尝试解析日期,假设文件名格式为 `order_system.log.YYYY-MM-DD`
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if let Some(date_str) = file_name.split('.').last() {
                            if let Ok(file_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                                if file_date < cutoff_date {
                                    info!("[LogCleanup] Deleting old log file: {:?}", path);
                                    if let Err(e) = fs::remove_file(&path) {
                                        tracing::error!("[LogCleanup] Failed to delete log file {:?}: {}", path, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!(
                "[LogCleanup] Failed to read log directory '{}': {}",
                log_dir,
                e
            );
        }
    }
}
