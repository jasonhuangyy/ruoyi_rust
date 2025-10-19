use chrono::NaiveDateTime;
use entity::prelude::SysDeptModel;
use entity::sys_dept::ActiveModel;
use sea_orm::ActiveValue::{NotSet, Set};
use serde::{Deserialize, Serialize};

/// 部门实体，与数据库 `sys_dept` 表的 Schema 完全对应
/// `sqlx::FromRow` 用于从数据库行自动映射
/// `Serialize`, `Deserialize` 用于 API 的 JSON 交互
#[derive(sqlx::FromRow, Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SysDept {
    pub dept_id: i64,
    // 以下字段根据 CREATE TABLE 语句的可空性(NULL)来决定是否使用 Option<T>
    pub parent_id: Option<i64>,
    pub ancestors: Option<String>,
    pub dept_name: Option<String>,
    pub order_num: Option<i32>,
    pub leader: Option<String>, // leader 在表中可为 NULL
    pub phone: Option<String>,  // phone 在表中可为 NULL
    pub email: Option<String>,  // email 在表中可为 NULL
    pub status: Option<String>,
    // del_flag, create_by, update_by 通常由后端逻辑控制，不一定需要暴露给所有API
    pub del_flag: Option<String>,
    pub create_by: Option<String>,
    pub create_time: Option<NaiveDateTime>, // create_time 在表中可为 NULL
    pub update_by: Option<String>,
    pub update_time: Option<NaiveDateTime>, // update_time 在表中可为 NULL
}

/// 用于构建部门树的视图对象 (VO)
/// 在 SysDept 的基础上增加了 `children` 字段，用于递归展示
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeptTreeVo {
    // 使用 `flatten` 属性，将 SysDept 的所有字段“拍平”到这一层
    #[serde(flatten)]
    pub dept: SysDeptModel,
    // 子部门列表
    pub children: Vec<DeptTreeVo>,
}

/// 用于接收前端“新增部门”请求的数据体
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddDeptVo {
    pub parent_id: i64,
    pub dept_name: String,
    pub order_num: i32,
    pub leader: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: String,
}

impl Into<ActiveModel> for AddDeptVo {
    fn into(self) -> ActiveModel {
        ActiveModel {
            dept_id: NotSet,
            parent_id: Set(Some(self.parent_id)),
            ancestors: Set(None),
            dept_name: Set(Some(self.dept_name)),
            order_num: Set(Some(self.order_num)),
            leader: Set(self.leader),
            phone: Set(self.phone),
            email: Set(self.email),
            status: Set(Some(self.status)),
            del_flag: Set(Some("0".to_string())),
            create_by: Set(None),
            create_time: NotSet,
            update_by: Set(None),
            update_time: NotSet,
        }
    }
}

/// 用于接收前端“修改部门”请求的数据体
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeptVo {
    pub dept_id: i64, // 修改时必须提供 ID
    // 其他字段与新增时相同
    pub parent_id: i64,
    pub dept_name: String,
    pub order_num: i32,
    pub leader: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: String,
}

/// 用于接收前端列表查询的参数
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListDeptQuery {
    pub dept_name: Option<String>,
    pub status: Option<String>,
}

/// 用于部门树形选择器 (Treeselect) 的视图对象
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeptTreeSelectVo {
    // 部门ID
    pub id: i64,
    // 部门名称
    pub label: String,
    // 子部门列表
    pub children: Vec<DeptTreeSelectVo>,
}
