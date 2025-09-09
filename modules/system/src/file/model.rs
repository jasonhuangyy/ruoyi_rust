use chrono::NaiveDateTime;
use serde::Serialize;

/// 数据库实体 (与 `sys_upload_files` 表完全对应)
#[derive(sqlx::FromRow, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SysUploadFile {
    pub file_id: i64,
    pub original_name: String,
    pub stored_path: String,
    pub file_url: String,
    pub file_size: Option<i64>,
    pub file_status: String,
    pub related_order_id: Option<i64>,
    pub uploader_name: Option<String>,
    pub upload_time: Option<NaiveDateTime>,
    pub remark: Option<String>,
}

/// 文件上传成功后返回给前端的视图对象 (VO) ，在 "data" 字段中返回的视图对象
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadFileVo {
    pub file_id: i64,
    pub file_name: String,
    pub url: String,
}

// 专门用于文件上传接口的、符合标准嵌套结构的响应体
#[derive(Serialize)]
pub struct FileUploadResponse {
    pub code: u16,
    pub msg: String,
    pub data: UploadFileVo, // data 字段直接就是 UploadFileVo
}