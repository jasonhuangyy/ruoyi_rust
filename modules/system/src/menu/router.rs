use crate::menu;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use framework::state::AppState;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    let menu_router = Router::new()
        .route("/system/menu/list", get(menu::handler::list))
        .route("/system/menu/treeselect", get(menu::handler::treeselect))
        .route("/system/menu", post(menu::handler::add))
        .route("/system/menu", put(menu::handler::update))
        .route("/system/menu/:menuId", get(menu::handler::get_detail))
        .route("/system/menu/roleMenuTreeselect/:roleId", get(menu::handler::role_menu_treeselect))
        .route("/system/menu/:menuId", delete(menu::handler::delete));
    
    menu_router
}
