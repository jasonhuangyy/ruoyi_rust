use axum::{
    async_trait,
    extract::{Extension, FromRequest, Request},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 业务操作类型枚举
/// 对应 `sys_oper_log` 表中的 `business_type` 字段
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BusinessType {
    Other,    // 0: 其他
    Insert,   // 1: 新增
    Update,   // 2: 修改
    Delete,   // 3: 删除
    Grant,    // 4: 授权
    Export,   // 5: 导出
    Import,   // 6: 导入
    Force,    // 7: 强退
    GenCode,  // 8: 生成代码
    Clean,    // 9: 清空数据
}
// 为 BusinessType 实现一个转换到 i32 的方法，方便存入数据库
impl From<BusinessType> for i32 {
    fn from(bt: BusinessType) -> Self {
        bt as i32
    }
}

#[derive(Clone, Debug)]
pub struct LogInfo {
    pub title: String,
    pub business_type: BusinessType,
}

// LogMarker 是 LogInfo 的一个包装器,这使得在处理器函数签名中写 `LogMarker(log_info)` 成为可能
#[derive(Debug, Clone)]
pub struct LogMarker(pub Arc<LogInfo>);
 

#[async_trait]
impl<S> FromRequest<S> for LogMarker
where
    S: Send + Sync,
{
    // 定义如果提取失败时返回的错误类型
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        // 期望在之前的中间件中，已经将 Arc<LogInfo> 放入了请求扩展。
        // 使用 axum::extract::Extension 来安全地提取它。
        let Extension(log_info) = req
            .extensions()
            .get::<Arc<LogInfo>>()
            .cloned()
            .map(Extension)
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "LogInfo not found in request extensions. Is the LogLayer missing?",
            ))?;

        // 如果成功提取，就用它来构造 LogMarker
        Ok(LogMarker(log_info))
    }
}