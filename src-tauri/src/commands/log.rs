//! 日志相关命令

use crate::logging::LogConfig;
use crate::AppState;
use tauri::State;

/// 获取日志配置
#[tauri::command]
pub async fn get_log_config(state: State<'_, AppState>) -> Result<LogConfig, String> {
    Ok(LogConfig::load(&state.config_dir))
}

/// 设置日志配置
#[tauri::command]
pub async fn set_log_config(
    enabled: Option<bool>,
    max_size_mb: Option<u32>,
    level: Option<String>,
    state: State<'_, AppState>,
) -> Result<LogConfig, String> {
    let mut config = LogConfig::load(&state.config_dir);
    
    if let Some(e) = enabled {
        config.enabled = e;
    }
    if let Some(size) = max_size_mb {
        // 限制范围 1-100 MB
        config.max_size_mb = size.clamp(1, 100);
    }
    if let Some(l) = level {
        // 验证日志级别
        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if valid_levels.contains(&l.to_lowercase().as_str()) {
            config.level = l.to_lowercase();
        } else {
            return Err(format!("无效的日志级别: {}", l));
        }
    }
    
    config.save(&state.config_dir).map_err(|e| e.to_string())?;
    
    Ok(config)
}
