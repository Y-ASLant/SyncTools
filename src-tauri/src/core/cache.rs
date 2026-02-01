//! 文件列表缓存
//! 
//! 用于缓存存储的文件列表，避免每次同步都重新扫描

use crate::storage::FileInfo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// 文件列表
    pub files: HashMap<String, FileInfo>,
    /// 缓存时间（Unix 时间戳）
    pub cached_at: u64,
    /// 存储配置哈希（用于判断配置是否变化）
    pub config_hash: String,
}

/// 缓存加载结果（包含文件列表和缓存时间）
#[derive(Debug, Clone)]
pub struct CacheResult {
    pub files: HashMap<String, FileInfo>,
    pub cached_at: u64,
}

/// 文件列表缓存管理器
pub struct FileListCache {
    cache_dir: PathBuf,
    /// 缓存有效期（秒），0 表示永不过期
    ttl_seconds: u64,
}

impl FileListCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        // 确保缓存目录存在
        let _ = std::fs::create_dir_all(&cache_dir);
        Self {
            cache_dir,
            ttl_seconds: 0, // 默认永不过期，直到手动刷新
        }
    }

    /// 设置缓存有效期（0 表示永不过期）
    pub fn with_ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = seconds;
        self
    }

    /// 获取缓存文件路径
    fn cache_path(&self, job_id: &str, storage_type: &str) -> PathBuf {
        self.cache_dir.join(format!("{}_{}.cache", job_id, storage_type))
    }

    /// 计算配置哈希
    fn hash_config(config: &str) -> String {
        let hash = blake3::hash(config.as_bytes());
        hash.to_hex()[..16].to_string()
    }

    /// 获取当前时间戳
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs()
    }

    /// 从缓存加载文件列表（返回文件列表和缓存时间）
    pub fn load(
        &self,
        job_id: &str,
        storage_type: &str,
        config_json: &str,
    ) -> Option<CacheResult> {
        let path = self.cache_path(job_id, storage_type);
        
        if !path.exists() {
            return None;
        }

        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(_) => return None,
        };

        let entry: CacheEntry = match serde_json::from_slice(&data) {
            Ok(e) => e,
            Err(_) => {
                // 缓存损坏，删除
                let _ = std::fs::remove_file(&path);
                return None;
            }
        };

        // 检查配置是否变化
        let current_hash = Self::hash_config(config_json);
        if entry.config_hash != current_hash {
            info!("缓存配置不匹配，清除缓存");
            let _ = std::fs::remove_file(&path);
            return None;
        }

        // 检查是否过期（ttl_seconds 为 0 表示永不过期）
        let now = Self::now();
        if self.ttl_seconds > 0 && now - entry.cached_at > self.ttl_seconds {
            info!("缓存已过期 ({}s)，清除缓存", now - entry.cached_at);
            let _ = std::fs::remove_file(&path);
            return None;
        }

        let age_str = Self::format_age(now - entry.cached_at);

        info!(
            "从缓存加载 {} 个文件 (缓存于 {})",
            entry.files.len(),
            age_str
        );

        Some(CacheResult {
            files: entry.files,
            cached_at: entry.cached_at,
        })
    }

    /// 格式化缓存时间
    pub fn format_age(age_seconds: u64) -> String {
        if age_seconds < 60 {
            format!("{}秒前", age_seconds)
        } else if age_seconds < 3600 {
            format!("{}分钟前", age_seconds / 60)
        } else if age_seconds < 86400 {
            format!("{}小时前", age_seconds / 3600)
        } else {
            format!("{}天前", age_seconds / 86400)
        }
    }

    /// 获取当前时间戳（公开方法）
    pub fn current_time() -> u64 {
        Self::now()
    }

    /// 保存文件列表到缓存
    pub fn save(
        &self,
        job_id: &str,
        storage_type: &str,
        config_json: &str,
        files: &HashMap<String, FileInfo>,
    ) -> Result<()> {
        let path = self.cache_path(job_id, storage_type);
        
        let entry = CacheEntry {
            files: files.clone(),
            cached_at: Self::now(),
            config_hash: Self::hash_config(config_json),
        };

        let data = serde_json::to_vec(&entry)?;
        std::fs::write(&path, data)?;

        info!("已缓存 {} 个文件到 {:?}", files.len(), path);

        Ok(())
    }

    /// 清除指定任务的缓存
    pub fn clear(&self, job_id: &str) {
        for storage_type in ["source", "dest"] {
            let path = self.cache_path(job_id, storage_type);
            let _ = std::fs::remove_file(&path);
        }
    }

    /// 清除所有缓存
    pub fn clear_all(&self) {
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map(|e| e == "cache").unwrap_or(false) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_config() {
        let hash1 = FileListCache::hash_config("config1");
        let hash2 = FileListCache::hash_config("config1");
        let hash3 = FileListCache::hash_config("config2");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
