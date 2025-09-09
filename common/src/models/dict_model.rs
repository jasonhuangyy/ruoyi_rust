use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// 字典数据实体，与 `sys_dict_data` 表完全对应
#[derive(sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SysDictData {
    pub dict_code: i64,
    pub dict_sort: Option<i32>,
    pub dict_label: Option<String>,
    pub dict_value: Option<String>,
    pub dict_type: Option<String>,
    pub css_class: Option<String>,
    pub list_class: Option<String>,
    pub is_default: Option<String>,
    pub status: Option<String>,
    pub create_by: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_by: Option<String>,
    pub update_time: Option<NaiveDateTime>,
    pub remark: Option<String>,
}