//! 应用配置模块

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    /// 远程存储缓存 TTL（秒），0 表示永不过期
    #[serde(default = "default_remote_ttl")]
    pub remote_ttl: u64,
}

fn default_remote_ttl() -> u64 {
    1800 // 默认 30 分钟
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            remote_ttl: default_remote_ttl(),
        }
    }
}

/// 传输配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferConfig {
    /// 分块大小（MB），默认 8
    #[serde(default = "default_chunk_size")]
    pub chunk_size_mb: u64,
    /// 启用流式传输的阈值（MB），默认 128
    #[serde(default = "default_stream_threshold")]
    pub stream_threshold_mb: u64,
}

fn default_chunk_size() -> u64 {
    8 // 默认 8MB
}

fn default_stream_threshold() -> u64 {
    128 // 默认 128MB
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            chunk_size_mb: default_chunk_size(),
            stream_threshold_mb: default_stream_threshold(),
        }
    }
}

impl TransferConfig {
    /// 从配置文件加载传输配置
    pub fn load(config_dir: &Path) -> Self {
        let config_file = config_dir.join("config.json");
        if config_file.exists() {
            if let Ok(content) = fs::read_to_string(&config_file) {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(transfer_config) = config.get("transfer") {
                        if let Ok(transfer) = serde_json::from_value::<TransferConfig>(transfer_config.clone()) {
                            return transfer;
                        }
                    }
                }
            }
        }
        Self::default()
    }

    /// 保存传输配置
    pub fn save(&self, config_dir: &Path) -> io::Result<()> {
        let config_file = config_dir.join("config.json");
        
        // 读取现有配置
        let mut config: serde_json::Value = if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
        
        // 更新传输配置
        config["transfer"] = serde_json::to_value(self).unwrap();
        
        // 写入文件
        let content = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_file, content)?;
        
        Ok(())
    }
}

impl CacheConfig {
    /// 从配置文件加载缓存配置
    pub fn load(config_dir: &Path) -> Self {
        let config_file = config_dir.join("config.json");
        if config_file.exists() {
            if let Ok(content) = fs::read_to_string(&config_file) {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(cache_config) = config.get("cache") {
                        if let Ok(cache) = serde_json::from_value::<CacheConfig>(cache_config.clone()) {
                            return cache;
                        }
                    }
                }
            }
        }
        Self::default()
    }

    /// 保存缓存配置
    pub fn save(&self, config_dir: &Path) -> io::Result<()> {
        let config_file = config_dir.join("config.json");
        
        // 读取现有配置
        let mut config: serde_json::Value = if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
        
        // 更新缓存配置
        config["cache"] = serde_json::to_value(self).unwrap();
        
        // 写入文件
        let content = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_file, content)?;
        
        Ok(())
    }
}
