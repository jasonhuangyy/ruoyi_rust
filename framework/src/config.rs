// use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtSettings {
    pub secret: String,
    pub issuer: String,
    pub expiration_hours: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub captcha_enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: ServerConfig,
    pub jwt: JwtSettings,
    pub security: SecurityConfig,
}

impl Settings {
    // 这里返回的错误类型是 config::ConfigError
    pub fn new() -> Result<Self, config::ConfigError> {
        let server = envy::from_env::<ServerConfig>().expect("host和port端口没有配置");
        let jwt = envy::from_env::<JwtSettings>().expect("jwt环境变量未配置");
        let security = envy::from_env::<SecurityConfig>().expect("security环境变量未配置");
        Ok(Self {
            server,
            jwt,
            security,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let settings = Settings::new().unwrap();
        assert_eq!(settings.server.host, "127.0.0.1");
        assert_eq!(settings.server.port, 8080);
    }
}
