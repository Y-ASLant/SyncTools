use super::{FileInfo, FileMeta, Storage};
use anyhow::Result;
use async_trait::async_trait;
use futures::TryStreamExt;
use opendal::{Metakey, Operator};

pub struct WebDavStorage {
    operator: Operator,
    name: String,
}

impl WebDavStorage {
    pub async fn new(
        endpoint: &str,
        username: &str,
        password: &str,
        root: Option<String>,
    ) -> Result<Self> {
        use opendal::services::Webdav;

        let mut builder = Webdav::default()
            .endpoint(endpoint)
            .username(username)
            .password(password);

        if let Some(ref r) = root {
            builder = builder.root(r);
        }

        let operator = Operator::new(builder)?.finish();

        let name = format!(
            "webdav://{}{}",
            endpoint.trim_end_matches('/'),
            root.as_deref()
                .map(|r| format!("/{}", r.trim_start_matches('/')))
                .unwrap_or_default()
        );

        // 尝试创建根目录（某些 WebDAV 服务器需要）
        // 忽略错误，目录可能已存在或不需要创建
        let _ = operator.create_dir("/").await;

        Ok(Self { operator, name })
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
        match self.operator.stat(path).await {
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
        let data = self.operator.read(path).await?;
        Ok(data.to_vec())
    }

    async fn read_range(&self, path: &str, offset: u64, length: u64) -> Result<Vec<u8>> {
        let data = self
            .operator
            .read_with(path)
            .range(offset..offset + length)
            .await?;
        Ok(data.to_vec())
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> Result<()> {
        // 规范化路径
        let path = path.replace('\\', "/");
        let path = path.trim_start_matches('/');
        
        // 确保父目录存在（递归创建）
        if let Some(parent) = std::path::Path::new(path).parent() {
            let parent_str = parent.to_string_lossy().replace('\\', "/");
            if !parent_str.is_empty() && parent_str != "." {
                // 递归创建所有父目录
                let parts: Vec<&str> = parent_str.split('/').filter(|s| !s.is_empty()).collect();
                let mut current_path = String::new();
                for part in parts {
                    current_path.push_str(part);
                    current_path.push('/');
                    // 忽略创建目录的错误（可能已存在）
                    let _ = self.operator.create_dir(&current_path).await;
                }
            }
        }
        
        self.operator.write(path, data).await?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        match self.operator.delete(path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        let dir_path = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };
        self.operator.create_dir(&dir_path).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
