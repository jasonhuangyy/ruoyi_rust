use common::models::online_model::SysUserOnline;
use entity::prelude::SysDictDataModel;
use moka::future::Cache;
use std::time::Duration;
use tracing::info;
pub struct AppCache {
    pub captcha_cache: Cache<String, String>,
    pub dict_cache: Cache<String, Vec<SysDictDataModel>>,
    pub online_user_cache: Cache<String, SysUserOnline>,
    pub token_blacklist_cache: Cache<String, ()>,
    pub config_cache: Cache<String, String>,
}

impl AppCache {
    /// 初始化缓存,创建一个新的 moka 缓存实例
    /// .time_to_live() 设置了每个条目的存活时间，这里是5分钟，和RuoYi原版保持一致
    /// .max_capacity() 设置了缓存的最大容量，防止内存无限增长
    pub fn new() -> Self {
        // 1. 初始化验证码缓存
        let captcha_cache = Cache::builder()
            .name("captcha_cache") // 给缓存起个名字，方便调试
            .time_to_live(Duration::from_secs(5 * 60)) // 5分钟过期
            .max_capacity(1000)
            .build();

        // 初始化字典数据缓存,字典数据通常是长久有效的，可以设置一个较长的过期时间，或者不过期
        // RuoYi 是通过手动刷新来更新的，所以可以不设置 TTL
        let dict_cache = Cache::builder()
            .name("dict_cache")
            .max_capacity(500) // 假设系统中有不超过500种字典类型
            .build();

        // 初始化在线用户缓存,TTL 设置为2小时，与JWT的过期时间保持一致
        let online_user_cache: Cache<String, SysUserOnline> = Cache::builder()
            .name("online_user_cache")
            .time_to_live(Duration::from_secs(2 * 60 * 60)) // 2小时过期
            .max_capacity(1000) // 假设最多支持1000个在线用户
            .build();

        // 初始化Token黑名单缓存,TTL可以设置得比JWT过期时间稍长，确保覆盖
        let token_blacklist_cache: Cache<String, ()> = Cache::builder()
            .name("token_blacklist_cache")
            .time_to_live(Duration::from_secs(2 * 60 * 60 + 5 * 60)) // 2小时5分钟过期
            .max_capacity(2000) // 容量可以稍大一些
            .build();

        // 创建并初始化 config_cache 实例, 参数配置通常不常变动，但读取频繁，非常适合缓存。  设置一个较长的缓存时间，例如1小时。
        let config_cache = Cache::builder()
            .name("config_cache")
            .max_capacity(100) // RuoYi 中参数不多，100个容量足够
            .time_to_live(Duration::from_secs(60 * 60)) // 缓存1小时
            .build();
        info!("Config cache created.");

        Self {
            captcha_cache,
            dict_cache,
            online_user_cache,
            token_blacklist_cache,
            config_cache,
        }
    }
}
