use crate::cache::AppCache;
use crate::config::Settings;
use crate::jwt::JwtUtil;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

// #[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub cache: AppCache,
    pub jwt_util: Arc<JwtUtil>, // 使用 Arc 包装以便在多线程中安全共享
    // 全局任务调度器实例
    pub job_scheduler: JobScheduler,
    // 用于存储 `sys_job.job_id` 到 `JobScheduler` 内部 `Uuid` 的映射。
    pub job_id_to_uuid_map: RwLock<HashMap<i64, Uuid>>,
    pub settings: Settings,
}
