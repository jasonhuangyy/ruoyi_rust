use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize,Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}
#[derive(Debug, Deserialize,Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize,Clone)]
pub struct JwtSettings {
    pub secret: String,
    pub issuer: String,
    pub expiration_hours: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub captcha_enabled: bool,
}

#[derive(Debug, Deserialize,Clone)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtSettings,
    pub security: SecurityConfig,
}

impl Settings {
    // 这里返回的错误类型是 config::ConfigError
    pub fn new() -> Result<Self, config::ConfigError> {
        let s = Config::builder()
            // 从 `config/default.toml` 开始
            .add_source(File::with_name("config/default"))
            // 从 `config/local.toml` 添加，这是可选的
            .add_source(File::with_name("config/local").required(false))
            // 从 `APP_` 开头的环境变量添加
            .add_source(Environment::with_prefix("APP"))
            .build()?;

        // 将所有配置反序列化到 Settings 结构体
        s.try_deserialize()
    }
}



