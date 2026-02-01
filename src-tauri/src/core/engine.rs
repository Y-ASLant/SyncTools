#![allow(non_snake_case)]

use crate::core::cache::FileListCache;
use crate::core::comparator::{ActionSummary, FileComparator, SyncAction};
use crate::core::file_state::{calculate_quick_hash, FileState, FileStateManager};
use crate::core::scanner::{FileScanner, ScanConfig};
use crate::db::{SyncJob, SyncProgress, SyncStatus};
use crate::storage::Storage;
use anyhow::Result;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tracing::{debug, error, info, warn};

/// 同步配置
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// 最大并发传输数
    pub max_concurrent_transfers: usize,
    /// 大文件阈值（字节），超过此大小的文件使用分块传输
    pub large_file_threshold: u64,
    /// 分块大小（字节）
    pub chunk_size: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试基础延迟（毫秒）
    pub retry_base_delay_ms: u64,
    /// 是否启用断点续传
    pub enable_resume: bool,
    /// 扫描配置
    pub scan_config: ScanConfig,
    /// 是否自动创建目标目录
    pub auto_create_dir: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_concurrent_transfers: 4, // 默认并行数为4
            large_file_threshold: 10 * 1024 * 1024, // 10MB
            chunk_size: 5 * 1024 * 1024,            // 5MB
            max_retries: 5,              // 增加重试次数
            retry_base_delay_ms: 2000,   // 增加重试延迟
            enable_resume: true,
            scan_config: ScanConfig::default(),
            auto_create_dir: true,
        }
    }
}

/// 同步报告
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub jobId: String,
    pub startTime: i64,
    pub endTime: i64,
    pub status: SyncStatus,
    pub filesScanned: u32,
    pub filesCopied: u32,
    pub filesDeleted: u32,
    pub filesSkipped: u32,
    pub filesFailed: u32,
    pub bytesTransferred: u64,
    pub duration: u64,
    pub errors: Vec<String>,
}

/// 传输统计
#[derive(Debug, Default)]
struct TransferStats {
    files_completed: AtomicU64,
    files_failed: AtomicU64,
    bytes_transferred: AtomicU64,
}

/// 执行结果，包含字节数和可选的 hash
struct ActionResult {
    bytes: u64,
    file_path: Option<String>,
    file_hash: Option<String>,
    file_size: Option<i64>,
}

/// 带重试的动作执行结果
struct RetryResult {
    bytes: u64,
    file_state: Option<FileState>,
}

/// 同步引擎
pub struct SyncEngine {
    db: Arc<sqlx::SqlitePool>,
    config: SyncConfig,
    cancelled: Arc<AtomicBool>,
}

impl SyncEngine {
    pub fn new(db: Arc<sqlx::SqlitePool>) -> Self {
        Self {
            db,
            config: SyncConfig::default(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_config(db: Arc<sqlx::SqlitePool>, config: SyncConfig) -> Self {
        Self {
            db,
            config,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 取消同步
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// 检查是否已取消
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// 运行同步任务
    pub async fn run_sync(
        &self,
        job: &SyncJob,
        progress_tx: Option<mpsc::Sender<SyncProgress>>,
    ) -> Result<SyncReport> {
        let start_time = chrono::Utc::now().timestamp();
        let job_id = job.id.clone();

        info!("开始同步任务: {} ({})", job.name, job_id);

        // 重置取消标志
        self.cancelled.store(false, Ordering::SeqCst);

        // 发送初始进度
        self.send_progress(
            &progress_tx,
            SyncProgress {
                jobId: job_id.clone(),
                status: SyncStatus::Scanning,
                phase: "正在连接存储...".to_string(),
                currentFile: String::new(),
                filesScanned: 0,
                filesToSync: 0,
                filesCompleted: 0,
                filesSkipped: 0,
                filesFailed: 0,
                bytesTransferred: 0,
                bytesTotal: 0,
                speed: 0,
                eta: 0,
                startTime: start_time,
            },
        )
        .await;

        // 创建存储连接
        let source_storage = match crate::storage::create_storage(&job.sourceConfig).await {
            Ok(s) => s,
            Err(e) => {
                error!("创建源存储失败: {}", e);
                return Ok(self.create_failed_report(
                    &job_id,
                    start_time,
                    vec![format!("源存储连接失败: {}", e)],
                ));
            }
        };

        let dest_storage = match crate::storage::create_storage(&job.destConfig).await {
            Ok(s) => s,
            Err(e) => {
                error!("创建目标存储失败: {}", e);
                return Ok(self.create_failed_report(
                    &job_id,
                    start_time,
                    vec![format!("目标存储连接失败: {}", e)],
                ));
            }
        };

        // 检测目标目录是否可访问
        match dest_storage.list_files(None).await {
            Ok(_) => {
                debug!("目标存储可访问");
            }
            Err(e) => {
                let err_str = e.to_string();
                // 检测是否是目录不存在的错误 (409 Conflict 或 404 Not Found)
                if err_str.contains("409") || err_str.contains("Conflict") || err_str.contains("404") || err_str.contains("NotFound") {
                    if self.config.auto_create_dir {
                        debug!("目标目录不存在，尝试自动创建...");
                        // 尝试创建根目录
                        if let Err(create_err) = dest_storage.create_dir("/").await {
                            debug!("创建根目录失败: {}", create_err);
                            // 再次尝试 list 检查是否可用
                            if dest_storage.list_files(None).await.is_err() {
                                warn!("目标目录不存在且无法创建");
                                return Ok(self.create_failed_report(
                                    &job_id,
                                    start_time,
                                    vec!["目标目录不存在且无法自动创建，请先在云端手动创建该目录".to_string()],
                                ));
                            }
                        }
                        debug!("目标目录创建成功或已存在");
                    } else {
                        warn!("目标目录不存在或无法访问");
                        return Ok(self.create_failed_report(
                            &job_id,
                            start_time,
                            vec!["目标目录不存在，请先在云端创建该目录，或在设置中开启「自动创建目录」".to_string()],
                        ));
                    }
                } else {
                    // 其他错误继续，可能只是临时问题
                    warn!("检测目标目录时出错（继续同步）: {}", e);
                }
            }
        }

        // 检查取消
        if self.is_cancelled() {
            return Ok(self.create_cancelled_report(&job_id, start_time));
        }

        // 扫描文件
        self.send_progress(
            &progress_tx,
            SyncProgress {
                jobId: job_id.clone(),
                status: SyncStatus::Scanning,
                phase: "正在扫描源文件...".to_string(),
                currentFile: String::new(),
                filesScanned: 0,
                filesToSync: 0,
                filesCompleted: 0,
                filesSkipped: 0,
                filesFailed: 0,
                bytesTransferred: 0,
                bytesTotal: 0,
                speed: 0,
                eta: 0,
                startTime: start_time,
            },
        )
        .await;

        let scanner = FileScanner::with_config(8, self.config.scan_config.clone());

        let source_tree = match scanner.scan_storage(source_storage.as_ref(), None).await {
            Ok(t) => t,
            Err(e) => {
                error!("扫描源存储失败: {}", e);
                return Ok(self.create_failed_report(
                    &job_id,
                    start_time,
                    vec![format!("扫描源存储失败: {}", e)],
                ));
            }
        };

        if self.is_cancelled() {
            return Ok(self.create_cancelled_report(&job_id, start_time));
        }

        self.send_progress(
            &progress_tx,
            SyncProgress {
                jobId: job_id.clone(),
                status: SyncStatus::Scanning,
                phase: format!("正在扫描云端文件 (本地 {} 个)...", source_tree.len()),
                currentFile: "检查缓存...".to_string(),
                filesScanned: source_tree.len() as u32,
                filesToSync: 0,
                filesCompleted: 0,
                filesSkipped: 0,
                filesFailed: 0,
                bytesTransferred: 0,
                bytesTotal: 0,
                speed: 0,
                eta: 0,
                startTime: start_time,
            },
        )
        .await;

        // 尝试从缓存加载目标文件列表（仅对远程存储使用缓存）
        let cache_dir = crate::dirs::cache_dir()
            .map(|p| p.join("synctools").join("file_cache"))
            .unwrap_or_else(|| std::path::PathBuf::from(".synctools/cache"));
        let cache = FileListCache::new(cache_dir).with_ttl(300); // 5分钟缓存
        let dest_config_json = serde_json::to_string(&job.destConfig).unwrap_or_default();
        let is_remote_dest = matches!(job.destConfig.typ, crate::db::StorageType::S3 | crate::db::StorageType::WebDav);

        let dest_tree = if is_remote_dest {
            // 尝试从缓存加载
            if let Some(cached) = cache.load(&job_id, "dest", &dest_config_json) {
                debug!("使用缓存的目标文件列表 ({} 个文件)", cached.len());
                self.send_progress(
                    &progress_tx,
                    SyncProgress {
                        jobId: job_id.clone(),
                        status: SyncStatus::Scanning,
                        phase: format!("从缓存加载云端文件列表 ({} 个)...", cached.len()),
                        currentFile: String::new(),
                        filesScanned: source_tree.len() as u32,
                        filesToSync: 0,
                        filesCompleted: 0,
                        filesSkipped: 0,
                        filesFailed: 0,
                        bytesTransferred: 0,
                        bytesTotal: 0,
                        speed: 0,
                        eta: 0,
                        startTime: start_time,
                    },
                )
                .await;
                cached
            } else {
                // 缓存未命中，重新扫描
                self.send_progress(
                    &progress_tx,
                    SyncProgress {
                        jobId: job_id.clone(),
                        status: SyncStatus::Scanning,
                        phase: format!("正在扫描云端文件 (本地 {} 个)...", source_tree.len()),
                        currentFile: "WebDAV 响应较慢，请耐心等待".to_string(),
                        filesScanned: source_tree.len() as u32,
                        filesToSync: 0,
                        filesCompleted: 0,
                        filesSkipped: 0,
                        filesFailed: 0,
                        bytesTransferred: 0,
                        bytesTotal: 0,
                        speed: 0,
                        eta: 0,
                        startTime: start_time,
                    },
                )
                .await;

                match scanner.scan_storage(dest_storage.as_ref(), None).await {
                    Ok(t) => {
                        // 保存到缓存
                        let _ = cache.save(&job_id, "dest", &dest_config_json, &t);
                        t
                    }
                    Err(e) => {
                        error!("扫描目标存储失败: {}", e);
                        return Ok(self.create_failed_report(
                            &job_id,
                            start_time,
                            vec![format!("扫描目标存储失败: {}", e)],
                        ));
                    }
                }
            }
        } else {
            // 本地存储不使用缓存
            match scanner.scan_storage(dest_storage.as_ref(), None).await {
                Ok(t) => t,
                Err(e) => {
                    error!("扫描目标存储失败: {}", e);
                    return Ok(self.create_failed_report(
                        &job_id,
                        start_time,
                        vec![format!("扫描目标存储失败: {}", e)],
                    ));
                }
            }
        };

        let files_scanned = (source_tree.len() + dest_tree.len()) as u32;
        debug!(
            "扫描完成: 源 {} 文件, 目标 {} 文件",
            source_tree.len(),
            dest_tree.len()
        );

        if self.is_cancelled() {
            return Ok(self.create_cancelled_report(&job_id, start_time));
        }

        // 比较文件
        self.send_progress(
            &progress_tx,
            SyncProgress {
                jobId: job_id.clone(),
                status: SyncStatus::Comparing,
                phase: "正在比较文件差异...".to_string(),
                currentFile: String::new(),
                filesScanned: files_scanned,
                filesToSync: 0,
                filesCompleted: 0,
                filesSkipped: 0,
                filesFailed: 0,
                bytesTransferred: 0,
                bytesTotal: 0,
                speed: 0,
                eta: 0,
                startTime: start_time,
            },
        )
        .await;

        let comparator = FileComparator::default();
        let mut actions = comparator.compare_trees(&source_tree, &dest_tree, &job.syncMode);

        // 加载已保存的文件状态，用于增量同步
        let state_manager = FileStateManager::new(self.db.clone());
        let saved_states = state_manager.get_job_states(&job_id).await.unwrap_or_default();
        
        // 用 hash 过滤不需要同步的文件
        let mut skipped_by_hash = 0usize;
        let mut files_to_hash: Vec<(String, SyncAction)> = Vec::new();
        
        for action in actions.iter_mut() {
            if let SyncAction::Copy { source_path, size, reverse, .. } = action {
                if !*reverse {
                    // 检查是否有保存的状态
                    if let Some(saved) = saved_states.get(source_path) {
                        // 如果大小相同且有 hash 记录，尝试读取文件检查 hash
                        if saved.file_size == *size as i64 && saved.checksum.is_some() {
                            files_to_hash.push((source_path.clone(), action.clone()));
                        }
                    }
                }
            }
        }
        
        // 计算需要检查的文件的 hash
        if !files_to_hash.is_empty() {
            debug!("检查 {} 个文件的 hash 是否变化...", files_to_hash.len());
            
            for (path, _) in &files_to_hash {
                if let Some(saved) = saved_states.get(path) {
                    if let Some(saved_hash) = &saved.checksum {
                        // 读取文件计算 hash
                        match source_storage.read(path).await {
                            Ok(data) => {
                                let current_hash = calculate_quick_hash(&data);
                                if &current_hash == saved_hash {
                                    // Hash 相同，转为 Skip
                                    debug!("文件未变化，跳过: {}", path);
                                    skipped_by_hash += 1;
                                    // 标记为跳过
                                    for action in actions.iter_mut() {
                                        if let SyncAction::Copy { source_path, .. } = action {
                                            if source_path == path {
                                                *action = SyncAction::Skip { path: path.clone() };
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("读取文件失败，继续同步: {} - {}", path, e);
                            }
                        }
                    }
                }
            }
        }
        
        let summary = FileComparator::summarize_actions(&actions);

        debug!(
            "比较完成: {} 个操作, {} 个复制, {} 个删除, {} 个跳过 (hash匹配跳过: {}), {} 个冲突",
            actions.len(),
            summary.copy_count + summary.reverse_copy_count,
            summary.delete_count,
            summary.skip_count,
            skipped_by_hash,
            summary.conflict_count
        );

        let files_to_sync =
            (summary.copy_count + summary.reverse_copy_count + summary.delete_count) as u32;
        let bytes_total = summary.total_transfer_bytes();

        if self.is_cancelled() {
            return Ok(self.create_cancelled_report(&job_id, start_time));
        }

        // 执行同步
        self.send_progress(
            &progress_tx,
            SyncProgress {
                jobId: job_id.clone(),
                status: SyncStatus::Syncing,
                phase: format!("准备同步 {} 个文件...", files_to_sync),
                currentFile: String::new(),
                filesScanned: files_scanned,
                filesToSync: files_to_sync,
                filesCompleted: 0,
                filesSkipped: summary.skip_count as u32,
                filesFailed: 0,
                bytesTransferred: 0,
                bytesTotal: bytes_total,
                speed: 0,
                eta: 0,
                startTime: start_time,
            },
        )
        .await;

        // 执行并行同步
        let result = self
            .execute_sync_parallel(
                &job_id,
                source_storage.clone(),
                dest_storage.clone(),
                actions,
                &summary,
                progress_tx.clone(),
                start_time,
                files_scanned,
            )
            .await;

        let (files_copied, files_deleted, files_failed, bytes_transferred, errors) = result;

        let end_time = chrono::Utc::now().timestamp();
        let status = if files_failed > 0 {
            SyncStatus::Failed
        } else if self.is_cancelled() {
            SyncStatus::Cancelled
        } else {
            SyncStatus::Completed
        };

        // 记录到数据库
        self.log_sync_result(
            &job_id,
            start_time,
            end_time,
            &status,
            files_scanned,
            files_copied,
            files_deleted,
            bytes_transferred,
            if errors.is_empty() {
                None
            } else {
                Some(errors.join("; "))
            },
        )
        .await;

        // 发送完成进度
        self.send_progress(
            &progress_tx,
            SyncProgress {
                jobId: job_id.clone(),
                status: status.clone(),
                phase: "同步完成".to_string(),
                currentFile: String::new(),
                filesScanned: files_scanned,
                filesToSync: files_to_sync,
                filesCompleted: files_copied + files_deleted,
                filesSkipped: summary.skip_count as u32,
                filesFailed: files_failed,
                bytesTransferred: bytes_transferred,
                bytesTotal: bytes_total,
                speed: 0,
                eta: 0,
                startTime: start_time,
            },
        )
        .await;

        info!(
            "同步任务完成: {} - 复制 {}, 删除 {}, 失败 {}",
            job_id, files_copied, files_deleted, files_failed
        );

        // 同步完成后清除缓存（文件列表已变化）
        if is_remote_dest && (files_copied > 0 || files_deleted > 0) {
            cache.clear(&job_id);
            debug!("已清除目标存储缓存");
        }

        Ok(SyncReport {
            jobId: job_id.clone(),
            startTime: start_time,
            endTime: end_time,
            status,
            filesScanned: files_scanned,
            filesCopied: files_copied,
            filesDeleted: files_deleted,
            filesSkipped: summary.skip_count as u32,
            filesFailed: files_failed,
            bytesTransferred: bytes_transferred,
            duration: (end_time - start_time) as u64,
            errors,
        })
    }

    /// 并行执行同步操作
    #[allow(clippy::too_many_arguments)]
    async fn execute_sync_parallel(
        &self,
        job_id: &str,
        source_storage: Arc<dyn Storage>,
        dest_storage: Arc<dyn Storage>,
        actions: Vec<SyncAction>,
        summary: &ActionSummary,
        progress_tx: Option<mpsc::Sender<SyncProgress>>,
        start_time: i64,
        files_scanned: u32,
    ) -> (u32, u32, u32, u64, Vec<String>) {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_transfers));
        let stats = Arc::new(TransferStats::default());
        let errors = Arc::new(RwLock::new(Vec::<String>::new()));
        let synced_states = Arc::new(RwLock::new(Vec::<FileState>::new()));
        let cancelled = self.cancelled.clone();

        let files_to_sync =
            (summary.copy_count + summary.reverse_copy_count + summary.delete_count) as u32;
        let bytes_total = summary.total_transfer_bytes();

        // 过滤出需要执行的动作
        let executable_actions: Vec<_> = actions
            .into_iter()
            .filter(|a| !matches!(a, SyncAction::Skip { .. }))
            .collect();

        let mut handles = Vec::new();
        let _transfer_start = Instant::now();

        // 启动进度更新任务
        let progress_tx_clone = progress_tx.clone();
        let stats_clone = stats.clone();
        let job_id_clone = job_id.to_string();
        let cancelled_clone = cancelled.clone();

        let progress_handle = tokio::spawn(async move {
            let mut last_bytes = 0u64;
            let mut last_time = Instant::now();

            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;

                if cancelled_clone.load(Ordering::SeqCst) {
                    break;
                }

                let completed = stats_clone.files_completed.load(Ordering::Relaxed);
                let failed = stats_clone.files_failed.load(Ordering::Relaxed);
                let bytes = stats_clone.bytes_transferred.load(Ordering::Relaxed);

                // 计算速度
                let now = Instant::now();
                let elapsed = now.duration_since(last_time).as_secs_f64();
                let speed = if elapsed > 0.0 {
                    ((bytes - last_bytes) as f64 / elapsed) as u64
                } else {
                    0
                };
                last_bytes = bytes;
                last_time = now;

                // 计算 ETA
                let remaining_bytes = bytes_total.saturating_sub(bytes);
                let eta = if speed > 0 {
                    remaining_bytes / speed
                } else {
                    0
                };

                if let Some(tx) = &progress_tx_clone {
                    let _ = tx
                        .send(SyncProgress {
                            jobId: job_id_clone.clone(),
                            status: SyncStatus::Syncing,
                            phase: format!("同步中 {}/{}", completed + failed, files_to_sync),
                            currentFile: String::new(),
                            filesScanned: files_scanned,
                            filesToSync: files_to_sync,
                            filesCompleted: (completed + failed) as u32,
                            filesSkipped: 0,
                            filesFailed: failed as u32,
                            bytesTransferred: bytes,
                            bytesTotal: bytes_total,
                            speed,
                            eta,
                            startTime: start_time,
                        })
                        .await;
                }

                // 检查是否完成
                if completed + failed >= files_to_sync as u64 {
                    break;
                }
            }
        });

        // 执行每个动作
        for action in executable_actions {
            if cancelled.load(Ordering::SeqCst) {
                break;
            }

            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let source = source_storage.clone();
            let dest = dest_storage.clone();
            let stats = stats.clone();
            let errors = errors.clone();
            let synced_states = synced_states.clone();
            let cancelled = cancelled.clone();
            let max_retries = self.config.max_retries;
            let retry_delay = self.config.retry_base_delay_ms;
            let job_id = job_id.to_string();

            let handle = tokio::spawn(async move {
                let result = Self::execute_action_with_retry(
                    &action,
                    source.as_ref(),
                    dest.as_ref(),
                    max_retries,
                    retry_delay,
                    &cancelled,
                    &job_id,
                )
                .await;

                match result {
                    Ok(retry_result) => {
                        stats.files_completed.fetch_add(1, Ordering::Relaxed);
                        stats.bytes_transferred.fetch_add(retry_result.bytes, Ordering::Relaxed);
                        
                        // 收集成功同步的文件状态
                        if let Some(state) = retry_result.file_state {
                            let mut states = synced_states.write().await;
                            states.push(state);
                        }
                    }
                    Err(e) => {
                        stats.files_failed.fetch_add(1, Ordering::Relaxed);
                        let mut errs = errors.write().await;
                        errs.push(e);
                    }
                }

                drop(permit);
            });

            handles.push(handle);
        }

        // 等待所有任务完成
        for handle in handles {
            let _ = handle.await;
        }

        // 停止进度更新
        progress_handle.abort();

        // 保存成功同步的文件状态
        let states_to_save = synced_states.read().await.clone();
        if !states_to_save.is_empty() {
            let state_manager = FileStateManager::new(self.db.clone());
            if let Err(e) = state_manager.batch_upsert(&states_to_save).await {
                warn!("保存文件状态失败: {}", e);
            } else {
                debug!("已保存 {} 个文件的同步状态", states_to_save.len());
            }
        }

        let files_completed = stats.files_completed.load(Ordering::Relaxed) as u32;
        let files_failed = stats.files_failed.load(Ordering::Relaxed) as u32;
        let bytes_transferred = stats.bytes_transferred.load(Ordering::Relaxed);

        // 分离复制和删除的计数
        let files_copied =
            files_completed.min(summary.copy_count as u32 + summary.reverse_copy_count as u32);
        let files_deleted = files_completed.saturating_sub(files_copied);

        let error_list = errors.read().await.clone();

        (
            files_copied,
            files_deleted,
            files_failed,
            bytes_transferred,
            error_list,
        )
    }

    /// 带重试的动作执行
    async fn execute_action_with_retry(
        action: &SyncAction,
        source: &dyn Storage,
        dest: &dyn Storage,
        max_retries: u32,
        base_delay_ms: u64,
        cancelled: &AtomicBool,
        job_id: &str,
    ) -> Result<RetryResult, String> {
        let mut last_error = String::new();

        for attempt in 0..=max_retries {
            if cancelled.load(Ordering::SeqCst) {
                return Err("操作已取消".to_string());
            }

            match Self::execute_action(action, source, dest).await {
                Ok(result) => {
                    // 如果有文件信息，创建 FileState
                    let file_state = if let (Some(path), Some(hash), Some(size)) = 
                        (result.file_path, result.file_hash, result.file_size) {
                        Some(FileState {
                            job_id: job_id.to_string(),
                            file_path: path,
                            file_size: size,
                            modified_time: chrono::Utc::now().timestamp(),
                            checksum: Some(hash),
                            last_sync_time: Some(chrono::Utc::now().timestamp()),
                        })
                    } else {
                        None
                    };
                    
                    return Ok(RetryResult {
                        bytes: result.bytes,
                        file_state,
                    });
                }
                Err(e) => {
                    last_error = e.to_string();

                    if attempt < max_retries {
                        // 指数退避
                        let delay = base_delay_ms * (2_u64.pow(attempt));
                        warn!(
                            "操作失败，{}ms 后重试 ({}/{}): {}",
                            delay,
                            attempt + 1,
                            max_retries,
                            last_error
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    } else {
                        error!("操作最终失败 (已重试{}次): {}", max_retries, last_error);
                    }
                }
            }
        }

        let path = match action {
            SyncAction::Copy { source_path, .. } => source_path.clone(),
            SyncAction::Delete { path, .. } => path.clone(),
            SyncAction::Skip { path } => path.clone(),
            SyncAction::Conflict { path, .. } => path.clone(),
        };

        Err(format!("{}: {}", path, last_error))
    }

    /// 执行单个动作
    async fn execute_action(
        action: &SyncAction,
        source: &dyn Storage,
        dest: &dyn Storage,
    ) -> Result<ActionResult> {
        match action {
            SyncAction::Copy {
                source_path,
                dest_path,
                size,
                reverse,
            } => {
                let (from, to, from_path, to_path) = if *reverse {
                    (dest, source, dest_path.as_str(), source_path.as_str())
                } else {
                    (source, dest, source_path.as_str(), dest_path.as_str())
                };

                debug!(
                    "复制: {} -> {} ({}字节, reverse={})",
                    from_path, to_path, size, reverse
                );

                let data = from.read(from_path).await?;
                debug!("  读取完成: {} 实际{}字节", from_path, data.len());

                // 计算文件 hash（用于增量同步）
                let file_hash = calculate_quick_hash(&data);
                let file_size = data.len() as i64;

                to.write(to_path, data).await?;
                debug!("  写入完成: {}", to_path);

                Ok(ActionResult {
                    bytes: *size,
                    file_path: if !*reverse { Some(source_path.clone()) } else { None },
                    file_hash: if !*reverse { Some(file_hash) } else { None },
                    file_size: if !*reverse { Some(file_size) } else { None },
                })
            }
            SyncAction::Delete { path, from_dest } => {
                let storage = if *from_dest { dest } else { source };
                storage.delete(path).await?;
                Ok(ActionResult {
                    bytes: 0,
                    file_path: None,
                    file_hash: None,
                    file_size: None,
                })
            }
            SyncAction::Skip { .. } => Ok(ActionResult {
                bytes: 0,
                file_path: None,
                file_hash: None,
                file_size: None,
            }),
            SyncAction::Conflict { path, .. } => {
                // 冲突暂时跳过，记录错误
                Err(anyhow::anyhow!("冲突未解决: {}", path))
            }
        }
    }

    /// 发送进度更新
    async fn send_progress(&self, tx: &Option<mpsc::Sender<SyncProgress>>, progress: SyncProgress) {
        if let Some(tx) = tx {
            let _ = tx.send(progress).await;
        }
    }

    /// 创建失败报告
    fn create_failed_report(
        &self,
        job_id: &str,
        start_time: i64,
        errors: Vec<String>,
    ) -> SyncReport {
        let end_time = chrono::Utc::now().timestamp();
        SyncReport {
            jobId: job_id.to_string(),
            startTime: start_time,
            endTime: end_time,
            status: SyncStatus::Failed,
            filesScanned: 0,
            filesCopied: 0,
            filesDeleted: 0,
            filesSkipped: 0,
            filesFailed: 0,
            bytesTransferred: 0,
            duration: (end_time - start_time) as u64,
            errors,
        }
    }

    /// 创建取消报告
    fn create_cancelled_report(&self, job_id: &str, start_time: i64) -> SyncReport {
        let end_time = chrono::Utc::now().timestamp();
        SyncReport {
            jobId: job_id.to_string(),
            startTime: start_time,
            endTime: end_time,
            status: SyncStatus::Cancelled,
            filesScanned: 0,
            filesCopied: 0,
            filesDeleted: 0,
            filesSkipped: 0,
            filesFailed: 0,
            bytesTransferred: 0,
            duration: (end_time - start_time) as u64,
            errors: vec!["同步已取消".to_string()],
        }
    }

    /// 记录同步结果到数据库
    #[allow(clippy::too_many_arguments)]
    async fn log_sync_result(
        &self,
        job_id: &str,
        start_time: i64,
        end_time: i64,
        status: &SyncStatus,
        files_scanned: u32,
        files_copied: u32,
        files_deleted: u32,
        bytes_transferred: u64,
        error_message: Option<String>,
    ) {
        let status_str = match status {
            SyncStatus::Completed => "completed",
            SyncStatus::Failed => "failed",
            SyncStatus::Cancelled => "cancelled",
            _ => "unknown",
        };

        let result = sqlx::query(
            r#"INSERT INTO sync_logs 
               (job_id, start_time, end_time, status, files_scanned, files_copied, files_deleted, bytes_transferred, error_message)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(job_id)
        .bind(start_time)
        .bind(end_time)
        .bind(status_str)
        .bind(files_scanned as i64)
        .bind(files_copied as i64)
        .bind(files_deleted as i64)
        .bind(bytes_transferred as i64)
        .bind(error_message)
        .execute(&*self.db)
        .await;

        if let Err(e) = result {
            warn!("记录同步日志失败: {}", e);
        }
    }

    /// 获取数据库引用
    pub fn db(&self) -> &sqlx::SqlitePool {
        &self.db
    }
}
