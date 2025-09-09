use common::error::AppError;
use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::error;

pub struct JwtConfig {
    pub secret: String, // 密钥
    pub issuer: String, // 签发者
    pub expiration_hours: i64, // 过期时间（小时）
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomClaims {
    pub user_id: i64,
    pub user_name: String,
}

/// 应用层使用的完整声明结构，包含了标准字段和自定义字段。
/// 这个结构体不会直接参与 JWT 的序列化/反序列化。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClaimsData {
    pub user_id: i64,
    pub user_name: String,
    pub jti: String,
}

/// 封装 JWT 操作的结构体
pub struct JwtUtil {
    key: HS256Key, // 内部持有一个处理好的密钥对象
    config: JwtConfig,
}

impl JwtUtil {
    pub fn new(config: JwtConfig) -> Self {
        let key = HS256Key::from_bytes(config.secret.as_bytes());
        Self { key, config }
    }

    /// 签发一个新的 JWT
    /// # Arguments
    /// * `claims_data` - 包含所有需要信息的完整声明结构
    pub fn sign(&self, claims_data: ClaimsData) -> Result<String, AppError> {
        // 创建只包含自定义字段的 CustomClaims 实例
        let custom_claims = CustomClaims {
            user_id: claims_data.user_id,
            user_name: claims_data.user_name,
        };

        // 使用 with_custom_claims 存入自定义部分
        let mut claims = Claims::with_custom_claims(
            serde_json::to_value(custom_claims).map_err(|e| {
                error!("[JWT_SIGN] Failed to serialize CustomClaims to JSON: {}", e);
                AppError::JwtError
            })?,
            Duration::from_hours(self.config.expiration_hours as u64),
        )
            .with_issuer(&self.config.issuer);

        // 使用 with_jwt_id 单独设置 jti 这个标准声明
        claims = claims.with_jwt_id(&claims_data.jti);

        // 进行签名
        self.key.authenticate(claims).map_err(|e| {
            error!("[JWT_SIGN] Failed to sign/authenticate claims: {}", e);
            AppError::JwtError
        })
    }

    /// 验证一个 JWT 并返回携带的数据
    pub fn verify(&self, token_str: &str) -> Result<ClaimsData, AppError> {
        let options = VerificationOptions {
            allowed_issuers: Some(vec![self.config.issuer.clone()].into_iter().collect()),
            ..Default::default()
        };

        // 验证时，泛型参数使用纯粹的 CustomClaims
        match self.key.verify_token::<CustomClaims>(token_str, Some(options)) {
            Ok(jwt_claims) => {
                // 从 jwt_claims 的顶层字段获取标准声明 (jti)
                let jti = jwt_claims.jwt_id.ok_or_else(|| {
                    error!("[JWT_VERIFY] Token is valid, but 'jti' field is missing.");
                    AppError::TokenInvalid
                })?;

                // 从 jwt_claims.custom 获取自定义声明
                let custom = jwt_claims.custom;

                // 将它们组合成我们应用层需要的完整 ClaimsData 结构
                Ok(ClaimsData {
                    user_id: custom.user_id,
                    user_name: custom.user_name,
                    jti,
                })
            }
            Err(e) => {
                error!("[JWT_VERIFY] Token verification FAILED. Raw error: {:?}", e);
                Err(AppError::TokenInvalid)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_jwt_sign_and_verify_symmetric() {
        let test_config = JwtConfig {
            secret: "a-very-secure-and-long-secret-for-testing-purpose-again".to_string(),
            issuer: "test-issuer".to_string(),
            expiration_hours: 1,
        };
        let jwt_util = JwtUtil::new(test_config);

        // 准备完整的应用层数据
        let user_data = ClaimsData {
            user_id: 100,
            user_name: "testuser".to_string(),
            jti: Uuid::new_v4().to_string(),
        };

        // 签发 (传入完整的 ClaimsData)
        let token = jwt_util.sign(user_data.clone()).expect("Signing should succeed");
        println!("Generated Test Token: {}", token);

        // 验证 (得到完整的 ClaimsData)
        let verified_data = jwt_util.verify(&token).expect("Verification should succeed");

        // 断言所有字段都正确
        assert_eq!(user_data.user_id, verified_data.user_id);
        assert_eq!(user_data.user_name, verified_data.user_name);
        assert_eq!(user_data.jti, verified_data.jti); 
    }
}