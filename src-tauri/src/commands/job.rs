#![allow(non_snake_case)]
#![allow(clippy::too_many_arguments)]

use crate::db::{StorageConfig, SyncJob, SyncMode};
use crate::AppState;
use tauri::State;

/// 解析同步模式
fn parse_sync_mode(mode: &str) -> Result<SyncMode, String> {
    match mode {
        "bidirectional" => Ok(SyncMode::Bidirectional),
        "mirror" => Ok(SyncMode::Mirror),
        "backup" => Ok(SyncMode::Backup),
        _ => Err(format!("无效的同步模式: {}", mode)),
    }
}

/// 解析存储配置
fn parse_storage_config(config: serde_json::Value, name: &str) -> Result<StorageConfig, String> {
    serde_json::from_value(config).map_err(|e| format!("无效的{}配置: {}", name, e))
}

/// 获取所有同步任务
#[tauri::command]
pub async fn get_jobs(state: State<'_, AppState>) -> Result<Vec<SyncJob>, String> {
    SyncJob::load_all(&state.db).await.map_err(|e| e.to_string())
}

/// 创建新的同步任务
#[tauri::command]
pub async fn create_job(
    name: String,
    sourceConfig: serde_json::Value,
    destConfig: serde_json::Value,
    syncMode: String,
    schedule: Option<String>,
    state: State<'_, AppState>,
) -> Result<SyncJob, String> {
    let source = parse_storage_config(sourceConfig, "源存储")?;
    let dest = parse_storage_config(destConfig, "目标存储")?;
    let mode = parse_sync_mode(&syncMode)?;

    let job = SyncJob::new(name, source, dest, mode, schedule);
    job.save(&state.db).await.map_err(|e| e.to_string())?;

    Ok(job)
}

/// 更新同步任务
#[tauri::command]
pub async fn update_job(
    id: String,
    name: Option<String>,
    sourceConfig: Option<serde_json::Value>,
    destConfig: Option<serde_json::Value>,
    syncMode: Option<String>,
    schedule: Option<Option<String>>,
    enabled: Option<bool>,
    state: State<'_, AppState>,
) -> Result<SyncJob, String> {
    let mut job = SyncJob::load(&state.db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("任务不存在: {}", id))?;

    if let Some(n) = name {
        job.name = n;
    }
    if let Some(sc) = sourceConfig {
        job.sourceConfig = parse_storage_config(sc, "源存储")?;
    }
    if let Some(dc) = destConfig {
        job.destConfig = parse_storage_config(dc, "目标存储")?;
    }
    if let Some(sm) = syncMode {
        job.syncMode = parse_sync_mode(&sm)?;
    }
    if let Some(s) = schedule {
        job.schedule = s;
    }
    if let Some(e) = enabled {
        job.enabled = e;
    }
    job.updatedAt = chrono::Utc::now().timestamp();

    job.save(&state.db).await.map_err(|e| e.to_string())?;

    Ok(job)
}

/// 删除同步任务
#[tauri::command]
pub async fn delete_job(id: String, state: State<'_, AppState>) -> Result<(), String> {
    SyncJob::delete(&state.db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取数据存储路径
#[tauri::command]
pub async fn get_data_path(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.config_dir.to_string_lossy().to_string())
}

/// 设置数据存储路径并迁移数据
#[tauri::command]
pub async fn set_data_path(path: String, state: State<'_, AppState>) -> Result<String, String> {
    use std::path::PathBuf;
    
    // 验证路径是否存在
    let new_path = PathBuf::from(&path);
    if !new_path.exists() {
        return Err("指定的路径不存在".to_string());
    }
    if !new_path.is_dir() {
        return Err("指定的路径不是目录".to_string());
    }
    
    let old_path = &state.config_dir;
    
    // 如果路径相同，不需要迁移
    if old_path == &new_path {
        return Ok("路径未改变".to_string());
    }
    
    // 迁移数据文件
    let mut migrated_files = Vec::new();
    let files_to_migrate = ["synctools.db", "synctools.db-shm", "synctools.db-wal"];
    
    for file_name in &files_to_migrate {
        let old_file = old_path.join(file_name);
        let new_file = new_path.join(file_name);
        
        if old_file.exists() {
            // 复制文件到新位置
            if let Err(e) = std::fs::copy(&old_file, &new_file) {
                // 回滚已复制的文件
                for migrated in &migrated_files {
                    let _ = std::fs::remove_file(new_path.join(migrated));
                }
                return Err(format!("迁移文件 {} 失败: {}", file_name, e));
            }
            migrated_files.push(file_name.to_string());
        }
    }
    
    // 获取配置文件路径（始终存在默认位置）
    let config_file = crate::dirs::config_dir()
        .map(|p| p.join("synctools").join("config.json"))
        .ok_or_else(|| "无法获取配置目录".to_string())?;
    
    // 确保父目录存在
    if let Some(parent) = config_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    // 写入配置
    let config = serde_json::json!({
        "data_path": path
    });
    std::fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| format!("保存配置失败: {}", e))?;
    
    // 删除旧文件
    for file_name in &migrated_files {
        let old_file = old_path.join(file_name);
        let _ = std::fs::remove_file(&old_file);
    }
    
    Ok(format!("已迁移 {} 个文件", migrated_files.len()))
}
