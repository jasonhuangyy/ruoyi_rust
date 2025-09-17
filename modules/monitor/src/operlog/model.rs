use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// 操作日志记录实体，与 `sys_oper_log` 数据库表完全对应。
#[derive(sqlx::FromRow, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SysOperLog {
    pub oper_id: i64,
    pub title: Option<String>,
    // 业务类型在数据库中是 int，这里用 i32
    pub business_type: Option<i32>,
    pub method: Option<String>,
    pub request_method: Option<String>,
    pub operator_type: Option<i32>,
    pub oper_name: Option<String>,
    pub dept_name: Option<String>,
    pub oper_url: Option<String>,
    pub oper_ip: Option<String>,
    pub oper_location: Option<String>,
    pub oper_param: Option<String>,
    pub json_result: Option<String>,
    // 操作状态在数据库中是 int，这里用 i32
    pub status: Option<i32>,
    pub error_msg: Option<String>,
    pub oper_time: Option<NaiveDateTime>,
    // cost_time 在数据库中是 bigint，这里用 i64
    pub cost_time: Option<i64>,
}

/// 用于操作日志列表查询的参数结构体
#[derive(Deserialize, Debug)]
pub struct ListOperLogQuery {
    // 业务查询参数
    #[serde(rename = "title")]
    pub title: Option<String>,
    #[serde(rename = "operName")]
    pub oper_name: Option<String>,
    #[serde(rename = "businessType")]
    pub business_type: Option<i32>,
    #[serde(rename = "status")]
    pub status: Option<i32>,
    #[serde(rename = "params[beginTime]")]
    pub begin_time: Option<String>,
    #[serde(rename = "params[endTime]")]
    pub end_time: Option<String>,
    #[serde(rename = "pageNum")]
    pub page_num: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
}