use super::handler;
use axum::handler::Handler;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use framework::state::AppState;
use monitor::operlog::extractor::BusinessType;
use monitor::operlog::layer::LogLayer;
use std::sync::Arc;

/// 字典管理路由
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // 字典类型路由
        .route("/system/dict/type/list", get(handler::list_types))
        .route("/system/dict/type/optionselect", get(handler::get_type_option_select))
        .route("/system/dict/type", post(handler::add_type))
        .route("/system/dict/type", put(handler::update_type))
        .route("/system/dict/type/refreshCache", delete(handler::refresh_cache))
        .route("/system/dict/type/:dictId", get(handler::get_type_detail).delete(handler::delete_type))

        // 字典数据路由
        .route("/system/dict/data/list", get(handler::list_data))
        .route("/system/dict/data/type/:dictType", get(handler::get_data_by_type))
        .route("/system/dict/data", post(handler::add_data))
        .route("/system/dict/data", put(handler::update_data))
        .route("/system/dict/data/:dictCode", get(handler::get_data_detail).delete(handler::delete_data))
        .route(
            "/system/dict/type/export",
            post(handler::export_types.layer(LogLayer::new("字典类型", BusinessType::Export)))
        )
}