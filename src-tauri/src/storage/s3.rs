use super::{FileInfo, FileMeta, Storage, IO_TIMEOUT_SECS, OP_TIMEOUT_SECS};
use anyhow::Result;
use async_trait::async_trait;
use futures::TryStreamExt;
use opendal::{layers::TimeoutLayer, Metakey, Operator};
use std::time::Duration;

pub struct S3Storage {
    operator: Operator,
    name: String,
}

impl S3Storage {
    pub async fn new(
        bucket: &str,
        region: &str,
        access_key: &str,
        secret_key: &str,
        endpoint: Option<String>,
        prefix: Option<String>,
    ) -> Result<Self> {
        use opendal::services::S3;

        let mut builder = S3::default()
            .bucket(bucket)
            .region(region)
            .access_key_id(access_key)
            .secret_access_key(secret_key);

        if let Some(ref ep) = endpoint {
            builder = builder.endpoint(ep);
        }

        if let Some(ref p) = prefix {
            builder = builder.root(p);
        }

        // 添加超时层
        let operator = Operator::new(builder)?
            .layer(
                TimeoutLayer::default()
                    .with_timeout(Duration::from_secs(OP_TIMEOUT_SECS))
                    .with_io_timeout(Duration::from_secs(IO_TIMEOUT_SECS))
            )
            .finish();

        let name = format!(
            "s3://{}{}",
            bucket,
            prefix
                .as_deref()
                .map(|p| format!("/{}", p))
                .unwrap_or_default()
        );

        Ok(Self { operator, name })
    }
}

#[async_trait]
impl Storage for S3Storage {
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
        self.operator.write(path, data).await?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        // S3 删除不存在的文件不会报错
        self.operator.delete(path).await?;
        Ok(())
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        // S3 不需要真正创建目录，但为了兼容性，创建一个占位对象
        let dir_path = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };
        self.operator.write(&dir_path, Vec::<u8>::new()).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
