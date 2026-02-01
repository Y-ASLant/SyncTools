use super::{FileInfo, FileMeta, Storage};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use walkdir::WalkDir;

pub struct LocalStorage {
    base_path: PathBuf,
    name: String,
}

impl LocalStorage {
    pub fn new(path: &str) -> Result<Self> {
        let base_path = PathBuf::from(path);
        if !base_path.exists() {
            std::fs::create_dir_all(&base_path)?;
        }
        let name = format!("local:{}", path);
        Ok(Self { base_path, name })
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        let path = path.trim_start_matches('/').trim_start_matches('\\');
        if path.is_empty() {
            self.base_path.clone()
        } else {
            self.base_path.join(path)
        }
    }

    /// 规范化路径分隔符（统一使用 /）
    fn normalize_path(path: &str) -> String {
        path.replace('\\', "/")
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn list_files(&self, prefix: Option<&str>) -> Result<Vec<FileInfo>> {
        let base = prefix.map_or_else(|| self.base_path.clone(), |p| self.resolve_path(p));

        if !base.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        let base_path = self.base_path.clone();

        // 使用 spawn_blocking 避免阻塞 async runtime
        let entries: Vec<_> = tokio::task::spawn_blocking(move || {
            WalkDir::new(&base)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    let metadata = entry.metadata().ok()?;

                    let relative_path = path.strip_prefix(&base_path).ok()?.to_str()?.to_string();

                    // 跳过根目录本身
                    if relative_path.is_empty() {
                        return None;
                    }

                    let modified = metadata
                        .modified()
                        .ok()?
                        .duration_since(std::time::UNIX_EPOCH)
                        .ok()?
                        .as_secs() as i64;

                    Some(FileInfo {
                        path: Self::normalize_path(&relative_path),
                        size: if metadata.is_dir() { 0 } else { metadata.len() },
                        modified_time: modified,
                        is_dir: metadata.is_dir(),
                        checksum: None,
                    })
                })
                .collect()
        })
        .await?;

        files.extend(entries);
        Ok(files)
    }

    async fn stat(&self, path: &str) -> Result<Option<FileMeta>> {
        let full_path = self.resolve_path(path);

        match fs::metadata(&full_path).await {
            Ok(metadata) => {
                let modified = metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs() as i64;

                Ok(Some(FileMeta {
                    size: if metadata.is_dir() { 0 } else { metadata.len() },
                    modified_time: modified,
                    is_dir: metadata.is_dir(),
                    etag: None,
                }))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>> {
        let data = fs::read(self.resolve_path(path)).await?;
        Ok(data)
    }

    async fn read_range(&self, path: &str, offset: u64, length: u64) -> Result<Vec<u8>> {
        let full_path = self.resolve_path(path);
        let mut file = fs::File::open(&full_path).await?;

        file.seek(std::io::SeekFrom::Start(offset)).await?;

        let mut buffer = vec![0u8; length as usize];
        let bytes_read = file.read_exact(&mut buffer).await;

        match bytes_read {
            Ok(_) => Ok(buffer),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // 文件剩余内容不足 length，读取实际可用的数据
                file.seek(std::io::SeekFrom::Start(offset)).await?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).await?;
                Ok(buffer)
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> Result<()> {
        let full_path = self.resolve_path(path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // 使用临时文件写入，然后原子重命名
        let temp_path = full_path.with_extension("tmp");
        fs::write(&temp_path, data).await?;
        fs::rename(&temp_path, &full_path).await?;

        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);

        if !full_path.exists() {
            return Ok(());
        }

        if full_path.is_dir() {
            fs::remove_dir_all(&full_path).await?;
        } else {
            fs::remove_file(&full_path).await?;
        }

        Ok(())
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::create_dir_all(&full_path).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
