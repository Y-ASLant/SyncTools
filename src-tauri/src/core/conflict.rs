use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

/// 冲突解决策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    KeepSource,
    KeepDest,
    KeepBoth,
    Skip,
}

impl std::fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictResolution::KeepSource => write!(f, "keep_source"),
            ConflictResolution::KeepDest => write!(f, "keep_dest"),
            ConflictResolution::KeepBoth => write!(f, "keep_both"),
            ConflictResolution::Skip => write!(f, "skip"),
        }
    }
}

impl From<&str> for ConflictResolution {
    fn from(s: &str) -> Self {
        match s {
            "keep_source" => ConflictResolution::KeepSource,
            "keep_dest" => ConflictResolution::KeepDest,
            "keep_both" => ConflictResolution::KeepBoth,
            "skip" => ConflictResolution::Skip,
            _ => ConflictResolution::Skip,
        }
    }
}

/// 冲突记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictRecord {
    pub id: i64,
    pub job_id: String,
    pub file_path: String,
    pub conflict_type: String,
    pub resolution: Option<String>,
    pub source_size: Option<u64>,
    pub source_time: Option<i64>,
    pub dest_size: Option<u64>,
    pub dest_time: Option<i64>,
    pub created_at: i64,
}

/// 数据库行
#[derive(Debug, sqlx::FromRow)]
struct ConflictRow {
    id: i64,
    job_id: String,
    file_path: String,
    conflict_type: String,
    resolution: Option<String>,
    source_time: Option<i64>,
    dest_time: Option<i64>,
    created_at: i64,
}

/// 冲突解决器
#[derive(Debug)]
pub struct ConflictResolver {
    db: Arc<SqlitePool>,
    default_resolution: ConflictResolution,
}

impl ConflictResolver {
    pub fn new(db: Arc<SqlitePool>, default_resolution: ConflictResolution) -> Self {
        Self {
            db,
            default_resolution,
        }
    }

    /// 记录冲突
    pub async fn record_conflict(
        &self,
        job_id: &str,
        file_path: &str,
        conflict_type: &str,
        source_time: Option<i64>,
        dest_time: Option<i64>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query(
            r#"INSERT INTO conflicts (job_id, file_path, conflict_type, source_time, dest_time, created_at)
               VALUES (?, ?, ?, ?, ?, ?)"#
        )
        .bind(job_id)
        .bind(file_path)
        .bind(conflict_type)
        .bind(source_time)
        .bind(dest_time)
        .bind(now)
        .execute(&*self.db)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 获取任务的未解决冲突
    pub async fn get_pending_conflicts(&self, job_id: &str) -> Result<Vec<ConflictRecord>> {
        let rows = sqlx::query_as::<_, ConflictRow>(
            "SELECT id, job_id, file_path, conflict_type, resolution, source_time, dest_time, created_at 
             FROM conflicts 
             WHERE job_id = ? AND resolution IS NULL
             ORDER BY created_at DESC"
        )
        .bind(job_id)
        .fetch_all(&*self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ConflictRecord {
                id: r.id,
                job_id: r.job_id,
                file_path: r.file_path,
                conflict_type: r.conflict_type,
                resolution: r.resolution,
                source_size: None,
                source_time: r.source_time,
                dest_size: None,
                dest_time: r.dest_time,
                created_at: r.created_at,
            })
            .collect())
    }

    /// 解决冲突
    pub async fn resolve_conflict(&self, id: i64, resolution: ConflictResolution) -> Result<()> {
        sqlx::query("UPDATE conflicts SET resolution = ? WHERE id = ?")
            .bind(resolution.to_string())
            .bind(id)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    /// 批量解决冲突
    pub async fn resolve_conflicts(
        &self,
        resolutions: Vec<(i64, ConflictResolution)>,
    ) -> Result<()> {
        for (id, resolution) in resolutions {
            self.resolve_conflict(id, resolution).await?;
        }
        Ok(())
    }

    /// 清理已解决的冲突记录
    pub async fn cleanup_resolved(&self, job_id: &str) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM conflicts WHERE job_id = ? AND resolution IS NOT NULL")
                .bind(job_id)
                .execute(&*self.db)
                .await?;

        Ok(result.rows_affected())
    }

    /// 获取默认解决策略
    pub fn resolve(
        &self,
        _path: &str,
        _conflict_type: &str,
        custom_resolution: Option<ConflictResolution>,
    ) -> ConflictResolution {
        custom_resolution.unwrap_or(self.default_resolution)
    }

    /// 生成冲突文件名
    pub fn generate_conflict_name(path: &str, side: &str, timestamp: i64) -> String {
        use chrono::DateTime;

        let dt = DateTime::from_timestamp(timestamp, 0)
            .map(|d| d.format("%Y%m%d_%H%M%S").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if let Some(ext_pos) = path.rfind('.') {
            let (name, ext) = path.split_at(ext_pos);
            format!("{}_conflict_{}_{}{}", name, side, dt, ext)
        } else {
            format!("{}_conflict_{}_{}", path, side, dt)
        }
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self {
            db: Arc::new(SqlitePool::connect_lazy("sqlite::memory:").unwrap()),
            default_resolution: ConflictResolution::Skip,
        }
    }
}
