use std::path::Path;
use std::process::Command;

/// 在文件管理器中显示文件/目录
#[tauri::command]
pub async fn show_in_folder(path: String) -> Result<(), String> {
    // 将正斜杠转换为反斜杠（Windows 兼容）
    let normalized_path = path.replace('/', "\\");
    let path = Path::new(&normalized_path);
    
    // 如果是文件，获取父目录
    let folder = if path.is_file() {
        path.parent().map(|p| p.to_path_buf()).unwrap_or(path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    
    if !folder.exists() {
        return Err(format!("路径不存在: {}", folder.display()));
    }
    
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 explorer /select 来选中文件
        // 注意: /select, 后面直接跟路径，不能有空格，且整个作为一个参数
        if path.is_file() {
            let select_arg = format!("/select,{}", path.display());
            Command::new("explorer")
                .arg(&select_arg)
                .spawn()
                .map_err(|e| format!("无法打开文件管理器: {}", e))?;
        } else {
            Command::new("explorer")
                .arg(&folder)
                .spawn()
                .map_err(|e| format!("无法打开文件管理器: {}", e))?;
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("无法打开 Finder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux: 尝试使用 xdg-open
        Command::new("xdg-open")
            .arg(&folder)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }
    
    Ok(())
}

/// 重命名文件
#[tauri::command]
pub async fn rename_file(old_path: String, new_name: String) -> Result<(), String> {
    let old_path = Path::new(&old_path);
    
    if !old_path.exists() {
        return Err(format!("文件不存在: {}", old_path.display()));
    }
    
    // 验证新名称不包含路径分隔符
    if new_name.contains('/') || new_name.contains('\\') {
        return Err("文件名不能包含路径分隔符".to_string());
    }
    
    let parent = old_path.parent()
        .ok_or_else(|| "无法获取父目录".to_string())?;
    let new_path = parent.join(&new_name);
    
    if new_path.exists() {
        return Err(format!("目标文件已存在: {}", new_path.display()));
    }
    
    std::fs::rename(&old_path, &new_path)
        .map_err(|e| format!("重命名失败: {}", e))?;
    
    tracing::info!("文件重命名: {} -> {}", old_path.display(), new_path.display());
    
    Ok(())
}

/// 删除文件
#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    let path = Path::new(&path);
    
    if !path.exists() {
        return Err(format!("文件不存在: {}", path.display()));
    }
    
    if path.is_dir() {
        std::fs::remove_dir_all(&path)
            .map_err(|e| format!("删除目录失败: {}", e))?;
    } else {
        std::fs::remove_file(&path)
            .map_err(|e| format!("删除文件失败: {}", e))?;
    }
    
    tracing::info!("文件已删除: {}", path.display());
    
    Ok(())
}
