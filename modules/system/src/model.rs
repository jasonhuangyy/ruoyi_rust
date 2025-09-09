use serde::{Deserialize, Serialize};

/// 登录请求体，用于从前端接收JSON数据。
/// `Deserialize` trait 使得这个结构体可以从JSON格式反序列化而来。
#[derive(Deserialize, Debug)] 
pub struct LoginRequest { 
    pub username: String,
    pub password: String,
    // 验证码的答案
    pub code: String,
    // 验证码的唯一标识
    pub uuid: String,
}

/// 系统用户模型，对应数据库的 `sys_user` 表。
#[derive(sqlx::FromRow, Debug)]
pub struct SysUser {
    #[sqlx(rename = "user_id")] // 如果数据库字段名和结构体字段名不一致，可以使用 rename
    pub user_id: i64,
    #[sqlx(rename = "user_name")]
    pub user_name: String,
    #[sqlx(rename = "nick_name")]
    pub nick_name: String,
    pub password: Option<String>,
}

/// 验证码接口的业务数据
#[derive(Serialize)]
pub struct CaptchaVo {
    pub uuid: String,
    pub img: String,
}

/// 登录接口的业务数据
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginVo {
    // 确保这里的 JSON key 是 "token"
    #[serde(rename = "token")]
    pub token: String,
}

/// 用户信息接口的业务数据
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoVo {
    pub user: UserDetailVo,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// 用户详细信息 VO
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetailVo {
    pub user_id: i64,
    pub user_name: String,
    pub nick_name: String,
}


/// 路由显示信息 VO (模拟前端的 antd-pro-vue 的路由结构)
/// 或者说是 RuoYi Vue 的路由结构
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterVo {
    // RuoYi 前端的一个特殊设计，对于非外链的顶级菜单，name 通常是 'ParentView'
    // 如果 hidden: true, 则不会显示在侧边栏
    pub name: String,
    pub path: String,
    pub hidden: bool,
    // redirect: 'noRedirect' 表示顶级菜单点击后不重定向
    pub redirect: Option<String>,
    // 对应加载的 Vue 组件路径
    pub component: String,
    // alwaysShow: true 确保即使只有一个子菜单，父级也会显示
    pub always_show: Option<bool>,
    pub meta: MetaVo,
    // 子菜单
    pub children: Option<Vec<RouterVo>>,
}

/// 路由的 meta 信息 VO
#[derive(Serialize,Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetaVo {
    pub title: String,
    pub icon: String,
    pub no_cache: bool,
    // 外链地址
    pub link: Option<String>,
}