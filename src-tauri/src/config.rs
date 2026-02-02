//! 应用配置模块

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

// ============================================================================
// 常量定义
// ============================================================================

/// 配置文件名
const CONFIG_FILE_NAME: &str = "config.json";
/// 默认远程缓存 TTL（秒，30分钟）
const DEFAULT_REMOTE_TTL: u64 = 1800;
/// 默认分块大小（MB）
const DEFAULT_CHUNK_SIZE_MB: u64 = 8;
/// 默认流式传输阈值（MB）
const DEFAULT_STREAM_THRESHOLD_MB: u64 = 128;

// ============================================================================
// 通用配置加载/保存工具
// ============================================================================

/// 从配置文件加载指定 section 的配置
fn load_config_section<T: DeserializeOwned + Default>(config_dir: &Path, section: &str) -> T {
    let config_file = config_dir.join(CONFIG_FILE_NAME);
    if config_file.exists() {
        if let Ok(content) = fs::read_to_string(&config_file) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(section_config) = config.get(section) {
                    if let Ok(parsed) = serde_json::from_value::<T>(section_config.clone()) {
                        return parsed;
                    }
                }
            }
        }
    }
    T::default()
}

/// 保存配置到指定 section（原子写入，防止并发丢失）
fn save_config_section<T: Serialize>(config_dir: &Path, section: &str, value: &T) -> io::Result<()> {
    let config_file = config_dir.join(CONFIG_FILE_NAME);
    
    // 确保配置目录存在
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    
    // 读取现有配置
    let mut config: serde_json::Value = if config_file.exists() {
        let content = fs::read_to_string(&config_file)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    // 更新指定 section
    config[section] = serde_json::to_value(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    
    // 序列化配置
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    
    // 原子写入：先写临时文件，再重命名
    let temp_file = config_dir.join(format!("{}.tmp", CONFIG_FILE_NAME));
    fs::write(&temp_file, &content)?;
    
    // Windows 上 rename 不会覆盖已存在的文件，需要先删除
    #[cfg(windows)]
    if config_file.exists() {
        let _ = fs::remove_file(&config_file);
    }
    
    // 重命名（在大多数文件系统上是原子操作）
    fs::rename(&temp_file, &config_file)?;
    
    Ok(())
}

// ============================================================================
// 缓存配置
// ============================================================================

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    /// 远程存储缓存 TTL（秒），0 表示永不过期
    #[serde(default = "default_remote_ttl")]
    pub remote_ttl: u64,
}

fn default_remote_ttl() -> u64 {
    DEFAULT_REMOTE_TTL
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            remote_ttl: DEFAULT_REMOTE_TTL,
        }
    }
}

impl CacheConfig {
    /// 从配置文件加载缓存配置
    pub fn load(config_dir: &Path) -> Self {
        load_config_section(config_dir, "cache")
    }

    /// 保存缓存配置
    pub fn save(&self, config_dir: &Path) -> io::Result<()> {
        save_config_section(config_dir, "cache", self)
    }
}

// ============================================================================
// 传输配置
// ============================================================================

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
    DEFAULT_CHUNK_SIZE_MB
}

fn default_stream_threshold() -> u64 {
    DEFAULT_STREAM_THRESHOLD_MB
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            chunk_size_mb: DEFAULT_CHUNK_SIZE_MB,
            stream_threshold_mb: DEFAULT_STREAM_THRESHOLD_MB,
        }
    }
}

impl TransferConfig {
    /// 从配置文件加载传输配置
    pub fn load(config_dir: &Path) -> Self {
        load_config_section(config_dir, "transfer")
    }

    /// 保存传输配置
    pub fn save(&self, config_dir: &Path) -> io::Result<()> {
        save_config_section(config_dir, "transfer", self)
    }
}
