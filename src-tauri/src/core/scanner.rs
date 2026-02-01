use crate::storage::{FileInfo, Storage};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

/// 文件扫描器配置
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// 是否包含目录
    pub include_dirs: bool,
    /// 排除规则（glob patterns）
    pub exclude_patterns: Vec<String>,
    /// 最大文件大小（0 表示不限制）
    pub max_file_size: u64,
    /// 仅包含的扩展名（空表示不限制）
    pub include_extensions: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            include_dirs: false,
            exclude_patterns: vec![
                // 常见的排除模式
                ".git/**".to_string(),
                ".svn/**".to_string(),
                "node_modules/**".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                "*.tmp".to_string(),
                "*.temp".to_string(),
                "~*".to_string(),
            ],
            max_file_size: 0,
            include_extensions: vec![],
        }
    }
}

/// 文件扫描器
pub struct FileScanner {
    max_concurrent: usize,
    config: ScanConfig,
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl FileScanner {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            config: ScanConfig::default(),
            cancel_flag: None,
        }
    }

    pub fn with_config(max_concurrent: usize, config: ScanConfig) -> Self {
        Self {
            max_concurrent,
            config,
            cancel_flag: None,
        }
    }

    /// 创建带取消标志的扫描器
    pub fn with_cancel(cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            max_concurrent: 8,
            config: ScanConfig::default(),
            cancel_flag: Some(cancel_flag),
        }
    }

    /// 检查是否已取消
    fn is_cancelled(&self) -> bool {
        self.cancel_flag
            .as_ref()
            .map(|f| f.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    /// 检查路径是否应该被排除
    fn should_exclude(&self, path: &str) -> bool {
        for pattern in &self.config.exclude_patterns {
            if self.matches_pattern(path, pattern) {
                return true;
            }
        }

        // 检查文件大小限制
        if self.config.max_file_size > 0 {
            // 这个在扫描时需要检查
        }

        // 检查扩展名
        if !self.config.include_extensions.is_empty() {
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            if !self
                .config
                .include_extensions
                .iter()
                .any(|e| e.to_lowercase() == ext)
            {
                return true;
            }
        }

        false
    }

    /// 简单的 glob 模式匹配
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        let path = path.to_lowercase();
        let pattern = pattern.to_lowercase();

        // 处理 ** 通配符
        if pattern.contains("**") {
            let parts: Vec<&str> = pattern.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_end_matches('/');
                let suffix = parts[1].trim_start_matches('/');

                if prefix.is_empty() && suffix.is_empty() {
                    return true;
                }

                if !prefix.is_empty() && !path.starts_with(prefix) {
                    return false;
                }

                if !suffix.is_empty() && !path.ends_with(suffix) {
                    return false;
                }

                return true;
            }
        }

        // 处理 * 通配符
        if pattern.contains('*') {
            let regex_pattern = pattern.replace('.', "\\.").replace('*', ".*");

            if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                return re.is_match(&path);
            }
        }

        // 精确匹配
        path == pattern || path.ends_with(&format!("/{}", pattern))
    }

    /// 扫描存储并返回文件树
    pub async fn scan_storage(
        &self,
        storage: &dyn Storage,
        prefix: Option<&str>,
    ) -> Result<HashMap<String, FileInfo>> {
        // 检查是否已取消
        if self.is_cancelled() {
            return Err(anyhow::anyhow!("操作已取消"));
        }

        info!("开始扫描存储: {}, prefix: {:?}", storage.name(), prefix);

        let files = storage.list_files(prefix).await?;
        info!("list_files 返回 {} 个条目", files.len());

        // 检查是否已取消
        if self.is_cancelled() {
            return Err(anyhow::anyhow!("操作已取消"));
        }

        let mut tree = HashMap::new();
        let mut excluded_count = 0;
        let mut dir_count = 0;

        for file in files {
            // 每处理一定数量检查一次取消状态
            if tree.len() % 100 == 0 && self.is_cancelled() {
                return Err(anyhow::anyhow!("操作已取消"));
            }

            // 跳过目录（除非配置要求包含）
            if file.is_dir && !self.config.include_dirs {
                dir_count += 1;
                continue;
            }

            // 检查排除规则
            if self.should_exclude(&file.path) {
                debug!("排除文件: {}", file.path);
                excluded_count += 1;
                continue;
            }

            // 检查文件大小
            if self.config.max_file_size > 0 && file.size > self.config.max_file_size {
                debug!("跳过大文件: {} ({})", file.path, file.size);
                excluded_count += 1;
                continue;
            }

            tree.insert(file.path.clone(), file);
        }

        info!(
            "扫描完成: {} 个文件, {} 个目录, {} 个被排除",
            tree.len(),
            dir_count,
            excluded_count
        );

        Ok(tree)
    }

    /// 并发扫描多个路径
    pub async fn scan_paths(
        &self,
        storage: Arc<dyn Storage>,
        paths: Vec<String>,
    ) -> Result<HashMap<String, FileInfo>> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let mut handles = Vec::new();

        for path in paths {
            let permit = semaphore.clone().acquire_owned().await?;
            let storage = storage.clone();
            let scanner_config = self.config.clone();

            let handle = tokio::spawn(async move {
                let scanner = FileScanner::with_config(1, scanner_config);
                let result = scanner.scan_storage(storage.as_ref(), Some(&path)).await;
                drop(permit);
                result
            });

            handles.push(handle);
        }

        let mut combined = HashMap::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(files)) => combined.extend(files),
                Ok(Err(e)) => warn!("扫描路径失败: {}", e),
                Err(e) => warn!("任务失败: {}", e),
            }
        }

        Ok(combined)
    }
}

impl Default for FileScanner {
    fn default() -> Self {
        Self {
            max_concurrent: 8,
            config: ScanConfig::default(),
            cancel_flag: None,
        }
    }
}
