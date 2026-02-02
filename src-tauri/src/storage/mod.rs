pub mod local;
pub mod s3;
pub mod webdav;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

pub use local::LocalStorage;
pub use s3::S3Storage;
pub use webdav::WebDavStorage;

// ============ 公共常量 ============

/// 非 IO 操作超时（秒）- stat, delete 等
pub const OP_TIMEOUT_SECS: u64 = 60;
/// IO 操作超时（秒）- read, write 等
pub const IO_TIMEOUT_SECS: u64 = 300;

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub modified_time: i64,
    pub is_dir: bool,
    pub checksum: Option<String>,
}

/// 文件元数据（用于快速检查）
#[derive(Debug, Clone)]
pub struct FileMeta {
    pub size: u64,
    pub modified_time: i64,
    pub is_dir: bool,
    pub etag: Option<String>,
}

/// 文件块（用于分块传输）
#[derive(Debug, Clone)]
pub struct FileChunk {
    pub data: Vec<u8>,
    pub offset: u64,
    pub size: usize,
}

/// 存储抽象接口
#[async_trait]
pub trait Storage: Send + Sync {
    /// 递归列出所有文件
    async fn list_files(&self, prefix: Option<&str>) -> Result<Vec<FileInfo>>;

    /// 获取文件元数据
    async fn stat(&self, path: &str) -> Result<Option<FileMeta>>;

    /// 读取整个文件
    async fn read(&self, path: &str) -> Result<Vec<u8>>;

    /// 读取文件的一部分（用于断点续传）
    async fn read_range(&self, path: &str, offset: u64, length: u64) -> Result<Vec<u8>>;

    /// 写入整个文件
    async fn write(&self, path: &str, data: Vec<u8>) -> Result<()>;

    /// 流式写入（用于大文件）
    async fn write_stream(
        &self,
        path: &str,
        mut stream: Pin<Box<dyn Stream<Item = Result<Vec<u8>>> + Send>>,
        _total_size: Option<u64>,
    ) -> Result<()> {
        // 默认实现：收集所有数据后写入
        use futures::StreamExt;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend(chunk?);
        }
        self.write(path, data).await
    }

    /// 删除文件或目录
    async fn delete(&self, path: &str) -> Result<()>;

    /// 检查文件是否存在
    async fn exists(&self, path: &str) -> Result<bool> {
        Ok(self.stat(path).await?.is_some())
    }

    /// 创建目录
    async fn create_dir(&self, path: &str) -> Result<()>;

    /// 复制文件（同一存储内）
    async fn copy(&self, from: &str, to: &str) -> Result<()> {
        let data = self.read(from).await?;
        self.write(to, data).await
    }

    /// 获取存储名称（用于日志）
    fn name(&self) -> &str;
}

/// 根据配置创建存储实例
pub async fn create_storage(
    config: &crate::db::StorageConfig,
) -> Result<std::sync::Arc<dyn Storage>> {
    match config.typ {
        crate::db::StorageType::Local => {
            let path = config
                .path
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Local storage requires path"))?;
            tracing::info!("初始化本地存储: {}", path);
            Ok(std::sync::Arc::new(LocalStorage::new(path)?) as std::sync::Arc<dyn Storage>)
        }
        crate::db::StorageType::S3 => {
            let bucket = config
                .bucket
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("S3 storage requires bucket"))?;
            let region = config
                .region
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("S3 storage requires region"))?;
            let access_key = config
                .accessKey
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("S3 storage requires accessKey"))?;
            let secret_key = config
                .secretKey
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("S3 storage requires secretKey"))?;
            tracing::info!("初始化S3存储: bucket={}, region={}", bucket, region);
            Ok(std::sync::Arc::new(
                S3Storage::new(
                    bucket,
                    region,
                    access_key,
                    secret_key,
                    config.endpoint.clone(),
                    config.prefix.clone(),
                )
                .await?,
            ) as std::sync::Arc<dyn Storage>)
        }
        crate::db::StorageType::WebDav => {
            let endpoint = config
                .webdavEndpoint
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("WebDAV storage requires endpoint"))?;
            let username = config
                .username
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("WebDAV storage requires username"))?;
            let password = config
                .password
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("WebDAV storage requires password"))?;
            tracing::info!("创建WebDAV存储: endpoint={}, root={:?}", endpoint, config.root);
            Ok(std::sync::Arc::new(
                WebDavStorage::new(endpoint, username, password, config.root.clone()).await?,
            ) as std::sync::Arc<dyn Storage>)
        }
    }
}
