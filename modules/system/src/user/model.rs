use crate::dept::model::SysDept;
// 引入部门模型
use crate::post::model::SysPost;
use crate::role::model::SysRole;
// 引入角色模型
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
// 引入岗位模型

/// 用户信息实体，与 `sys_user` 数据库表完全对应。
#[derive(sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SysUser {
    pub user_id: i64,
    pub dept_id: Option<i64>,
    pub user_name: String,
    pub nick_name: String,
    pub user_type: Option<String>,
    pub email: Option<String>,
    pub phonenumber: Option<String>,
    pub sex: Option<String>,
    pub avatar: Option<String>,
    #[serde(skip_serializing)] // 密码字段永远不应该被序列化返回给前端
    pub password: Option<String>,
    pub status: Option<String>,
    #[serde(skip_serializing)]
    pub del_flag: Option<String>,
    pub login_ip: Option<String>,
    pub login_date: Option<NaiveDateTime>,
    pub pwd_update_date: Option<NaiveDateTime>,
    pub create_by: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_by: Option<String>,
    pub update_time: Option<NaiveDateTime>,
    pub remark: Option<String>,
}

/// 用户列表查询的参数结构体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListUserQuery {
    pub user_name: Option<String>,
    pub phonenumber: Option<String>,
    pub status: Option<String>,
    pub dept_id: Option<i64>,
    #[serde(rename = "params[beginTime]")]
    pub begin_time: Option<String>,
    #[serde(rename = "params[endTime]")]
    pub end_time: Option<String>,
    pub page_num: Option<u64>,
    pub page_size: Option<u64>,
}

/// 用户列表展示的视图对象  
/// 它结合了 `SysUser` 和 `SysDept` 的信息。
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserListVo {
    // 使用 `flatten` 将 SysUser 的所有字段“拍平”到这一层
    #[serde(flatten)]
    pub user: SysUser,
    // 关联的部门信息
    pub dept: Option<SysDept>,
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddUserInitVo {
    // 系统中所有可用的角色列表
    pub roles: Vec<SysRole>,
    pub posts: Vec<SysPost>,
}
/// 新增用户时接收前端数据的请求体  
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddUserVo {
    pub dept_id: Option<i64>,
    pub user_name: String,
    pub nick_name: String,
    pub password: Option<String>, // 新增时密码是必须的
    pub phonenumber: Option<String>,
    pub email: Option<String>,
    pub sex: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
    pub role_ids: Option<Vec<i64>>, // 关联的角色ID列表
    pub post_ids: Option<Vec<i64>>,
}

/// 修改用户时接收前端数据的请求体  
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserVo {
    pub user_id: i64, // 修改时必须携带用户ID
    pub dept_id: Option<i64>,
    pub nick_name: String,
    pub phonenumber: Option<String>,
    pub email: Option<String>,
    pub sex: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
    pub role_ids: Option<Vec<i64>>,
    pub post_ids: Option<Vec<i64>>,
}

/// 重置用户密码的请求体
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResetPwdVo {
    pub user_id: i64,
    pub password: String,
}

/// 修改用户状态的请求体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChangeStatusVo {
    pub user_id: i64,
    pub status: String,
}

/// 获取用户详情时返回给前端的完整数据结构
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserDetailVo {
    // 当前用户的数据
    pub data: Option<SysUser>,
    // 当前用户已关联的角色ID列表
    pub role_ids: Vec<i64>,
    // 系统中所有可用的角色列表
    pub roles: Vec<SysRole>,
    // 当前用户已关联的岗位ID列表
    pub post_ids: Vec<i64>,
    // 系统中所有可用的岗位列表
    pub posts: Vec<SysPost>,
}

/// 获取用户个人信息时返回给前端的视图对象  
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileVo {
    // 个人详细信息
    pub data: SysUser,
    // 所属角色组，格式为 "管理员,普通角色"
    pub role_group: String,
    // 所属岗位组，格式为 "董事长,项目经理"
    pub post_group: String,
}

/// 用户修改个人密码时接收前端数据的请求体  
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePwdVo {
    pub old_password: String,
    pub new_password: String,
}

/// 角色分配页面展示的视图对象  
/// 这个结构体是为了 `get_auth_role` 接口返回数据而设计的。
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthRoleVo {
    pub user: SysUser,
    pub roles: Vec<SysRole>,
}

/// 更新用户角色分配时接收的请求体  
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAuthRoleVo {
    pub user_id: i64,
    pub role_ids: String, // 前端传来的是以逗号分隔的字符串 "1,2,3"
}

/// 个人中心-用户详细资料-视图对象  
/// 包含了用户基本信息和其关联的部门信息
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileDetailVo {
    #[serde(flatten)]
    pub user: SysUser,
    pub dept: Option<SysDept>,
}

/// 个人中心-更新基本资料-请求体  
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileVo {
    pub nick_name: String,
    pub phonenumber: String,
    pub email: String,
    pub sex: String,
}

/// 个人中心-上传头像-成功响应体   
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAvatarVo {
    pub img_url: String,
    pub msg: String,
}
