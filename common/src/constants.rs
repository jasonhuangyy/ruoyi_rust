/// 用于存放系统中所有缓存相关的常量
pub mod cache_keys {
    /// 字典数据的缓存键前缀
    /// 最终的键会是 "sys_dict_key:dict_type"，例如 "sys_dict_key:sys_user_sex"
    pub const SYS_DICT_KEY: &str = "sys_dict_key:";
}