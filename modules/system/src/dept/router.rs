use super::handler;
use axum::{
    routing::{get, post, },
    Router,
};
use framework::state::AppState;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // 将静态路径放在前面
        .route("/system/dept/list", get(handler::list))
        .route("/system/dept/treeselect", get(handler::treeselect))
        // 专门处理“修改”时，获取排除子节点后的部门列表的请求
        .route("/system/dept/list/exclude/:deptId", get(handler::list_exclude_child))
        // POST 和 PUT 使用不同的路径，或者在同一个 .route() 中定义
        // RuoYi 的 API 设计就是 POST 和 PUT 都用 /system/dept
        .route("/system/dept", post(handler::add).put(handler::update))
        // 将 GET 和 DELETE 链接到同一个 .route() 定义上
        .route(
            "/system/dept/:deptId",
            get(handler::get_detail).delete(handler::delete),
        )

}