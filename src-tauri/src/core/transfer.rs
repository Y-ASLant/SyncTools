//! 传输状态管理 - 支持断点续传

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 传输状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Paused,
}

impl std::fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferStatus::Pending => write!(f, "pending"),
            TransferStatus::InProgress => write!(f, "in_progress"),
            TransferStatus::Completed => write!(f, "completed"),
            TransferStatus::Failed => write!(f, "failed"),
            TransferStatus::Paused => write!(f, "paused"),
        }
    }
}

impl From<&str> for TransferStatus {
    fn from(s: &str) -> Self {
        match s {
            "pending" => TransferStatus::Pending,
            "in_progress" => TransferStatus::InProgress,
            "completed" => TransferStatus::Completed,
            "failed" => TransferStatus::Failed,
            "paused" => TransferStatus::Paused,
            _ => TransferStatus::Pending,
        }
    }
}

/// 传输状态记录
#[derive(Debug, Clone)]
pub struct TransferState {
    pub id: String,
    pub job_id: String,
    pub file_path: String,
    pub total_size: u64,
    pub transferred_size: u64,
    pub upload_id: Option<String>,
    pub parts_completed: Vec<u32>,
    pub status: TransferStatus,
    pub started_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// 数据库行
#[derive(Debug, sqlx::FromRow)]
struct TransferStateRow {
    id: String,
    job_id: String,
    file_path: String,
    total_size: i64,
    transferred_size: i64,
    upload_id: Option<String>,
    parts_completed: Option<String>,
    status: String,
    started_at: Option<i64>,
    updated_at: Option<i64>,
}

impl From<TransferStateRow> for TransferState {
    fn from(row: TransferStateRow) -> Self {
        let parts: Vec<u32> = row
            .parts_completed
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        TransferState {
            id: row.id,
            job_id: row.job_id,
            file_path: row.file_path,
            total_size: row.total_size as u64,
            transferred_size: row.transferred_size as u64,
            upload_id: row.upload_id,
            parts_completed: parts,
            status: TransferStatus::from(row.status.as_str()),
            started_at: row.started_at,
            updated_at: row.updated_at,
        }
    }
}

/// 传输管理器
pub struct TransferManager {
    db: Arc<SqlitePool>,
    /// 内存缓存，用于快速查询
    cache: RwLock<HashMap<String, TransferState>>,
}

impl TransferManager {
    pub fn new(db: Arc<SqlitePool>) -> Self {
        Self {
            db,
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// 获取任务的所有未完成传输
    pub async fn get_pending_transfers(&self, job_id: &str) -> Result<Vec<TransferState>> {
        let rows = sqlx::query_as::<_, TransferStateRow>(
            "SELECT * FROM transfer_states WHERE job_id = ? AND status IN ('pending', 'in_progress', 'paused')"
        )
        .bind(job_id)
        .fetch_all(&*self.db)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 创建或更新传输状态
    pub async fn save_transfer(&self, state: &TransferState) -> Result<()> {
        let parts_json = serde_json::to_string(&state.parts_completed)?;
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"INSERT INTO transfer_states 
               (id, job_id, file_path, total_size, transferred_size, upload_id, parts_completed, status, started_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                   transferred_size = excluded.transferred_size,
                   upload_id = excluded.upload_id,
                   parts_completed = excluded.parts_completed,
                   status = excluded.status,
                   updated_at = excluded.updated_at"#
        )
        .bind(&state.id)
        .bind(&state.job_id)
        .bind(&state.file_path)
        .bind(state.total_size as i64)
        .bind(state.transferred_size as i64)
        .bind(&state.upload_id)
        .bind(&parts_json)
        .bind(state.status.to_string())
        .bind(state.started_at.unwrap_or(now))
        .bind(now)
        .execute(&*self.db)
        .await?;

        // 更新缓存
        let mut cache = self.cache.write().await;
        cache.insert(state.id.clone(), state.clone());

        Ok(())
    }

    /// 更新传输进度
    pub async fn update_progress(&self, id: &str, transferred: u64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query("UPDATE transfer_states SET transferred_size = ?, updated_at = ? WHERE id = ?")
            .bind(transferred as i64)
            .bind(now)
            .bind(id)
            .execute(&*self.db)
            .await?;

        // 更新缓存
        let mut cache = self.cache.write().await;
        if let Some(state) = cache.get_mut(id) {
            state.transferred_size = transferred;
            state.updated_at = Some(now);
        }

        Ok(())
    }

    /// 标记传输完成
    pub async fn mark_completed(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query("UPDATE transfer_states SET status = 'completed', updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(&*self.db)
            .await?;

        // 从缓存移除
        let mut cache = self.cache.write().await;
        cache.remove(id);

        Ok(())
    }

    /// 标记传输失败
    pub async fn mark_failed(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query("UPDATE transfer_states SET status = 'failed', updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(&*self.db)
            .await?;

        // 更新缓存
        let mut cache = self.cache.write().await;
        if let Some(state) = cache.get_mut(id) {
            state.status = TransferStatus::Failed;
        }

        Ok(())
    }

    /// 清理已完成的传输记录
    pub async fn cleanup_completed(&self, job_id: &str) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM transfer_states WHERE job_id = ? AND status = 'completed'")
                .bind(job_id)
                .execute(&*self.db)
                .await?;

        Ok(result.rows_affected())
    }

    /// 清理所有传输记录
    pub async fn cleanup_all(&self, job_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM transfer_states WHERE job_id = ?")
            .bind(job_id)
            .execute(&*self.db)
            .await?;

        // 清理缓存
        let mut cache = self.cache.write().await;
        cache.retain(|_, v| v.job_id != job_id);

        Ok(result.rows_affected())
    }

    /// 获取传输状态
    pub async fn get_transfer(&self, id: &str) -> Result<Option<TransferState>> {
        // 先查缓存
        {
            let cache = self.cache.read().await;
            if let Some(state) = cache.get(id) {
                return Ok(Some(state.clone()));
            }
        }

        // 查数据库
        let row =
            sqlx::query_as::<_, TransferStateRow>("SELECT * FROM transfer_states WHERE id = ?")
                .bind(id)
                .fetch_optional(&*self.db)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    /// 创建新的传输状态
    pub fn create_transfer_state(job_id: &str, file_path: &str, total_size: u64) -> TransferState {
        TransferState {
            id: uuid::Uuid::new_v4().to_string(),
            job_id: job_id.to_string(),
            file_path: file_path.to_string(),
            total_size,
            transferred_size: 0,
            upload_id: None,
            parts_completed: vec![],
            status: TransferStatus::Pending,
            started_at: Some(chrono::Utc::now().timestamp()),
            updated_at: None,
        }
    }
}
