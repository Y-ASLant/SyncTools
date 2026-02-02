use super::{FileInfo, FileMeta, Storage, IO_TIMEOUT_SECS, OP_TIMEOUT_SECS};
use anyhow::Result;
use async_trait::async_trait;
use futures::TryStreamExt;
use opendal::{layers::TimeoutLayer, Metakey, Operator};
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// HTTP 连接超时（秒）
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 30;
/// 目录缓存最大条目数（防止内存泄漏）
const MAX_DIR_CACHE_SIZE: usize = 10000;

pub struct WebDavStorage {
    operator: Operator,
    /// 复用的 HTTP 客户端（连接池）
    http_client: reqwest::Client,
    /// 已创建的目录缓存（避免重复创建导致 423 Locked）
    created_dirs: Arc<RwLock<HashSet<String>>>,
    name: String,
    endpoint: String,
    username: String,
    password: String,
}

impl WebDavStorage {
    pub async fn new(
        endpoint: &str,
        username: &str,
        password: &str,
        root: Option<String>,
    ) -> Result<Self> {
        use opendal::services::Webdav;

        // 如果有 root 路径，将其拼接到 endpoint 中（避免 OpenDAL 的 URL 编码问题）
        let final_endpoint = if let Some(ref r) = root {
            if !r.is_empty() {
                // 把 root 路径拼接到 endpoint 中
                let trimmed_endpoint = endpoint.trim_end_matches('/');
                let trimmed_root = r.trim_start_matches('/').trim_end_matches('/');
                format!("{}/{}", trimmed_endpoint, trimmed_root)
            } else {
                endpoint.to_string()
            }
        } else {
            endpoint.to_string()
        };

        let builder = Webdav::default()
            .endpoint(&final_endpoint)
            .username(username)
            .password(password);

        // 添加超时层
        let operator = Operator::new(builder)?
            .layer(
                TimeoutLayer::default()
                    .with_timeout(Duration::from_secs(OP_TIMEOUT_SECS))
                    .with_io_timeout(Duration::from_secs(IO_TIMEOUT_SECS))
            )
            .finish();

        let name = format!("webdav://{}", final_endpoint.trim_start_matches("https://").trim_start_matches("http://"));

        // 创建复用的 HTTP 客户端，带超时设置（用于流式传输）
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(IO_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
            .pool_max_idle_per_host(4)
            .build()?;

        // 尝试创建根目录（某些 WebDAV 服务器需要）
        // 忽略错误，目录可能已存在或不需要创建
        let _ = operator.create_dir("/").await;

        // 初始化目录缓存，根目录已存在
        let mut initial_dirs = HashSet::new();
        initial_dirs.insert("/".to_string());

        Ok(Self {
            operator,
            http_client,
            created_dirs: Arc::new(RwLock::new(initial_dirs)),
            name,
            endpoint: endpoint.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        })
    }

    /// 规范化路径：统一使用正斜杠，去除前导斜杠
    #[inline]
    fn normalize_path(path: &str) -> String {
        path.replace('\\', "/").trim_start_matches('/').to_string()
    }

    /// 确保目录存在（带缓存，避免重复创建）
    async fn ensure_parent_dirs(&self, file_path: &str) -> Result<()> {
        let path = file_path.replace('\\', "/");
        let path = path.trim_start_matches('/');
        
        if let Some(parent) = std::path::Path::new(path).parent() {
            let parent_str = parent.to_string_lossy().replace('\\', "/");
            if parent_str.is_empty() || parent_str == "." {
                return Ok(());
            }
            
            // 收集需要创建的目录
            let parts: Vec<&str> = parent_str.split('/').filter(|s| !s.is_empty()).collect();
            let mut dirs_to_create = Vec::new();
            let mut current_path = String::new();
            
            {
                let cache = self.created_dirs.read().await;
                for part in &parts {
                    current_path.push_str(part);
                    current_path.push('/');
                    if !cache.contains(&current_path) {
                        dirs_to_create.push(current_path.clone());
                    }
                }
            }
            
            // 只创建不在缓存中的目录
            if !dirs_to_create.is_empty() {
                let mut cache = self.created_dirs.write().await;
                
                // 缓存大小限制检查，防止内存泄漏
                if cache.len() >= MAX_DIR_CACHE_SIZE {
                    tracing::debug!("目录缓存已满({})，清空重建", MAX_DIR_CACHE_SIZE);
                    cache.clear();
                }
                
                for dir in dirs_to_create {
                    if !cache.contains(&dir) {
                        // 忽略创建目录的错误（可能已存在或并发创建）
                        let _ = self.operator.create_dir(&dir).await;
                        cache.insert(dir);
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl Storage for WebDavStorage {
    async fn list_files(&self, prefix: Option<&str>) -> Result<Vec<FileInfo>> {
        let mut files = Vec::new();
        let path = prefix.unwrap_or("");

        // 使用 lister_with 进行递归列表
        let mut lister = self
            .operator
            .lister_with(path)
            .recursive(true)
            .metakey(Metakey::ContentLength | Metakey::LastModified | Metakey::Mode)
            .await?;

        while let Some(entry) = lister.try_next().await? {
            let path_str = entry.path().to_string();

            // 跳过根目录
            if path_str.is_empty() || path_str == "/" {
                continue;
            }

            let meta = entry.metadata();

            files.push(FileInfo {
                path: path_str.trim_start_matches('/').to_string(),
                size: meta.content_length(),
                modified_time: meta.last_modified().map_or(0, |t| t.timestamp()),
                is_dir: meta.is_dir(),
                checksum: meta.etag().map(|s| s.trim_matches('"').to_string()),
            });
        }

        Ok(files)
    }

    async fn stat(&self, path: &str) -> Result<Option<FileMeta>> {
        let normalized_path = Self::normalize_path(path);
        match self.operator.stat(&normalized_path).await {
            Ok(meta) => Ok(Some(FileMeta {
                size: meta.content_length(),
                modified_time: meta.last_modified().map_or(0, |t| t.timestamp()),
                is_dir: meta.is_dir(),
                etag: meta.etag().map(|s| s.trim_matches('"').to_string()),
            })),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>> {
        // 规范化路径，移除可能的前缀（如 webdav/Sync/...）
        let normalized_path = Self::normalize_path(path);
        let data = self.operator.read(&normalized_path).await?;
        Ok(data.to_vec())
    }

    async fn read_range(&self, path: &str, offset: u64, length: u64) -> Result<Vec<u8>> {
        // 规范化路径
        let normalized_path = Self::normalize_path(path);
        let data = self
            .operator
            .read_with(&normalized_path)
            .range(offset..offset + length)
            .await?;
        Ok(data.to_vec())
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> Result<()> {
        // 规范化路径
        let normalized_path = Self::normalize_path(path);
        
        // 确保父目录存在（使用缓存避免重复创建）
        self.ensure_parent_dirs(&normalized_path).await?;
        
        self.operator.write(&normalized_path, data).await?;
        Ok(())
    }
    
    async fn write_stream(
        &self,
        path: &str,
        stream: Pin<Box<dyn futures::Stream<Item = Result<Vec<u8>>> + Send>>,
        total_size: Option<u64>,
    ) -> Result<()> {
        use futures::StreamExt;
        use reqwest::Body;
        
        // 规范化路径（去除前导斜杠，避免双斜杠）
        let path_normalized = Self::normalize_path(path);
        
        // 确保父目录存在（使用缓存避免重复创建）
        self.ensure_parent_dirs(&path_normalized).await?;
        
        // 使用复用的 HTTP 客户端进行流式 PUT 请求（绕过 OpenDAL 限制）
        let url = if path_normalized.is_empty() {
            self.endpoint.trim_end_matches('/').to_string()
        } else {
            format!("{}/{}", self.endpoint.trim_end_matches('/'), path_normalized)
        };
        
        // 将 Stream<Result<Vec<u8>>> 转换为 Stream<Result<Bytes>>
        let bytes_stream = stream.map(|result| {
            result.map(|vec| bytes::Bytes::from(vec))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        });
        
        let body = Body::wrap_stream(bytes_stream);
        
        // 使用复用的客户端（连接池）
        let mut request = self.http_client.put(&url).body(body);
        
        // 添加认证
        request = request.basic_auth(&self.username, Some(&self.password));
        
        // 如果知道大小，添加 Content-Length
        if let Some(size) = total_size {
            request = request.header("Content-Length", size.to_string());
        }
        
        let response = request.send().await
            .map_err(|e| anyhow::anyhow!("WebDAV 请求失败: {}", e))?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "WebDAV PUT 失败: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }
        
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let normalized_path = Self::normalize_path(path);
        match self.operator.delete(&normalized_path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        let normalized_path = Self::normalize_path(path);
        let dir_path = if normalized_path.ends_with('/') {
            normalized_path
        } else {
            format!("{}/", normalized_path)
        };
        self.operator.create_dir(&dir_path).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
