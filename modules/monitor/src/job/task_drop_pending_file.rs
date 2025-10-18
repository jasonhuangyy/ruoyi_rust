use chrono::{Duration, Utc};
use common::error::AppError;
use sqlx::{PgPool, QueryBuilder};
use tracing::{error, info, instrument, warn};

////////////////////////////////////////////
/// 清理过期的待定(pending)文件
///
/// 此函数会查找并删除所有状态为 'pending' 且上传时间超过24小时的文件记录及其对应的物理文件。
/// 这是一个独立的、可被定时任务调用的服务。
#[instrument(skip(db))]
pub async fn cleanup_expired_pending_files(db: &PgPool) -> Result<u64, AppError> {
    info!("[FILE_CLEANUP_SERVICE] Starting cleanup of expired pending files...");

    // 计算出24小时前的时间点
    let expiration_time = Utc::now().naive_utc() - Duration::hours(24);

    // 在一个事务中执行，确保数据一致性
    let mut tx = db.begin().await?;

    // 查找所有符合条件的待清理文件 (ID 和 存储路径)
    let files_to_delete: Vec<(i64, String)> = sqlx::query_as("SELECT file_id, stored_path FROM sys_upload_files WHERE file_status = 'pending' AND upload_time < ?")
        .bind(expiration_time)
        .fetch_all(&mut *tx) // 在事务中查询
        .await?;

    if files_to_delete.is_empty() {
        info!("[FILE_CLEANUP_SERVICE] No expired pending files found. Exiting.");
        tx.commit().await?; // 即使没有文件要删，也要提交事务以释放连接
        return Ok(0);
    }

    let file_ids: Vec<i64> = files_to_delete.iter().map(|(id, _)| *id).collect();
    info!(
        "[FILE_CLEANUP_SERVICE] Found {} files to clean up. File IDs: {:?}",
        files_to_delete.len(),
        file_ids
    );

    // 删除物理文件
    for (file_id, stored_path) in &files_to_delete {
        let physical_path = std::path::Path::new("uploads").join(stored_path);
        if let Err(e) = tokio::fs::remove_file(&physical_path).await {
            if e.kind() == std::io::ErrorKind::NotFound {
                warn!(
                    "[FILE_CLEANUP_SERVICE] Physical file not found for pending file_id: {}, path: {:?}. It may have been manually deleted. Proceeding to delete DB record.",
                    file_id, physical_path
                );
            } else {
                error!(
                    "[FILE_CLEANUP_SERVICE] Failed to delete physical file for pending file_id: {}: {}. Rolling back transaction.",
                    file_id, e
                );
                let errmsg = AppError::ValidationFailed(format!("文件删除出错,{}", e.to_string()));
                tx.rollback().await?;
                return Err(errmsg);
            }
        }
    }

    // 从 sys_upload_files 表中批量删除记录
    let mut delete_query = QueryBuilder::new("DELETE FROM sys_upload_files WHERE file_id IN (");
    let mut separated = delete_query.separated(",");
    for id in &file_ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(")");

    let result = delete_query.build().execute(&mut *tx).await?;
    info!(
        "[FILE_CLEANUP_SERVICE] Deleted {} records from sys_upload_files.",
        result.rows_affected()
    );

    // 提交事务
    tx.commit().await?;
    info!("[FILE_CLEANUP_SERVICE] Cleanup transaction committed successfully.");

    Ok(result.rows_affected())
}
