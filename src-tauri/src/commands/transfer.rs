//! 传输配置相关命令

use crate::config::TransferConfig;
use crate::AppState;
use tauri::State;

/// 获取传输配置
#[tauri::command]
pub async fn get_transfer_config(state: State<'_, AppState>) -> Result<TransferConfig, String> {
    Ok(TransferConfig::load(&state.config_dir))
}

/// 设置传输配置
#[tauri::command]
pub async fn set_transfer_config(
    chunk_size_mb: Option<u64>,
    stream_threshold_mb: Option<u64>,
    state: State<'_, AppState>,
) -> Result<TransferConfig, String> {
    let mut config = TransferConfig::load(&state.config_dir);
    
    if let Some(size) = chunk_size_mb {
        config.chunk_size_mb = size;
    }
    if let Some(threshold) = stream_threshold_mb {
        config.stream_threshold_mb = threshold;
    }
    
    config.save(&state.config_dir).map_err(|e| e.to_string())?;
    
    Ok(config)
}
