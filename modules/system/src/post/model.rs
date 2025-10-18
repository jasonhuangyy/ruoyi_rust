//! 岗位管理模块的实体与视图对象

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// 数据库实体 (与 `sys_post` 表完全对应)
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SysPost {
    pub post_id: i64,
    pub post_code: String,
    pub post_name: String,
    pub post_sort: i32,
    pub status: String,
    pub create_by: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_by: Option<String>,
    pub update_time: Option<NaiveDateTime>,
    pub remark: Option<String>,
}

/// 列表分页查询参数 DTO
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPostQuery {
    pub page_num: Option<i64>,
    pub page_size: Option<i64>,
    #[serde(rename = "postCode")]
    pub post_code: Option<String>,
    pub post_name: Option<String>,
    pub status: Option<String>,
}

/// 新增岗位的请求体 VO
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPostVo {
    pub post_code: String,
    pub post_name: String,
    pub post_sort: i32,
    pub status: String,
    pub remark: Option<String>,
}

/// 修改岗位的请求体 VO
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePostVo {
    pub post_id: i64, // 修改时必须携带主键
    pub post_code: String,
    pub post_name: String,
    pub post_sort: i32,
    pub status: String,
    pub remark: Option<String>,
}

/// 用于用户管理等页面下拉框选项的视图对象
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PostOptionVo {
    pub post_id: i64,
    pub post_name: String,
}
