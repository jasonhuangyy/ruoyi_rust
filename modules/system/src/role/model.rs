// use chrono::NaiveDateTime;
use serde::Deserialize;

// /// 角色信息实体，与 `sys_role` 数据库表完全对应。
// /// `sqlx::FromRow` 使得它可以从数据库查询结果自动映射。
// /// `Serialize`, `Deserialize` 用于JSON序列化和反序列化。
// #[derive(sqlx::FromRow, Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")] // 确保JSON字段为驼峰命名，以匹配前端
// pub struct SysRole {
//     pub role_id: i64,
//     pub role_name: String,
//     pub role_key: String,
//     pub role_sort: i32,
//     pub data_scope: Option<String>,
//     pub menu_check_strictly: Option<i8>, // tinyint(1) 对应 i8
//     pub dept_check_strictly: Option<i8>,
//     pub status: String,
//     #[serde(skip_serializing)]
//     pub del_flag: Option<String>,
//     pub create_by: Option<String>,
//     pub create_time: Option<NaiveDateTime>,
//     pub update_by: Option<String>,
//     pub update_time: Option<NaiveDateTime>,
//     pub remark: Option<String>,
// }

/// 用于角色列表查询的参数结构体
/// `Deserialize` 使其能从URL的query string中反序列化
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListRoleQuery {
    // 业务查询参数
    pub role_name: Option<String>,
    pub role_key: Option<String>,
    pub status: Option<String>,
    // RuoYi特有的日期范围查询参数
    #[serde(rename = "params[beginTime]")]
    pub begin_time: Option<String>,
    #[serde(rename = "params[endTime]")]
    pub end_time: Option<String>,
    // 分页参数
    pub page_num: Option<i64>,
    pub page_size: Option<i64>,
}

/// 新增角色时接收前端数据的请求体 (DTO)
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddRoleVo {
    pub role_name: String,
    pub role_key: String,
    pub role_sort: i32,
    pub status: String,
    pub remark: Option<String>,
    // 新增角色时，可能同时会关联菜单
    pub menu_ids: Option<Vec<i64>>,
}

/// 修改角色时接收前端数据的请求体 (DTO)
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleVo {
    pub role_id: i64, // 修改时必须携带ID
    pub role_name: String,
    pub role_key: String,
    pub role_sort: i32,
    pub status: String,
    pub remark: Option<String>,
    // 修改角色时，也可能重新关联菜单
    pub menu_ids: Option<Vec<i64>>,
}

/// 修改角色状态时使用的请求体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChangeStatusVo {
    pub role_id: i64,
    pub status: String,
}
