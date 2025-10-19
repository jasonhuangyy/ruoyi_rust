use super::model::UploadFileVo;
use chrono::Local;
use common::error::AppError;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, NotSet};
use std::path::Path;
use tokio::fs;
use tracing::{error, info};
use uuid::Uuid;

pub async fn save_file(db: &DatabaseConnection, username: &str, original_filename: &str, data: &[u8]) -> Result<UploadFileVo, AppError> {
    // 1. 生成年月目录和用户目录
    let yyyymm = Local::now().format("%Y%m").to_string();
    let user_dir = Path::new("uploads").join(&yyyymm).join(username);

    // 2. 异步创建目录
    fs::create_dir_all(&user_dir).await.map_err(|e| {
        // 在日志中记录原始、详细的错误
        error!("创建上传目录失败 '{}': {:?}", user_dir.display(), e);
        // 返回一个对用户友好的、业务上的错误
        AppError::ValidationFailed("服务器创建文件目录失败".to_string())
    })?;

    // 3. 生成唯一文件名
    let extension = Path::new(original_filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s))
        .unwrap_or_default();
    let new_filename = format!("{}{}", Uuid::new_v4(), extension);

    let file_path = user_dir.join(&new_filename);

    // 4. 异步写入文件
    fs::write(&file_path, data).await.map_err(|e| {
        error!("写入上传文件失败 '{}': {:?}", file_path.display(), e);
        AppError::ValidationFailed("服务器保存文件失败".to_string())
    })?;

    // 5. 构造数据库记录
    let stored_path = Path::new(&yyyymm)
        .join(username)
        .join(&new_filename)
        .to_str()
        .unwrap_or_default()
        .to_string();
    let stored_path_str = stored_path.replace('\\', "/");

    let file_url = format!("/uploads/{}", stored_path_str);
    let file_size = data.len() as i64;

    // 6. 插入数据库记录
    let result = entity::sys_upload_files::ActiveModel {
        file_id: NotSet,
        original_name: Set(original_filename.to_string()),
        stored_path: Set(stored_path.clone()),
        file_url: Set(file_url.clone()),
        file_size: Set(Some(file_size)),
        uploader_name: Set(Some(username.to_string())),
        file_status: Set("pending".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    // let result = sqlx::query!(
    //     r#"
    //     INSERT INTO sys_upload_files
    //     (original_name, stored_path, file_url, file_size, uploader_name, file_status)
    //     VALUES (?, ?, ?, ?, ?, 'pending')
    //     "#,
    //     original_filename,
    //     stored_path,
    //     file_url,
    //     file_size,
    //     username
    // )
    // .execute(db)
    // .await?;

    let new_file_id = result.file_id;
    info!(
        "Successfully uploaded file '{}' for user '{}', stored as '{}', file_id: {}",
        original_filename, username, stored_path, new_file_id
    );

    // 7. 返回给前端的VO
    Ok(UploadFileVo {
        file_id: new_file_id,
        file_name: original_filename.to_string(),
        url: file_url,
    })
}
