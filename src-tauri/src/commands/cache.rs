//! 缓存相关命令

use crate::config::CacheConfig;
use crate::AppState;
use tauri::State;

/// 获取缓存配置
#[tauri::command]
pub async fn get_cache_config(state: State<'_, AppState>) -> Result<CacheConfig, String> {
    Ok(CacheConfig::load(&state.config_dir))
}

/// 设置缓存配置
#[tauri::command]
pub async fn set_cache_config(
    remote_ttl: Option<u64>,
    state: State<'_, AppState>,
) -> Result<CacheConfig, String> {
    let mut config = CacheConfig::load(&state.config_dir);
    
    if let Some(ttl) = remote_ttl {
        config.remote_ttl = ttl;
    }
    
    config.save(&state.config_dir).map_err(|e| e.to_string())?;
    
    Ok(config)
}
