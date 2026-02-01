#![allow(non_snake_case)]

pub mod models;
pub use models::*;

use anyhow::Result;
pub use sqlx::SqlitePool;

impl SyncJob {
    /// 从数据库加载所有任务
    pub async fn load_all(pool: &SqlitePool) -> Result<Vec<SyncJob>> {
        let rows =
            sqlx::query_as::<_, SyncJobRow>("SELECT * FROM sync_jobs ORDER BY created_at DESC")
                .fetch_all(pool)
                .await?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row.try_into()?);
        }
        Ok(jobs)
    }

    /// 从数据库加载单个任务
    pub async fn load(pool: &SqlitePool, id: &str) -> Result<Option<SyncJob>> {
        let row = sqlx::query_as::<_, SyncJobRow>("SELECT * FROM sync_jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    /// 保存到数据库
    pub async fn save(&self, pool: &SqlitePool) -> Result<()> {
        let source_config = serde_json::to_string(&self.sourceConfig)?;
        let dest_config = serde_json::to_string(&self.destConfig)?;
        let sync_mode = serde_json::to_string(&self.syncMode)?;

        sqlx::query(
            r#"
            INSERT INTO sync_jobs (id, name, source_type, source_config, dest_type, dest_config, sync_mode, schedule, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                source_type = excluded.source_type,
                source_config = excluded.source_config,
                dest_type = excluded.dest_type,
                dest_config = excluded.dest_config,
                sync_mode = excluded.sync_mode,
                schedule = excluded.schedule,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&self.id)
        .bind(&self.name)
        .bind(format!("{:?}", self.sourceConfig.typ).to_lowercase())
        .bind(&source_config)
        .bind(format!("{:?}", self.destConfig.typ).to_lowercase())
        .bind(&dest_config)
        .bind(&sync_mode)
        .bind(&self.schedule)
        .bind(self.enabled)
        .bind(self.createdAt)
        .bind(self.updatedAt)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 从数据库删除
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM sync_jobs WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 创建新任务
    pub fn new(
        name: String,
        sourceConfig: StorageConfig,
        destConfig: StorageConfig,
        syncMode: SyncMode,
        schedule: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            sourceConfig,
            destConfig,
            syncMode,
            schedule,
            enabled: true,
            createdAt: now,
            updatedAt: now,
        }
    }
}
