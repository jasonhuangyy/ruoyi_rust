use crate::config::Settings;
use crate::jwt::JwtUtil;
use common::models::dict_model::SysDictData;
use common::models::online_model::SysUserOnline;
use moka::future::Cache;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

// #[derive(Clone)]
pub struct AppState {
    pub db_pool: MySqlPool,
    // 这个缓存将用于存储 <UUID, 验证码答案> 的键值对。Key是String类型的UUID，Value是String类型的答案
    pub captcha_cache: Cache<String, String>, 
    // 用于字典数据的缓存：<字典类型, 字典数据列表>
    pub dict_cache: Cache<String, Vec<SysDictData>>,
    pub jwt_util: Arc<JwtUtil>, // 使用 Arc 包装以便在多线程中安全共享 
    // 在线用户缓存。Key: jti (JWT ID, 作为 tokenId)，Value: SysUserOnline 结构体，包含用户详细信息和完整 token
    pub online_user_cache: Cache<String, SysUserOnline>,

    // Token 黑名单缓存。Key: 完整的 JWT 字符串，Value: () 空元组，仅用于存在性检查
    pub token_blacklist_cache: Cache<String, ()>,

    // 全局任务调度器实例
    pub job_scheduler: JobScheduler,
    // 用于存储 `sys_job.job_id` 到 `JobScheduler` 内部 `Uuid` 的映射。
    pub job_id_to_uuid_map: RwLock<HashMap<i64, Uuid>>,
    pub settings: Settings,
    pub config_cache: Cache<String, String>,
}