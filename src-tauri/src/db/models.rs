#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

/// 存储类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    Local,
    S3,
    WebDav,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    #[serde(rename = "type")]
    pub typ: StorageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessKey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secretKey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webdavEndpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
}

/// 同步模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    Bidirectional,
    Mirror,
    Backup,
}

/// 同步状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Idle,
    Scanning,
    Comparing,
    Syncing,
    Completed,
    Failed,
    Cancelled,
}

/// 同步任务
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncJob {
    pub id: String,
    pub name: String,
    pub sourceConfig: StorageConfig,
    pub destConfig: StorageConfig,
    pub syncMode: SyncMode,
    pub schedule: Option<String>,
    pub enabled: bool,
    pub createdAt: i64,
    pub updatedAt: i64,
}

/// 同步进度
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub jobId: String,
    pub status: SyncStatus,
    pub phase: String,
    pub currentFile: String,
    pub filesScanned: u32,
    pub filesToSync: u32,
    pub filesCompleted: u32,
    pub filesSkipped: u32,
    pub filesFailed: u32,
    pub bytesTransferred: u64,
    pub bytesTotal: u64,
    pub speed: u64,
    pub startTime: i64,
    pub endTime: i64,  // 完成时间（0 表示未完成）
}

/// 同步报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SyncReport {
    pub job_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub status: SyncStatus,
    pub files_scanned: u32,
    pub files_copied: u32,
    pub files_deleted: u32,
    pub files_skipped: u32,
    pub files_failed: u32,
    pub bytes_transferred: u64,
    pub duration: u64,
    pub errors: Vec<String>,
}

// 数据库表模型
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SyncJobRow {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub source_config: String,
    pub dest_type: String,
    pub dest_config: String,
    pub sync_mode: String,
    pub schedule: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl TryFrom<SyncJobRow> for SyncJob {
    type Error = anyhow::Error;

    fn try_from(row: SyncJobRow) -> Result<Self, Self::Error> {
        // 兼容旧数据（可能带引号）和新数据
        let mode_str = row.sync_mode.trim_matches('"');
        let sync_mode = match mode_str {
            "bidirectional" => SyncMode::Bidirectional,
            "mirror" => SyncMode::Mirror,
            "backup" => SyncMode::Backup,
            _ => return Err(anyhow::anyhow!("Invalid sync mode: {}", row.sync_mode)),
        };

        let source_config: StorageConfig = serde_json::from_str(&row.source_config)?;
        let dest_config: StorageConfig = serde_json::from_str(&row.dest_config)?;

        Ok(SyncJob {
            id: row.id,
            name: row.name,
            sourceConfig: source_config,
            destConfig: dest_config,
            syncMode: sync_mode,
            schedule: row.schedule,
            enabled: row.enabled,
            createdAt: row.created_at,
            updatedAt: row.updated_at,
        })
    }
}
