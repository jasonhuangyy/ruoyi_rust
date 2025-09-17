use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SysJobLog {
    pub job_log_id: i64,
    pub job_name: String,
    pub job_group: String,
    pub invoke_target: String,
    pub job_message: Option<String>,
    pub status: Option<String>,
    pub exception_info: Option<String>,
    pub create_time: Option<NaiveDateTime>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListJobLogQuery {
    pub job_name: Option<String>,
    pub job_group: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "params[beginTime]")]
    pub begin_time: Option<String>,
    #[serde(rename = "params[endTime]")]
    pub end_time: Option<String>,
    pub page_num: Option<u64>,
    pub page_size: Option<u64>,
}