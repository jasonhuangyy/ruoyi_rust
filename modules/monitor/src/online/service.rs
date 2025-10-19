use common::error::AppError;
use common::models::online_model::SysUserOnline;
use framework::state::AppState;
use moka::future::Cache;
use std::sync::Arc;
use tracing::{info, warn};

/// 从缓存中获取所有在线用户列表
///
/// # Arguments
/// * `cache` - 在线用户缓存的引用
///
/// # Returns
/// 一个包含所有 SysUserOnline 对象的 Vec
pub async fn list_online_users(cache: &Cache<String, SysUserOnline>) -> Vec<SysUserOnline> {
    info!("[SERVICE] Attempting to list all online users from cache.");
    // moka 的 .iter() 方法返回一个迭代器，我们可以用它来收集所有的值
    // .values() 直接获取所有值的集合，然后克隆出来
    let users: Vec<SysUserOnline> = cache.iter().map(|(_key, value)| value.clone()).collect();

    info!("[SERVICE] Found {} online users in cache.", users.len());
    users
}

/// 强制用户下线（强退）
///
/// # Arguments
/// * `state` - 完整的应用共享状态
/// * `token_id` - 要被踢下线的用户的 tokenId (即 jti)
///
/// # Returns
/// 成功时返回 Ok(()), 失败时返回 AppError
pub async fn force_logout(state: &Arc<AppState>, token_id: &str) -> Result<(), AppError> {
    info!(
        "[SERVICE] Attempting to force logout for tokenId: {}",
        token_id
    );

    // 从在线用户缓存中获取用户信息,使用 .remove() 方法，它会原子性地获取并删除条目。
    match state.cache.online_user_cache.remove(token_id).await {
        Some(user_online) => {
            // 如果找到了用户，将其完整的 JWT 添加到黑名单缓存
            info!(
                "[SERVICE] Found user '{}' for tokenId '{}'. Adding token to blacklist.",
                user_online.user_name, token_id
            );
            state
                .cache
                .token_blacklist_cache
                .insert(user_online.token, ())
                .await;

            info!(
                "[SERVICE] Successfully processed force logout for tokenId: {}.",
                token_id
            );
            Ok(())
        }
        None => {
            // 如果在在线缓存中没找到，可能用户已过期或已登出。这通常不是一个错误，但我们应该记录下来。
            warn!(
                "[SERVICE] Could not find user with tokenId '{}' in online cache. Maybe already logged out or expired.",
                token_id
            );
            // 即使没找到，也返回成功，因为目标（用户下线）已经达成。
            Ok(())
        }
    }
}
