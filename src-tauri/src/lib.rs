use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub mod commands;
pub mod config;
pub mod core;
pub mod db;
pub mod logging;
pub mod storage;

pub use core::{SyncConfig, SyncEngine, SyncReport};
pub use db::models::{StorageConfig, StorageType, SyncJob, SyncMode};

/// 应用状态，在 Tauri 命令中共享
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub sync_engine: Arc<Mutex<Option<SyncEngine>>>,
    pub config_dir: PathBuf,
    /// 同步任务取消信号
    pub cancel_signals: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
    /// 分析任务取消标志（使用 AtomicBool 便于跨线程检查）
    pub analyze_cancels: Arc<Mutex<HashMap<String, Arc<std::sync::atomic::AtomicBool>>>>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        // 获取默认应用配置目录
        let default_config_dir = dirs::config_dir()
            .map(|p| p.join("synctools"))
            .unwrap_or_else(|| PathBuf::from(".synctools"));

        std::fs::create_dir_all(&default_config_dir)?;

        // 尝试读取自定义数据路径（使用链式调用简化嵌套逻辑）
        let config_file = default_config_dir.join("config.json");
        let config_dir = std::fs::read_to_string(&config_file)
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .and_then(|config| config.get("data_path")?.as_str().map(PathBuf::from))
            .filter(|p| p.exists() && p.is_dir())
            .inspect(|p| tracing::debug!("使用自定义数据路径: {:?}", p))
            .unwrap_or(default_config_dir);

        std::fs::create_dir_all(&config_dir)?;

        // 初始化数据库（带连接池配置）
        let db_path = config_dir.join("synctools.db");
        // SQLite 连接字符串格式: sqlite://path 或 sqlite:path
        // Windows 路径需要转换反斜杠为正斜杠
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?
            .replace('\\', "/");
        
        let db = SqlitePoolOptions::new()
            .max_connections(5)  // SQLite 单文件，不需要太多连接
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))  // 10分钟空闲超时
            .connect(&format!("sqlite:{}?mode=rwc", db_path_str))
            .await?;

        // 运行数据库迁移
        sqlx::migrate!("./migrations").run(&db).await?;

        Ok(Self {
            db: Arc::new(db),
            sync_engine: Arc::new(Mutex::new(None)),
            config_dir,
            cancel_signals: Arc::new(Mutex::new(HashMap::new())),
            analyze_cancels: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// 清理资源（应用关闭时调用）
    pub async fn cleanup(&self) {
        tracing::info!("正在清理应用资源...");

        // 1. 取消所有正在进行的同步任务
        {
            let mut signals = self.cancel_signals.lock().await;
            for (job_id, sender) in signals.drain() {
                tracing::debug!("取消同步任务: {}", job_id);
                let _ = sender.send(());
            }
        }

        // 2. 标记所有分析任务为已取消
        {
            let cancels = self.analyze_cancels.lock().await;
            for (job_id, flag) in cancels.iter() {
                tracing::debug!("取消分析任务: {}", job_id);
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }

        // 3. 关闭数据库连接池
        tracing::debug!("关闭数据库连接池...");
        self.db.close().await;

        tracing::info!("资源清理完成");
    }
}

// 为了兼容性，添加 dirs 依赖
pub mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        if cfg!(target_os = "windows") {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
        } else {
            // Linux
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        }
    }

    pub fn cache_dir() -> Option<PathBuf> {
        if cfg!(target_os = "windows") {
            std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library").join("Caches"))
        } else {
            // Linux
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".cache"))
        }
    }
}
