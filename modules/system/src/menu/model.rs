use entity::{prelude::SysMenuModel, sys_menu::ActiveModel as SysMenuActiveModel};
use sea_orm::ActiveValue::{NotSet, Set};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;

//处理JSON中的字符串 "0" 或数字 0，并将其统一转换为 i32
fn deserialize_string_or_number_to_i32<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::String(s) => s.parse::<i32>().map_err(de::Error::custom),
        Value::Number(num) => num
            .as_i64()
            .map(|n| n as i32)
            .ok_or_else(|| de::Error::custom(format!("无效的数字 {}", num))),
        _ => Err(de::Error::custom("期望一个字符串或数字")),
    }
}

// 系统菜单实体，严格根据 CREATE TABLE 语句定义
// #[derive(sqlx::FromRow, Debug,Default, Serialize, Deserialize, Clone)]
// #[serde(rename_all = "camelCase")]
// pub struct SysMenu {
//     pub menu_id: i64,
//     pub menu_name: String, // menu_name 是 NOT NULL，但为了映射安全，也可以是 Option，然后在业务层处理
//     pub parent_id: Option<i64>,
//     pub order_num: Option<i32>,
//     pub path: Option<String>,
//     //
//     pub component: Option<String>,
//     pub query: Option<String>,
//     pub route_name: Option<String>,
//     pub is_frame: Option<i32>,
//     pub is_cache: Option<i32>,
//     pub menu_type: Option<String>,
//     pub visible: Option<String>,
//     pub status: Option<String>,
//     pub perms: Option<String>,
//     pub icon: Option<String>,
//     pub create_by: Option<String>,
//     pub create_time: Option<NaiveDateTime>,
//     pub update_by: Option<String>,
//     pub update_time: Option<NaiveDateTime>,
//     pub remark: Option<String>,
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MenuTreeVo {
    #[serde(flatten)]
    pub menu: SysMenuModel,
    pub children: Vec<MenuTreeVo>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddMenuVo {
    pub parent_id: Option<i64>, // 前端可能不传 parent_id (对于根节点) // 新增时 parentId 是必须的
    pub menu_name: String,
    pub order_num: Option<i32>,
    pub path: Option<String>,
    pub component: Option<String>,
    #[serde(deserialize_with = "deserialize_string_or_number_to_i32")]
    pub is_frame: i32, // 前端传的是数字 0 或 1
    #[serde(deserialize_with = "deserialize_string_or_number_to_i32")]
    pub is_cache: i32,
    pub menu_type: String,
    pub visible: String,
    pub status: String,
    pub perms: Option<String>,
    pub icon: Option<String>,
    pub remark: Option<String>,
}

impl Into<SysMenuActiveModel> for AddMenuVo {
    fn into(self) -> SysMenuActiveModel {
        SysMenuActiveModel {
            menu_name: Set(self.menu_name),
            parent_id: Set(self.parent_id),
            order_num: Set(self.order_num),
            path: Set(self.path),
            component: Set(self.component),
            is_frame: Set(self.is_frame == 1),
            is_cache: Set(self.is_cache == 1),
            menu_type: Set(Some(self.menu_type)),
            visible: Set(self.visible),
            status: Set(Some(self.status)),
            perms: Set(self.perms),
            icon: Set(self.icon),
            remark: Set(self.remark),
            menu_id: NotSet,
            query: Set(None),
            route_name: Set(None),
            create_by: Set(None),
            create_time: Set(None),
            update_by: Set(None),
            update_time: Set(None),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMenuVo {
    pub menu_id: i64,
    pub parent_id: Option<i64>,
    pub menu_name: String,
    pub order_num: Option<i32>,
    pub path: Option<String>,
    pub component: Option<String>,
    #[serde(deserialize_with = "deserialize_string_or_number_to_i32")]
    pub is_frame: i32,
    #[serde(deserialize_with = "deserialize_string_or_number_to_i32")]
    pub is_cache: i32,
    pub menu_type: String,
    pub visible: String,
    pub status: String,
    pub perms: Option<String>,
    pub icon: Option<String>,
    pub remark: Option<String>,
}

/// 用于菜单树形选择器 (Treeselect) 的视图对象
/// 这个结构是为了匹配 RuoYi 前端 el-tree 组件的默认 props: { value: 'id', label: 'label', children: 'children' }
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MenuTreeSelectVo {
    // 菜单ID，对应 el-tree 的 value
    pub id: i64,
    // 菜单名称，对应 el-tree 的 label
    pub label: String,
    // 子菜单列表
    pub children: Vec<MenuTreeSelectVo>,
}
