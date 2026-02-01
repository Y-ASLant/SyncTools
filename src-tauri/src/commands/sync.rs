use crate::core::comparator::FileComparator;
use crate::core::scanner::FileScanner;
use crate::core::SyncEngine;
use crate::db::SyncJob;
use crate::AppState;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// 差异分析结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub path: String,
    pub size: u64,
    pub reverse: bool,
    pub source_exists: bool,
    pub dest_exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub source_name: String,
    pub dest_name: String,
    pub source_files: usize,
    pub dest_files: usize,
    pub actions: Vec<DiffAction>,
    pub copy_count: usize,
    pub delete_count: usize,
    pub skip_count: usize,
    pub conflict_count: usize,
    pub total_bytes: u64,
    /// 源缓存时间（Unix时间戳，0表示未使用缓存）
    pub source_cached_at: u64,
    /// 目标缓存时间（Unix时间戳，0表示未使用缓存）
    pub dest_cached_at: u64,
}

/// 分析同步任务（不执行同步，只返回差异）
#[tauri::command]
pub async fn analyze_job(
    job_id: String,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<DiffResult, String> {
    let force_refresh = force_refresh.unwrap_or(false);
    // 创建取消标志
    let cancel_flag = Arc::new(AtomicBool::new(false));
    state
        .analyze_cancels
        .lock()
        .await
        .insert(job_id.clone(), cancel_flag.clone());

    // 在函数结束时清理取消标志
    let cleanup_state = state.analyze_cancels.clone();
    let cleanup_job_id = job_id.clone();
    scopeguard::defer! {
        tokio::spawn(async move {
            cleanup_state.lock().await.remove(&cleanup_job_id);
        });
    }

    let job = SyncJob::load(&state.db, &job_id)
        .await
        .map_err(|e| format!("加载任务失败: {}", e))?
        .ok_or_else(|| "任务不存在".to_string())?;

    // 检查是否已取消
    if cancel_flag.load(Ordering::Relaxed) {
        return Err("操作已取消".to_string());
    }

    // 创建存储
    let source_storage = crate::storage::create_storage(&job.sourceConfig)
        .await
        .map_err(|e| format!("源存储连接失败: {}", e))?;
    let dest_storage = crate::storage::create_storage(&job.destConfig)
        .await
        .map_err(|e| format!("目标存储连接失败: {}", e))?;

    // 检查是否已取消
    if cancel_flag.load(Ordering::Relaxed) {
        return Err("操作已取消".to_string());
    }

    // 初始化缓存（只对远程存储使用缓存），缓存目录跟随数据存储目录
    let cache_dir = state.config_dir.join("cache");
    
    // 从配置读取缓存 TTL，本地存储不使用缓存
    let cache_config = crate::config::CacheConfig::load(&state.config_dir);
    let source_is_local = matches!(job.sourceConfig.typ, crate::db::StorageType::Local);
    let dest_is_local = matches!(job.destConfig.typ, crate::db::StorageType::Local);
    let source_ttl = if source_is_local { 0 } else { cache_config.remote_ttl };
    let dest_ttl = if dest_is_local { 0 } else { cache_config.remote_ttl };
    
    let source_cache = crate::core::FileListCache::new(cache_dir.clone()).with_ttl(source_ttl);
    let dest_cache = crate::core::FileListCache::new(cache_dir).with_ttl(dest_ttl);
    
    let source_config_json = serde_json::to_string(&job.sourceConfig).unwrap_or_default();
    let dest_config_json = serde_json::to_string(&job.destConfig).unwrap_or_default();

    // 如果强制刷新，先清除缓存
    if force_refresh {
        source_cache.clear(&job_id);
    }

    // 扫描源存储（支持缓存）
    let scanner = FileScanner::with_cancel(cancel_flag.clone());
    let mut source_cached_at: u64 = 0;
    let source_tree = if !force_refresh {
        if let Some(cached) = source_cache.load(&job_id, "source", &source_config_json) {
            source_cached_at = cached.cached_at;
            cached.files
        } else {
            let tree = scanner
                .scan_storage(source_storage.as_ref(), None)
                .await
                .map_err(|e| {
                    if cancel_flag.load(Ordering::Relaxed) {
                        "操作已取消".to_string()
                    } else {
                        format!("扫描源存储失败: {}", e)
                    }
                })?;
            let _ = source_cache.save(&job_id, "source", &source_config_json, &tree);
            tree
        }
    } else {
        let tree = scanner
            .scan_storage(source_storage.as_ref(), None)
            .await
            .map_err(|e| {
                if cancel_flag.load(Ordering::Relaxed) {
                    "操作已取消".to_string()
                } else {
                    format!("扫描源存储失败: {}", e)
                }
            })?;
        let _ = source_cache.save(&job_id, "source", &source_config_json, &tree);
        tree
    };

    // 检查是否已取消
    if cancel_flag.load(Ordering::Relaxed) {
        return Err("操作已取消".to_string());
    }

    // 扫描目标存储（支持缓存）
    let mut dest_cached_at: u64 = 0;
    let dest_tree = if !force_refresh {
        if let Some(cached) = dest_cache.load(&job_id, "dest", &dest_config_json) {
            dest_cached_at = cached.cached_at;
            cached.files
        } else {
            let tree = scanner
                .scan_storage(dest_storage.as_ref(), None)
                .await
                .map_err(|e| {
                    if cancel_flag.load(Ordering::Relaxed) {
                        "操作已取消".to_string()
                    } else {
                        format!("扫描目标存储失败: {}", e)
                    }
                })?;
            let _ = dest_cache.save(&job_id, "dest", &dest_config_json, &tree);
            tree
        }
    } else {
        let tree = scanner
            .scan_storage(dest_storage.as_ref(), None)
            .await
            .map_err(|e| {
                if cancel_flag.load(Ordering::Relaxed) {
                    "操作已取消".to_string()
                } else {
                    format!("扫描目标存储失败: {}", e)
                }
            })?;
        let _ = dest_cache.save(&job_id, "dest", &dest_config_json, &tree);
        tree
    };

    // 比较文件
    let comparator = FileComparator::default();
    let actions = comparator.compare_trees(&source_tree, &dest_tree, &job.syncMode);
    let summary = FileComparator::summarize_actions(&actions);

    // 转换为前端需要的格式
    let diff_actions: Vec<DiffAction> = actions
        .iter()
        .map(|action| match action {
            crate::core::comparator::SyncAction::Copy {
                source_path,
                size,
                reverse,
                ..
            } => DiffAction {
                action_type: "copy".to_string(),
                path: source_path.clone(),
                size: *size,
                reverse: *reverse,
                source_exists: !*reverse || source_tree.contains_key(source_path),
                dest_exists: *reverse || dest_tree.contains_key(source_path),
            },
            crate::core::comparator::SyncAction::Delete { path, from_dest } => DiffAction {
                action_type: "delete".to_string(),
                path: path.clone(),
                size: dest_tree.get(path).map(|f| f.size).unwrap_or(0),
                reverse: false,
                source_exists: !*from_dest,
                dest_exists: *from_dest,
            },
            crate::core::comparator::SyncAction::Skip { path } => DiffAction {
                action_type: "skip".to_string(),
                path: path.clone(),
                size: source_tree.get(path).map(|f| f.size).unwrap_or(0),
                reverse: false,
                source_exists: true,
                dest_exists: true,
            },
            crate::core::comparator::SyncAction::Conflict { path, .. } => DiffAction {
                action_type: "conflict".to_string(),
                path: path.clone(),
                size: source_tree.get(path).map(|f| f.size).unwrap_or(0),
                reverse: false,
                source_exists: source_tree.contains_key(path),
                dest_exists: dest_tree.contains_key(path),
            },
        })
        .collect();

    Ok(DiffResult {
        source_name: source_storage.name().to_string(),
        dest_name: dest_storage.name().to_string(),
        source_files: source_tree.len(),
        dest_files: dest_tree.len(),
        actions: diff_actions,
        copy_count: summary.copy_count + summary.reverse_copy_count,
        delete_count: summary.delete_count,
        skip_count: summary.skip_count,
        conflict_count: summary.conflict_count,
        total_bytes: summary.total_transfer_bytes(),
        source_cached_at,
        dest_cached_at,
    })
}

/// 开始同步任务
#[tauri::command]
pub async fn start_sync(
    job_id: String,
    auto_create_dir: Option<bool>,
    max_concurrent: Option<usize>,
    conflict_resolutions: Option<std::collections::HashMap<String, String>>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let auto_create = auto_create_dir.unwrap_or(true);
    let concurrent = max_concurrent.unwrap_or(4).clamp(1, 128); // 限制在 1-128 之间
    let resolutions = conflict_resolutions.unwrap_or_default();
    // 从数据库加载任务
    let job = SyncJob::load(&state.db, &job_id)
        .await
        .map_err(|e| format!("加载任务失败: {}", e))?
        .ok_or_else(|| "任务不存在".to_string())?;

    // 检查任务是否已禁用
    if !job.enabled {
        return Err("任务已禁用".to_string());
    }

    // 创建进度通道
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<crate::db::SyncProgress>(100);

    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();

    // 保存取消信号
    state
        .cancel_signals
        .lock()
        .await
        .insert(job_id.clone(), cancel_tx);

    // 启动进度监听任务
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = app_clone.emit("sync-progress", &progress);
        }
    });

    // 在后台执行同步
    let db_clone = state.db.clone();
    let job_id_for_emit = job_id.clone();
    let job_for_sync = job.clone();
    let app_for_emit = app.clone();
    let cancel_signals = state.cancel_signals.clone();
    let cache_dir = state.config_dir.join("cache");
    let cache_config = crate::config::CacheConfig::load(&state.config_dir);

    let resolutions_for_sync = resolutions.clone();
    tokio::spawn(async move {
        let config = crate::core::SyncConfig {
            auto_create_dir: auto_create,
            max_concurrent_transfers: concurrent,
            conflict_resolutions: resolutions_for_sync,
            cache_dir: Some(cache_dir),
            remote_cache_ttl: cache_config.remote_ttl,
            ..Default::default()
        };
        
        tracing::debug!("同步配置: 并行数={}, 自动创建目录={}, 冲突解决方案数={}", 
            concurrent, auto_create, config.conflict_resolutions.len());
        
        let engine = Arc::new(SyncEngine::with_config(db_clone, config));
        let engine_for_cancel = engine.clone();

        // 监听取消信号
        let cancel_handle = tokio::spawn(async move {
            let _ = cancel_rx.await;
            engine_for_cancel.cancel();
        });

        let result = engine.run_sync(&job_for_sync, Some(progress_tx)).await;

        // 取消取消监听
        cancel_handle.abort();

        // 从取消信号中移除
        cancel_signals.lock().await.remove(&job_id_for_emit);

        // 发送完成事件
        let _ = app_for_emit.emit(
            "sync-complete",
            serde_json::json!({
                "job_id": job_id_for_emit,
                "result": result.as_ref()
                    .map(|r| serde_json::to_value(r).ok())
                    .map_err(|e| e.to_string()),
            }),
        );
    });

    Ok(job_id)
}

/// 取消同步任务
#[tauri::command]
pub async fn cancel_sync(job_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut signals = state.cancel_signals.lock().await;
    if let Some(sender) = signals.remove(&job_id) {
        let _ = sender.send(());
        Ok(())
    } else {
        Err("没有正在运行的同步任务".to_string())
    }
}

/// 取消分析任务
#[tauri::command]
pub async fn cancel_analyze(job_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let cancels = state.analyze_cancels.lock().await;
    if let Some(flag) = cancels.get(&job_id) {
        flag.store(true, Ordering::Relaxed);
        Ok(())
    } else {
        // 没有正在运行的分析任务也返回成功，前端可能已经取消
        Ok(())
    }
}

/// 获取未完成的传输状态
#[tauri::command]
pub async fn get_pending_transfers(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PendingTransfer>, String> {
    use crate::core::TransferManager;

    let manager = TransferManager::new(state.db.clone());
    let transfers = manager
        .get_pending_transfers(&job_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(transfers
        .into_iter()
        .map(|t| PendingTransfer {
            id: t.id,
            file_path: t.file_path,
            total_size: t.total_size,
            transferred_size: t.transferred_size,
            status: format!("{}", t.status),
        })
        .collect())
}

/// 未完成传输信息
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingTransfer {
    pub id: String,
    pub file_path: String,
    pub total_size: u64,
    pub transferred_size: u64,
    pub status: String,
}

/// 恢复同步任务（从断点继续）
#[tauri::command]
pub async fn resume_sync(
    job_id: String,
    auto_create_dir: Option<bool>,
    max_concurrent: Option<usize>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    // 检查是否有未完成的传输
    use crate::core::TransferManager;
    let manager = TransferManager::new(state.db.clone());
    let pending = manager
        .get_pending_transfers(&job_id)
        .await
        .map_err(|e| e.to_string())?;

    if pending.is_empty() {
        // 没有未完成的传输，执行正常同步
        return start_sync(job_id, auto_create_dir, max_concurrent, None, state, app).await;
    }

    tracing::debug!(
        "恢复同步任务: {}, 有 {} 个未完成的传输",
        job_id,
        pending.len()
    );

    // 重新开始同步（会自动跳过已完成的文件）
    start_sync(job_id, auto_create_dir, max_concurrent, None, state, app).await
}

/// 同步历史记录条目
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncHistoryEntry {
    pub id: i64,
    pub job_id: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub status: String,
    pub files_scanned: i64,
    pub files_copied: i64,
    pub files_deleted: Option<i64>,
    pub files_skipped: Option<i64>,
    pub files_failed: Option<i64>,
    pub bytes_transferred: i64,
    pub error_message: Option<String>,
}

/// 同步日志数据库行
#[derive(Debug, Clone, sqlx::FromRow)]
struct SyncLogRow {
    pub id: i64,
    pub job_id: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub status: String,
    pub files_scanned: i64,
    pub files_copied: i64,
    pub files_deleted: Option<i64>,
    pub bytes_transferred: i64,
    pub error_message: Option<String>,
}

/// 获取同步历史记录
#[tauri::command]
pub async fn get_sync_history(
    job_id: String,
    limit: i64,
    state: State<'_, AppState>,
) -> Result<Vec<SyncHistoryEntry>, String> {
    let logs = sqlx::query_as::<_, SyncLogRow>(
        "SELECT id, job_id, start_time, end_time, status, files_scanned, files_copied, files_deleted, bytes_transferred, error_message
         FROM sync_logs
         WHERE job_id = ?
         ORDER BY start_time DESC
         LIMIT ?"
    )
    .bind(&job_id)
    .bind(limit)
    .fetch_all(&*state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(logs
        .into_iter()
        .map(|log| SyncHistoryEntry {
            id: log.id,
            job_id: log.job_id,
            start_time: log.start_time,
            end_time: log.end_time,
            status: log.status,
            files_scanned: log.files_scanned,
            files_copied: log.files_copied,
            files_deleted: log.files_deleted,
            files_skipped: None, // 未在数据库中存储
            files_failed: None,  // 未在数据库中存储
            bytes_transferred: log.bytes_transferred,
            error_message: log.error_message,
        })
        .collect())
}

/// 清除任务的扫描缓存
#[tauri::command]
pub async fn clear_scan_cache(
    job_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cache_dir = state.config_dir.join("cache");
    let cache = crate::core::FileListCache::new(cache_dir);
    
    match job_id {
        Some(id) => {
            cache.clear(&id);
            tracing::info!("已清除任务 {} 的扫描缓存", id);
        }
        None => {
            cache.clear_all();
            tracing::info!("已清除所有扫描缓存");
        }
    }
    
    Ok(())
}
