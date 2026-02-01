// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use synctools_lib::logging::{get_log_dir, LogConfig, SizeRotatingWriter};
use synctools_lib::AppState;
use tracing_subscriber::prelude::*;

/// 初始化日志系统
fn init_logging() {
    let log_dir = get_log_dir();
    let _ = std::fs::create_dir_all(&log_dir);
    
    let config = LogConfig::load(&log_dir);
    
    if !config.enabled {
        // 日志已禁用，只初始化一个空的 subscriber
        let subscriber = tracing_subscriber::registry();
        let _ = tracing::subscriber::set_global_default(subscriber);
        return;
    }
    
    // 创建日志级别过滤器
    let level = config.tracing_level();
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(level.into())
        .add_directive("tao=error".parse().unwrap()) // 隐藏 tao 的警告
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap());
    
    // 创建文件日志写入器
    if let Ok(file_writer) = SizeRotatingWriter::new(&log_dir, config.max_size_mb) {
        // 文件日志层 - 始终输出到文件
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file_writer)
            .with_ansi(false)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false);
        
        // 在 debug 模式下也输出到控制台
        #[cfg(debug_assertions)]
        {
            let console_layer = tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false);
            
            let subscriber = tracing_subscriber::registry()
                .with(env_filter)
                .with(file_layer)
                .with(console_layer);
            
            let _ = tracing::subscriber::set_global_default(subscriber);
        }
        
        // 在 release 模式下只输出到文件
        #[cfg(not(debug_assertions))]
        {
            let subscriber = tracing_subscriber::registry()
                .with(env_filter)
                .with(file_layer);
            
            let _ = tracing::subscriber::set_global_default(subscriber);
        }
    } else {
        // 文件日志创建失败，回退到控制台
        #[cfg(debug_assertions)]
        {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .init();
        }
    }
}

#[tokio::main]
async fn main() {
    // 初始化日志系统
    init_logging();

    let state = AppState::new()
        .await
        .expect("Failed to initialize application state");

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            synctools_lib::commands::job::get_jobs,
            synctools_lib::commands::job::create_job,
            synctools_lib::commands::job::update_job,
            synctools_lib::commands::job::delete_job,
            synctools_lib::commands::job::get_data_path,
            synctools_lib::commands::job::set_data_path,
            synctools_lib::commands::sync::start_sync,
            synctools_lib::commands::sync::cancel_sync,
            synctools_lib::commands::sync::cancel_analyze,
            synctools_lib::commands::sync::pause_sync,
            synctools_lib::commands::sync::resume_sync,
            synctools_lib::commands::sync::get_pending_transfers,
            synctools_lib::commands::sync::get_sync_history,
            synctools_lib::commands::sync::analyze_job,
            synctools_lib::commands::test::test_connection,
            synctools_lib::commands::log::get_log_config,
            synctools_lib::commands::log::set_log_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
