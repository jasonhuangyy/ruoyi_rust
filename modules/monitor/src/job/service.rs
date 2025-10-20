use super::model::{AddJobVo, ChangeStatusVo, ListJobQuery, UpdateJobVo};
use crate::job::task_drop_pending_file::cleanup_expired_pending_files;
use crate::logininfor;
use common::{error::AppError, page::TableDataInfo};
use entity::prelude::*;
use framework::state::AppState;
use rust_xlsxwriter::Workbook;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::IntoActiveModel;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sqlx::Postgres;
use sqlx::QueryBuilder;
use std::str::ParseBoolError;
use std::sync::Arc;
use tokio_cron_scheduler::Job;
use tracing::{debug, error, info, instrument, warn};

/// 查询定时任务列表（分页）
pub async fn select_job_list(conn: &DatabaseConnection, params: ListJobQuery) -> Result<TableDataInfo<SysJobModel>, AppError> {
    info!(
        "[SERVICE] Entering job::select_job_list with params: {:?}",
        params
    );

    let db = conn.get_postgres_connection_pool();

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_job WHERE 1=1");
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM sys_job WHERE 1=1");

    if let Some(name) = params.job_name {
        if !name.trim().is_empty() {
            let condition = format!("%{}%", name);
            query_builder
                .push(" AND job_name LIKE ")
                .push_bind(condition.clone());
            count_builder
                .push(" AND job_name LIKE ")
                .push_bind(condition);
        }
    }
    if let Some(group) = params.job_group {
        if !group.trim().is_empty() {
            query_builder
                .push(" AND job_group = ")
                .push_bind(group.clone());
            count_builder.push(" AND job_group = ").push_bind(group);
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            query_builder
                .push(" AND status = ")
                .push_bind(status.clone());
            count_builder.push(" AND status = ").push_bind(status);
        }
    }

    let total_row: (i64,) = count_builder.build_query_as().fetch_one(db).await?;
    let total = total_row.0;

    let page_num = params.page_num.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page_num - 1) * page_size;

    query_builder
        .push(" ORDER BY job_id DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);

    let rows: Vec<SysJobModel> = query_builder.build_query_as().fetch_all(db).await?;
    info!(
        "[DB_RESULT] Found {} jobs for the current page.",
        rows.len()
    );
    Ok(TableDataInfo::new(rows, total))
}

/// 根据ID查询任务详情
pub async fn select_job_by_id(conn: &DatabaseConnection, job_id: i64) -> Result<SysJobModel, AppError> {
    info!(
        "[SERVICE] Entering job::select_job_by_id for id: {}",
        job_id
    );
    // let db = conn.get_postgres_connection_pool();
    // sqlx::query_as!(SysJob, "SELECT * FROM sys_job WHERE job_id = ?", job_id)
    //     .fetch_one(db)
    //     .await
    //     .map_err(AppError::from)
    let dto: Option<SysJobModel> = SysJob::find()
        .filter(SysJobColumn::JobId.eq(job_id))
        .one(conn)
        .await
        .map_err(AppError::from)?;
    dto.ok_or_else(|| {
        let msg = format!("任务ID: {} 不存在", job_id);
        error!("{}", msg);
        AppError::RecordNotFound
    })
}

// ======================= 调度器交互服务 =======================
async fn add_job_to_scheduler(job: &SysJobModel, state: &Arc<AppState>) -> Result<(), AppError> {
    info!(
        "[SCHEDULER] Attempting to add job_id '{}' to scheduler.",
        job.job_id
    );

    let cron_expr = job.cron_expression.as_deref().ok_or_else(|| {
        let msg = format!(
            "任务 '{}' (ID: {}) 缺少Cron表达式",
            job.job_name, job.job_id
        );
        error!("{}", msg);
        AppError::ValidationFailed(msg)
    })?;
    let invoke_target = job.invoke_target.clone();

    let state_clone = state.clone();

    let job_task = Job::new_async(cron_expr, move |uuid, mut l| {
        let app_state = state_clone.clone();
        let target = invoke_target.clone();
        Box::pin(async move {
            info!(
                "[SCHEDULER_EVENT] Job with UUID {:?} triggered. Target: {}",
                uuid, target
            );
            execute_task(app_state, target).await;
            let next_tick = l.next_tick_for_job(uuid).await;
            match next_tick {
                Ok(Some(ts)) => info!(
                    "[SCHEDULER_EVENT] Next tick for job {:?} is at {:?}",
                    uuid, ts
                ),
                _ => warn!(
                    "[SCHEDULER_EVENT] Could not get next tick for job {:?}",
                    uuid
                ),
            }
        })
    })
    .map_err(|e| {
        let msg = format!("创建调度任务失败 (Cron: '{}'): {}", cron_expr, e);
        error!("{}", msg);
        AppError::JobSchedulerError(msg)
    })?;

    let job_uuid = state.job_scheduler.add(job_task).await.map_err(|e| {
        let msg = format!("添加任务到调度器失败: {}", e);
        error!("{}", msg);
        AppError::JobSchedulerError(msg)
    })?;

    let mut map_guard = state.job_id_to_uuid_map.write().await;
    map_guard.insert(job.job_id, job_uuid);

    info!(
        "[SCHEDULER] Successfully added job_id '{}' to scheduler with UUID '{}'",
        job.job_id, job_uuid
    );
    Ok(())
}

async fn remove_job_from_scheduler(job_id: i64, state: &Arc<AppState>) -> Result<(), AppError> {
    info!(
        "[SCHEDULER] Attempting to remove job_id '{}' from scheduler.",
        job_id
    );

    let mut map_guard = state.job_id_to_uuid_map.write().await;

    if let Some(uuid) = map_guard.get(&job_id).cloned() {
        if let Err(e) = state.job_scheduler.remove(&uuid).await {
            let msg = format!("从调度器移除任务失败 (UUID: {}): {}", uuid, e);
            error!("{}", msg);
            return Err(AppError::JobSchedulerError(msg));
        }
        map_guard.remove(&job_id);
        info!(
            "[SCHEDULER] Successfully removed job_id '{}' (UUID: {}) from scheduler.",
            job_id, uuid
        );
    } else {
        warn!(
            "[SCHEDULER] Job_id '{}' not found in scheduler map, nothing to remove.",
            job_id
        );
    }
    Ok(())
}

/// 新增任务（DB + Scheduler）
pub async fn add_job(state: Arc<AppState>, vo: AddJobVo) -> Result<(), AppError> {
    info!("[SERVICE] Entering job::add_job with vo: {:?}", vo);
    let db = state.db.get_postgres_connection_pool();
    let tx = db.begin().await?;

    // let result = sqlx::query!(
    //     "INSERT INTO sys_job (job_name, job_group, invoke_target, cron_expression, misfire_policy, concurrent, status, remark, create_by, create_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'admin', NOW())",
    //     vo.job_name,
    //     vo.job_group,
    //     vo.invoke_target,
    //     vo.cron_expression,
    //     vo.misfire_policy,
    //     vo.concurrent,
    //     vo.status,
    //     vo.remark
    // )
    // .execute(&mut *tx)
    // .await?;
    let model: SysJobModel = vo.into();
    let mut active_model = model.into_active_model();
    active_model.job_id = NotSet;
    active_model.create_time = NotSet;
    active_model.update_time = NotSet;
    let new_job: SysJobModel = active_model.insert(&state.db).await?;

    tx.commit().await?;

    if new_job.status.as_deref() == Some("0") {
        add_job_to_scheduler(&new_job, &state).await?;
    }

    info!(
        "[SERVICE] Successfully added job_id '{}' to DB and scheduler.",
        new_job.job_id
    );
    Ok(())
}

/// 修改任务（DB + Scheduler）
pub async fn update_job(state: Arc<AppState>, vo: UpdateJobVo) -> Result<(), AppError> {
    info!("[SERVICE] Entering job::update_job with vo: {:?}", vo);

    remove_job_from_scheduler(vo.job_id, &state).await?;

    let exist: SysJobModel = select_job_by_id(&state.db, vo.job_id).await?;
    let mut model = exist.into_active_model();
    model.job_name = Set(vo.job_name);
    model.job_group = Set(vo.job_group);
    model.invoke_target = Set(vo.invoke_target);
    model.cron_expression = Set(vo.cron_expression);
    model.misfire_policy = Set(Some(vo.misfire_policy));
    model.concurrent = Set(Some(vo.concurrent));
    model.status = Set(Some(vo.status));
    model.remark = Set(vo.remark);
    model.update_by = Set(Some("admin".to_string()));
    model.update_time = NotSet;

    // let pool = state.db.get_postgres_connection_pool();

    // sqlx::query!(
    //     "UPDATE sys_job SET job_name=?, job_group=?, invoke_target=?, cron_expression=?, misfire_policy=?, concurrent=?, status=?, remark=?, update_by='admin', update_time=NOW() WHERE job_id=?",
    //     vo.job_name,
    //     vo.job_group,
    //     vo.invoke_target,
    //     vo.cron_expression,
    //     vo.misfire_policy,
    //     vo.concurrent,
    //     vo.status,
    //     vo.remark,
    //     vo.job_id
    // )
    // .execute(pool)
    // .await?;
    let updated_job = model.update(&state.db).await?;

    // let updated_job = select_job_by_id(&state.db, vo.job_id).await?;

    if updated_job.status.as_deref() == Some("0") {
        add_job_to_scheduler(&updated_job, &state).await?;
    }

    info!(
        "[SERVICE] Successfully updated job_id '{}' in DB and scheduler.",
        vo.job_id
    );
    Ok(())
}

/// 删除任务（DB + Scheduler）
pub async fn delete_job_by_ids(state: Arc<AppState>, job_ids: &[i64]) -> Result<(), AppError> {
    info!(
        "[SERVICE] Entering job::delete_job_by_ids with ids: {:?}",
        job_ids
    );
    let pool = state.db.get_postgres_connection_pool();
    let mut tx = pool.begin().await?;
    let params = job_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM sys_job WHERE job_id IN ({})", params);

    let mut query = sqlx::query(&sql);
    for id in job_ids {
        query = query.bind(id);
    }
    query.execute(&mut *tx).await?;

    tx.commit().await?;

    for id in job_ids {
        if let Err(e) = remove_job_from_scheduler(*id, &state).await {
            warn!(
                "[SERVICE] Failed to remove job_id {} from scheduler, but it was deleted from DB. Error: {:?}",
                id, e
            );
        }
    }
    Ok(())
}

/// 改变任务状态（DB + Scheduler）
pub async fn change_job_status(state: Arc<AppState>, vo: ChangeStatusVo) -> Result<(), AppError> {
    info!(
        "[SERVICE] Entering job::change_job_status with vo: {:?}",
        vo
    );

    sqlx::query("UPDATE sys_job SET status = ? WHERE job_id = ?")
        .bind(vo.status.clone())
        .bind(vo.job_id.clone())
        .execute(state.db.get_postgres_connection_pool())
        .await?;

    let job = select_job_by_id(&state.db, vo.job_id).await?;

    if vo.status == "1" {
        remove_job_from_scheduler(vo.job_id, &state).await?;
    } else if vo.status == "0" {
        let _ = remove_job_from_scheduler(vo.job_id, &state).await;
        add_job_to_scheduler(&job, &state).await?;
    }
    Ok(())
}

/// 立即执行一次任务
pub async fn run_job_once(state: Arc<AppState>, job_id: i64) -> Result<(), AppError> {
    info!("[SERVICE] Entering job::run_job_once for id: {}", job_id);
    let job = select_job_by_id(&state.db, job_id).await?;

    let invoke_target = job.invoke_target;
    let state_clone = state.clone();
    tokio::spawn(async move {
        execute_task(state_clone, invoke_target).await;
    });
    Ok(())
}

/// 应用启动时，初始化所有定时任务
pub async fn init_scheduler(state: Arc<AppState>) -> Result<(), AppError> {
    info!("[INIT] Initializing job scheduler...");
    // let jobs: Vec<SysJob> = sqlx::query_as("SELECT * FROM sys_job WHERE status = '0'")
    //     .fetch_all(state.db.get_postgres_connection_pool())
    //     .await?;

    let jobs: Vec<SysJobModel> = SysJob::find()
        .filter(SysJobColumn::Status.eq("0"))
        .all(&state.db)
        .await?;

    info!(
        "[INIT] Found {} enabled jobs in database to schedule.",
        jobs.len()
    );
    for job in jobs {
        if let Err(e) = add_job_to_scheduler(&job, &state).await {
            error!("[INIT] Failed to schedule job_id {}: {:?}", job.job_id, e);
        }
    }

    if let Err(e) = state.job_scheduler.start().await {
        let msg = format!("启动主定时任务调度器失败: {}", e);
        error!("{}", msg);
        return Err(AppError::JobSchedulerError(msg));
    }

    info!("✅ Job scheduler started successfully.");
    Ok(())
}

// 将每个具体的任务逻辑拆分成独立的 async 函数，增加可读性和可维护性。

/// 模拟 RuoYi 的无参任务
#[instrument]
async fn ry_no_params() {
    info!("--- RuoYi-Rust task [ry_no_params] is running ---");
    // 模拟一些耗时操作
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    info!("--- Task [ry_no_params] finished ---");
}

/// 模拟 RuoYi 的带单个字符串参数的任务
#[instrument(skip(param))] // param 可能会很大，跳过在日志中完整记录
async fn ry_params(param: &str) {
    info!(
        "--- RuoYi-Rust task [ry_params] is running with param: '{}' ---",
        param
    );
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    info!("--- Task [ry_params] finished ---");
}

/// 模拟 RuoYi 的带多个参数的任务
#[instrument]
async fn ry_multiple_params(s: &str, b: bool, l: i64, d: f64, i: i32) {
    info!("--- RuoYi-Rust task [ry_multiple_params] is running with params: ---");
    info!("  String: '{}'", s);
    info!("  Boolean: {}", b);
    info!("  Long: {}", l);
    info!("  Double: {}", d);
    info!("  Integer: {}", i);
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    info!("--- Task [ry_multiple_params] finished ---");
}

/// 任务执行的主入口和分发器 ,取代了 Java 中的反射调用逻辑。
#[instrument(skip(state))]
async fn execute_task(state: Arc<AppState>, invoke_target: String) {
    debug!("Attempting to parse and execute task: '{}'", invoke_target);

    // --- 简易的调用目标解析器 ---
    // 查找第一个 '(' 和最后一个 ')' 的位置
    let method_name_end = invoke_target.find('(');
    let params_end = invoke_target.rfind(')');

    if let (Some(method_end), Some(params_end)) = (method_name_end, params_end) {
        // 提取方法名，例如 "ryTask.ryParams"
        let method_name = &invoke_target[..method_end];
        // 提取参数字符串，例如 "'hello', true, 123, 4.56, 789"
        let params_str = &invoke_target[method_end + 1..params_end];

        // 使用 match 进行静态分发
        match method_name {
            "ryTask.ryParams" => {
                // 参数是单个字符串，去除首尾的单引号
                let param = params_str.trim().trim_matches('\'');
                ry_params(param).await;
            }
            "ryTask.ryMultipleParams" => {
                // 解析多个参数
                let parts: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                if parts.len() == 5 {
                    // 为第一个 Ok 添加明确的类型注解，使其 Err 类型与其他 Result 兼容
                    if let (Ok(s), Ok(b), Ok(l), Ok(d), Ok(i)) = (
                        // 明确告诉编译器这是一个 Result<&str, std::num::ParseBoolError>
                        // 我们选择 ParseBoolError 只是为了让元组中类型统一，任何一个 parse 返回的 Err 类型都可以
                        Ok(parts[0].trim_matches('\'')) as Result<&str, ParseBoolError>,
                        parts[1].parse::<bool>(),
                        parts[2].parse::<i64>(),
                        parts[3].parse::<f64>(),
                        parts[4].parse::<i32>(),
                    ) {
                        ry_multiple_params(s, b, l, d, i).await;
                    } else {
                        error!(
                            "Failed to parse arguments for ryMultipleParams: {:?}",
                            parts
                        );
                    }
                } else {
                    error!(
                        "Incorrect number of arguments for ryMultipleParams. Expected 5, got {}.",
                        parts.len()
                    );
                }
            }
            //在数据库 sys_job 表中添加任务记录:
            // 最后，只需要在数据库中 sys_job 表手动插入一条记录，或者通过已经实现的定时任务管理界面来添加：
            // 任务名称: 清理临时上传文件
            // 任务组: SYSTEM
            // 调用目标字符串: fileTask.cleanupPendingFiles
            // Cron表达式: 0 0 2 * * ? (每天凌晨2点执行)
            // 状态: 0 (正常)
            "ryTask.cleanPendingFiles" => {
                info!("--- Task [cleanPendingFiles] started ---");
                match cleanup_expired_pending_files(state.db.get_postgres_connection_pool()).await {
                    Ok(count) => info!("Successfully cleaned up {} pending files.", count),
                    Err(e) => error!("Failed to clean up pending files: {:?}", e),
                }
                info!("--- Task [cleanPendingFiles] finished ---");
            }
            _ => {
                error!("Unknown method with parameters: '{}'", method_name);
            }
        }
    } else {
        // 处理无参数的方法
        match invoke_target.as_str() {
            "ryTask.ryNoParams" => {
                ry_no_params().await;
            }
            // 在这里可以添加更多无参数的任务
            // 例如，可以创建一个清理日志的任务
            "ryTask.cleanLoginLog" => {
                info!("--- Task [cleanLoginLog] started ---");
                match logininfor::service::clean_logininfor(&state.db).await {
                    Ok(affected) => info!("Successfully cleaned {} login logs.", affected),
                    Err(e) => error!("Failed to clean login logs: {:?}", e),
                }
                info!("--- Task [cleanLoginLog] finished ---");
            }
            _ => {
                error!("Unknown or unsupported invoke_target: '{}'", invoke_target);
            }
        }
    }
}

#[instrument(skip(db, params))]
pub async fn export_job_list(db: &DatabaseConnection, params: ListJobQuery) -> Result<Vec<u8>, AppError> {
    info!(
        "[SERVICE] Starting job list export with params: {:?}",
        params
    );
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_job WHERE 1=1");

    if let Some(name) = params.job_name {
        if !name.trim().is_empty() {
            query_builder
                .push(" AND job_name LIKE ")
                .push_bind(format!("%{}%", name));
        }
    }
    if let Some(group) = params.job_group {
        if !group.trim().is_empty() {
            query_builder.push(" AND job_group = ").push_bind(group);
        }
    }
    if let Some(status) = params.status {
        if !status.trim().is_empty() {
            query_builder.push(" AND status = ").push_bind(status);
        }
    }
    query_builder.push(" ORDER BY job_id DESC");

    let jobs: Vec<SysJobModel> = query_builder
        .build_query_as()
        .fetch_all(db.get_postgres_connection_pool())
        .await?;
    info!("[DB_RESULT] Fetched {} jobs for export.", jobs.len());

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let headers = [
        "任务ID",
        "任务名称",
        "任务组名",
        "调用目标字符串",
        "cron执行表达式",
        "状态",
    ];
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }
    for (row_num, job) in jobs.iter().enumerate() {
        let row = (row_num + 1) as u32;

        let status_str = if job.status.as_deref() == Some("0") {
            "正常"
        } else {
            "暂停"
        };
        let group_str = match job.job_group.as_str() {
            "DEFAULT" => "默认",
            "SYSTEM" => "系统",
            _ => "未知",
        };

        worksheet.write(row, 0, job.job_id)?;
        worksheet.write(row, 1, job.job_name.as_str())?;
        worksheet.write(row, 2, group_str)?;
        worksheet.write(row, 3, job.invoke_target.as_str())?;
        worksheet.write(row, 4, job.cron_expression.as_deref().unwrap_or(""))?;
        worksheet.write(row, 5, status_str)?;
    }
    let buffer = workbook.save_to_buffer()?;
    info!(
        "[SERVICE] Excel buffer created successfully, size: {} bytes.",
        buffer.len()
    );

    Ok(buffer)
}
