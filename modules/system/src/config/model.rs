use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// 数据库实体 (与 `sys_config` 表完全对应)
/// 使用 `sqlx::FromRow` 以便从数据库查询结果自动映射。
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SysConfig {
    pub config_id: i32,
    pub config_name: Option<String>,
    pub config_key: Option<String>,
    pub config_value: Option<String>,
    pub config_type: Option<String>,
    pub create_by: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_by: Option<String>,
    pub update_time: Option<NaiveDateTime>,
    pub remark: Option<String>,
}

/// 列表分页查询参数 DTO (Data Transfer Object)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConfigQuery {
    pub page_num: Option<i64>,
    pub page_size: Option<i64>,
    pub config_name: Option<String>,
    pub config_key: Option<String>,
    pub config_type: Option<String>,
    // RuoYi 使用 `params[beginTime]` 和 `params[endTime]` 格式，
    // 通过内嵌一个结构体并使用 `#[serde(rename = "params")]` 来完美匹配。
    #[serde(rename = "params")]
    pub date_range: Option<DateRange>,
}

/// 日期范围查询参数，作为 `ListConfigQuery` 的一部分。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateRange {
    pub begin_time: Option<String>,
    pub end_time: Option<String>,
}

/// 新增参数配置的请求体 VO (View Object)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddConfigVo {
    pub config_name: String,
    pub config_key: String,
    pub config_value: String,
    pub config_type: String,
    pub remark: Option<String>,
}

/// 修改参数配置的请求体 VO
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfigVo {
    pub config_id: i32, // 修改时必须携带主键
    pub config_name: String,
    pub config_key: String,
    pub config_value: String,
    pub config_type: String,
    pub remark: Option<String>,
}
