use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// 字典类型实体，与 `sys_dict_type` 表完全对应
#[derive(sqlx::FromRow, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SysDictType {
    pub dict_id: i64,
    // 所有在数据库中可能为 NULL 或有 DEFAULT 值的字段，都定义为 Option<T>
    pub dict_name: Option<String>,
    pub dict_type: Option<String>,
    pub status: Option<String>,
    pub create_by: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_by: Option<String>,
    pub update_time: Option<NaiveDateTime>,
    pub remark: Option<String>,
}

/// 用于接收“新增字典类型”请求的数据体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddDictTypeVo {
    pub dict_name: String,
    pub dict_type: String,
    pub status: String,
    pub remark: Option<String>,
}

/// 用于接收“修改字典类型”请求的数据体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDictTypeVo {
    pub dict_id: i64, // 修改时必须提供主键
    pub dict_name: String,
    pub dict_type: String,
    pub status: String,
    pub remark: Option<String>,
}

/// 用于接收字典类型列表查询的参数
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListDictTypeQuery {
    // 业务查询参数
    pub dict_name: Option<String>,
    pub dict_type: Option<String>,
    pub status: Option<String>,

    // 这两个字段现在是可选的，如果前端不传，给一个默认值
    pub page_num: Option<i64>,
    pub page_size: Option<i64>,
}

/// 用于接收“新增字典数据”请求的数据体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddDictDataVo {
    pub dict_sort: i32,
    pub dict_label: String,
    pub dict_value: String,
    pub dict_type: String,
    pub css_class: Option<String>,
    pub list_class: Option<String>,
    pub is_default: String,
    pub status: String,
    pub remark: Option<String>,
}

/// 用于接收“修改字典数据”请求的数据体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDictDataVo {
    pub dict_code: i64, // 修改时必须提供主键
    pub dict_sort: i32,
    pub dict_label: String,
    pub dict_value: String,
    pub dict_type: String,
    pub css_class: Option<String>,
    pub list_class: Option<String>,
    pub is_default: String,
    pub status: String,
    pub remark: Option<String>,
}

/// 用于接收字典数据列表查询的参数 (修正版)
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListDictDataQuery {
    // 业务查询参数
    pub dict_type: String, // 查询字典数据时，必须提供字典类型
    pub dict_label: Option<String>,
    pub status: Option<String>,

    pub page_num: Option<i64>,
    pub page_size: Option<i64>,
}

/// 用于字典类型下拉框选项的视图对象
/// 只包含前端需要的 id 和 name
#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictTypeOptionVo {
    pub dict_id: i64,
    pub dict_name: Option<String>,
}
