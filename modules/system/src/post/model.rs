//! 岗位管理模块的实体与视图对象

use chrono::NaiveDateTime;
use entity::sys_post::ActiveModel as SysPostActiveModel;
use sea_orm::ActiveValue::{NotSet, Set};
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

impl Into<SysPostActiveModel> for AddPostVo {
    fn into(self) -> SysPostActiveModel {
        SysPostActiveModel {
            post_code: Set(self.post_code),
            post_name: Set(self.post_name),
            post_sort: Set(self.post_sort),
            status: Set(self.status),
            remark: Set(self.remark),
            post_id: NotSet,
            create_by: Set(None),
            create_time: Set(None),
            update_by: Set(None),
            update_time: Set(None),
        }
    }
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
