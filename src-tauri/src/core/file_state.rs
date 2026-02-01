//! 文件状态管理 - 用于增量同步

use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// 文件状态记录
#[derive(Debug, Clone)]
pub struct FileState {
    pub job_id: String,
    pub file_path: String,
    pub file_size: i64,
    pub modified_time: i64,
    pub checksum: Option<String>,
    pub last_sync_time: Option<i64>,
}

/// 数据库行
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct FileStateRow {
    id: i64,
    job_id: String,
    file_path: String,
    file_size: i64,
    modified_time: i64,
    checksum: Option<String>,
    last_sync_time: Option<i64>,
}

impl From<FileStateRow> for FileState {
    fn from(row: FileStateRow) -> Self {
        FileState {
            job_id: row.job_id,
            file_path: row.file_path,
            file_size: row.file_size,
            modified_time: row.modified_time,
            checksum: row.checksum,
            last_sync_time: row.last_sync_time,
        }
    }
}

/// 文件状态管理器
pub struct FileStateManager {
    db: Arc<SqlitePool>,
}

impl FileStateManager {
    pub fn new(db: Arc<SqlitePool>) -> Self {
        Self { db }
    }

    /// 获取任务的所有文件状态（返回 HashMap 以便快速查找）
    pub async fn get_job_states(&self, job_id: &str) -> Result<HashMap<String, FileState>> {
        let rows = sqlx::query_as::<_, FileStateRow>(
            "SELECT * FROM file_states WHERE job_id = ?"
        )
        .bind(job_id)
        .fetch_all(&*self.db)
        .await?;

        let mut map = HashMap::new();
        for row in rows {
            let state: FileState = row.into();
            map.insert(state.file_path.clone(), state);
        }

        Ok(map)
    }

    /// 获取单个文件状态
    pub async fn get_file_state(&self, job_id: &str, file_path: &str) -> Result<Option<FileState>> {
        let row = sqlx::query_as::<_, FileStateRow>(
            "SELECT * FROM file_states WHERE job_id = ? AND file_path = ?"
        )
        .bind(job_id)
        .bind(file_path)
        .fetch_optional(&*self.db)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    /// 更新或插入文件状态
    pub async fn upsert_file_state(&self, state: &FileState) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"INSERT INTO file_states (job_id, file_path, file_size, modified_time, checksum, last_sync_time)
               VALUES (?, ?, ?, ?, ?, ?)
               ON CONFLICT(job_id, file_path) DO UPDATE SET
                   file_size = excluded.file_size,
                   modified_time = excluded.modified_time,
                   checksum = excluded.checksum,
                   last_sync_time = excluded.last_sync_time"#
        )
        .bind(&state.job_id)
        .bind(&state.file_path)
        .bind(state.file_size)
        .bind(state.modified_time)
        .bind(&state.checksum)
        .bind(state.last_sync_time.unwrap_or(now))
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    /// 批量更新文件状态
    pub async fn batch_upsert(&self, states: &[FileState]) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        for state in states {
            sqlx::query(
                r#"INSERT INTO file_states (job_id, file_path, file_size, modified_time, checksum, last_sync_time)
                   VALUES (?, ?, ?, ?, ?, ?)
                   ON CONFLICT(job_id, file_path) DO UPDATE SET
                       file_size = excluded.file_size,
                       modified_time = excluded.modified_time,
                       checksum = excluded.checksum,
                       last_sync_time = excluded.last_sync_time"#
            )
            .bind(&state.job_id)
            .bind(&state.file_path)
            .bind(state.file_size)
            .bind(state.modified_time)
            .bind(&state.checksum)
            .bind(state.last_sync_time.unwrap_or(now))
            .execute(&*self.db)
            .await?;
        }

        info!("批量更新 {} 个文件状态", states.len());
        Ok(())
    }

    /// 删除文件状态
    pub async fn delete_file_state(&self, job_id: &str, file_path: &str) -> Result<()> {
        sqlx::query("DELETE FROM file_states WHERE job_id = ? AND file_path = ?")
            .bind(job_id)
            .bind(file_path)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    /// 删除任务的所有文件状态
    pub async fn delete_job_states(&self, job_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM file_states WHERE job_id = ?")
            .bind(job_id)
            .execute(&*self.db)
            .await?;

        Ok(result.rows_affected())
    }

    /// 清理不存在的文件状态（文件已被删除）
    pub async fn cleanup_missing(&self, job_id: &str, existing_paths: &[String]) -> Result<u64> {
        if existing_paths.is_empty() {
            return Ok(0);
        }

        // 构建参数占位符
        let placeholders: Vec<&str> = existing_paths.iter().map(|_| "?").collect();
        let query = format!(
            "DELETE FROM file_states WHERE job_id = ? AND file_path NOT IN ({})",
            placeholders.join(",")
        );

        let mut q = sqlx::query(&query).bind(job_id);
        for path in existing_paths {
            q = q.bind(path);
        }

        let result = q.execute(&*self.db).await?;
        let deleted = result.rows_affected();

        if deleted > 0 {
            debug!("清理了 {} 个已删除文件的状态记录", deleted);
        }

        Ok(deleted)
    }
}

/// 计算文件内容的 hash（使用 BLAKE3 快速哈希）
pub fn calculate_hash(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    // 只取前 16 字节（32 个十六进制字符），足够检测变化
    hash.to_hex()[..32].to_string()
}

/// 快速计算文件 hash（基于采样，适用于大文件）
pub fn calculate_quick_hash(data: &[u8]) -> String {
    let len = data.len();
    if len <= 65536 {
        // 小于 64KB，完整哈希
        return calculate_hash(data);
    }
    
    // 大文件：采样哈希（头部 + 中部 + 尾部 + 大小）
    let mut hasher = blake3::Hasher::new();
    let chunk_size = 16384; // 16KB
    
    hasher.update(&data[..chunk_size]); // 头部
    hasher.update(&data[len / 2 - chunk_size / 2..len / 2 + chunk_size / 2]); // 中部
    hasher.update(&data[len - chunk_size..]); // 尾部
    hasher.update(&len.to_le_bytes()); // 文件大小也参与哈希
    
    let hash = hasher.finalize();
    hash.to_hex()[..32].to_string()
}
