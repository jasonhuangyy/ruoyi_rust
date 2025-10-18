use chrono::NaiveDateTime;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// 定时任务调度实体，与 `sys_job` 数据库表完全对应。
#[derive(sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SysJob {
    pub job_id: i64,
    pub job_name: Option<String>,
    pub job_group: Option<String>,
    // 调用目标字符串，例如 "ryTask.ryNoParams" 或 "ryTask.ryParams('hello')"
    pub invoke_target: Option<String>,
    pub cron_expression: Option<String>,
    // 计划执行错误策略 (1=立即执行, 2=执行一次, 3=放弃执行)
    pub misfire_policy: Option<String>,
    // 是否并发执行 (0=允许, 1=禁止)
    pub concurrent: Option<String>,
    // 状态 (0=正常, 1=暂停)
    pub status: Option<String>,
    pub create_by: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_by: Option<String>,
    pub update_time: Option<NaiveDateTime>,
    pub remark: Option<String>,
}

/// 用于定时任务列表查询的参数结构体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListJobQuery {
    pub job_name: Option<String>,
    pub job_group: Option<String>,
    pub status: Option<String>,
    pub page_num: Option<i64>,
    pub page_size: Option<i64>,
}

// 这个函数现在能处理 JSON 值，无论是数字还是字符串，都将其转换为无引号的字符串。
fn deserialize_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    // 将输入反序列化为任意的 serde_json::Value
    let value = Value::deserialize(deserializer)?;

    match value {
        // 如果它是一个字符串，我们去掉首尾的引号 (如果有的话)
        Value::String(s) => Ok(s),
        // 如果它是一个数字，我们直接将其转换为字符串
        Value::Number(n) => Ok(n.to_string()),
        // 其他类型，我们视为错误
        _ => Err(D::Error::custom(format!(
            "expected string or number, found {}",
            value
        ))),
    }
}

// 一个返回默认 jobGroup 的函数
fn default_job_group() -> String {
    "DEFAULT".to_string()
}

/// 新增定时任务时接收前端数据的请求体 (VO/DTO)
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddJobVo {
    pub job_name: String,
    #[serde(default = "default_job_group")]
    pub job_group: String,
    pub invoke_target: String,
    pub cron_expression: String,

    #[serde(deserialize_with = "deserialize_to_string")]
    pub misfire_policy: String,

    #[serde(deserialize_with = "deserialize_to_string")]
    pub concurrent: String,

    #[serde(deserialize_with = "deserialize_to_string")]
    pub status: String,

    pub remark: Option<String>,
}

/// 修改定时任务时接收前端数据的请求体 (VO/DTO)
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobVo {
    pub job_id: i64,
    pub job_name: String,
    #[serde(default = "default_job_group")]
    pub job_group: String,
    pub invoke_target: String,
    pub cron_expression: String,

    #[serde(deserialize_with = "deserialize_to_string")]
    pub misfire_policy: String,

    #[serde(deserialize_with = "deserialize_to_string")]
    pub concurrent: String,

    #[serde(deserialize_with = "deserialize_to_string")]
    pub status: String,

    pub remark: Option<String>,
}

/// 修改任务状态时使用的请求体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChangeStatusVo {
    pub job_id: i64,
    pub status: String,
}
