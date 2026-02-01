//! 日志模块 - 提供文件日志和大小管理功能

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogConfig {
    /// 是否启用日志记录
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 最大日志文件大小（MB）
    #[serde(default = "default_max_size_mb")]
    pub max_size_mb: u32,
    /// 日志级别: "error", "warn", "info", "debug", "trace"
    #[serde(default = "default_level")]
    pub level: String,
}

fn default_enabled() -> bool {
    true
}

fn default_max_size_mb() -> u32 {
    5 // 默认 5MB
}

fn default_level() -> String {
    "info".to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            max_size_mb: default_max_size_mb(),
            level: default_level(),
        }
    }
}

impl LogConfig {
    /// 从配置文件加载日志配置
    pub fn load(config_dir: &Path) -> Self {
        let config_file = config_dir.join("config.json");
        if config_file.exists() {
            if let Ok(content) = fs::read_to_string(&config_file) {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(log_config) = config.get("log") {
                        if let Ok(log) = serde_json::from_value::<LogConfig>(log_config.clone()) {
                            return log;
                        }
                    }
                }
            }
        }
        Self::default()
    }

    /// 保存日志配置
    pub fn save(&self, config_dir: &Path) -> io::Result<()> {
        let config_file = config_dir.join("config.json");
        
        // 读取现有配置
        let mut config: serde_json::Value = if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
        
        // 更新日志配置
        config["log"] = serde_json::to_value(self).unwrap();
        
        // 写入文件
        fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap())
    }

    /// 将配置的日志级别转换为 tracing Level
    pub fn tracing_level(&self) -> tracing::Level {
        match self.level.to_lowercase().as_str() {
            "error" => tracing::Level::ERROR,
            "warn" => tracing::Level::WARN,
            "debug" => tracing::Level::DEBUG,
            "trace" => tracing::Level::TRACE,
            _ => tracing::Level::INFO,
        }
    }
}

/// 带大小限制的日志写入器
pub struct SizeRotatingWriter {
    file_path: PathBuf,
    max_size: u64,
    writer: Arc<Mutex<Option<BufWriter<File>>>>,
}

impl SizeRotatingWriter {
    pub fn new(log_dir: &Path, max_size_mb: u32) -> io::Result<Self> {
        fs::create_dir_all(log_dir)?;
        
        let file_path = log_dir.join("app.log");
        let max_size = (max_size_mb as u64) * 1024 * 1024;
        
        let writer = Self::open_file(&file_path, max_size)?;
        
        Ok(Self {
            file_path,
            max_size,
            writer: Arc::new(Mutex::new(Some(writer))),
        })
    }
    
    fn open_file(file_path: &Path, max_size: u64) -> io::Result<BufWriter<File>> {
        // 检查现有文件大小，如果超过限制则轮转
        if file_path.exists() {
            if let Ok(metadata) = fs::metadata(file_path) {
                if metadata.len() > max_size {
                    Self::rotate_log(file_path)?;
                }
            }
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;
        
        Ok(BufWriter::new(file))
    }
    
    /// 轮转日志文件
    fn rotate_log(file_path: &Path) -> io::Result<()> {
        // 创建备份文件名 app.log.old
        let backup_path = file_path.with_extension("log.old");
        
        // 如果备份已存在，删除它
        if backup_path.exists() {
            fs::remove_file(&backup_path)?;
        }
        
        // 重命名当前日志为备份
        fs::rename(file_path, &backup_path)?;
        
        Ok(())
    }
    
    /// 检查并轮转日志
    fn check_and_rotate(&self) -> io::Result<()> {
        if self.file_path.exists() {
            if let Ok(metadata) = fs::metadata(&self.file_path) {
                if metadata.len() > self.max_size {
                    // 需要轮转
                    let mut writer_guard = self.writer.lock().unwrap();
                    
                    // 关闭当前写入器
                    if let Some(mut w) = writer_guard.take() {
                        let _ = w.flush();
                    }
                    
                    // 轮转文件
                    Self::rotate_log(&self.file_path)?;
                    
                    // 重新打开
                    let new_writer = Self::open_file(&self.file_path, self.max_size)?;
                    *writer_guard = Some(new_writer);
                }
            }
        }
        Ok(())
    }
}

impl Clone for SizeRotatingWriter {
    fn clone(&self) -> Self {
        Self {
            file_path: self.file_path.clone(),
            max_size: self.max_size,
            writer: self.writer.clone(),
        }
    }
}

/// 日志写入器包装
pub struct LogWriter {
    inner: Arc<Mutex<Option<BufWriter<File>>>>,
    file_path: PathBuf,
    max_size: u64,
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self.inner.lock().unwrap();
        
        if let Some(ref mut writer) = *guard {
            let result = writer.write(buf)?;
            writer.flush()?;
            
            // 检查文件大小
            drop(guard);
            if self.file_path.exists() {
                if let Ok(metadata) = fs::metadata(&self.file_path) {
                    if metadata.len() > self.max_size {
                        // 重新获取锁进行轮转
                        let mut guard = self.inner.lock().unwrap();
                        if let Some(mut w) = guard.take() {
                            let _ = w.flush();
                        }
                        
                        let _ = SizeRotatingWriter::rotate_log(&self.file_path);
                        
                        if let Ok(new_writer) = SizeRotatingWriter::open_file(&self.file_path, self.max_size) {
                            *guard = Some(new_writer);
                        }
                    }
                }
            }
            
            Ok(result)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Writer not available"))
        }
    }
    
    fn flush(&mut self) -> io::Result<()> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(ref mut writer) = *guard {
            writer.flush()
        } else {
            Ok(())
        }
    }
}

impl<'a> MakeWriter<'a> for SizeRotatingWriter {
    type Writer = LogWriter;
    
    fn make_writer(&'a self) -> Self::Writer {
        // 在创建写入器前检查轮转
        let _ = self.check_and_rotate();
        
        LogWriter {
            inner: self.writer.clone(),
            file_path: self.file_path.clone(),
            max_size: self.max_size,
        }
    }
}

/// 获取日志目录路径（跟随数据存储位置）
pub fn get_log_dir() -> PathBuf {
    // 获取默认配置目录
    let default_config_dir = crate::dirs::config_dir()
        .map(|p| p.join("synctools"))
        .unwrap_or_else(|| PathBuf::from(".synctools"));
    
    // 尝试读取自定义数据路径
    let config_file = default_config_dir.join("config.json");
    if config_file.exists() {
        if let Ok(content) = fs::read_to_string(&config_file) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(custom_path) = config.get("data_path").and_then(|v| v.as_str()) {
                    let custom_dir = PathBuf::from(custom_path);
                    if custom_dir.exists() && custom_dir.is_dir() {
                        return custom_dir;
                    }
                }
            }
        }
    }
    
    default_config_dir
}
