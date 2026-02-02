// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use synctools_lib::logging::{get_log_dir, LogConfig, SizeRotatingWriter};
use synctools_lib::AppState;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Listener, Manager, RunEvent, WindowEvent,
};
use tracing_subscriber::prelude::*;

/// 显示主窗口
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 初始化日志系统
fn init_logging() {
    let log_dir = get_log_dir();
    let _ = std::fs::create_dir_all(&log_dir);
    let config = LogConfig::load(&log_dir);

    if !config.enabled {
        let _ = tracing::subscriber::set_global_default(tracing_subscriber::registry());
        return;
    }

    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(config.tracing_level().into())
        .add_directive("tao=error".parse().unwrap())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap());

    let Ok(file_writer) = SizeRotatingWriter::new(&log_dir, config.max_size_mb) else {
        #[cfg(debug_assertions)]
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
        return;
    };

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false);

    #[cfg(debug_assertions)]
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(tracing_subscriber::fmt::layer().with_target(false).with_thread_ids(false).with_thread_names(false));

    #[cfg(not(debug_assertions))]
    let subscriber = tracing_subscriber::registry().with(env_filter).with(file_layer);

    let _ = tracing::subscriber::set_global_default(subscriber);
}

#[tokio::main]
async fn main() {
    // 初始化日志系统
    init_logging();

    let state = AppState::new()
        .await
        .expect("Failed to initialize application state");
    
    // 包装在 Arc 中以便在退出时访问
    let state_for_cleanup = Arc::new(state.clone());

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(|app| {
            // 创建托盘菜单
            let show_item = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // 创建系统托盘
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("SyncTools - 文件同步工具")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(event, TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up, ..
                    }) {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // 监听前端 ready 事件后显示窗口
            let app_handle = app.handle().clone();
            app.listen("frontend-ready", move |_| {
                show_main_window(&app_handle);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // 主窗口关闭时隐藏到托盘
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
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
            synctools_lib::commands::sync::resume_sync,
            synctools_lib::commands::sync::get_pending_transfers,
            synctools_lib::commands::sync::get_sync_history,
            synctools_lib::commands::sync::analyze_job,
            synctools_lib::commands::sync::clear_scan_cache,
            synctools_lib::commands::test::test_connection,
            synctools_lib::commands::log::get_log_config,
            synctools_lib::commands::log::set_log_config,
            synctools_lib::commands::cache::get_cache_config,
            synctools_lib::commands::cache::set_cache_config,
            synctools_lib::commands::transfer::get_transfer_config,
            synctools_lib::commands::transfer::set_transfer_config,
            synctools_lib::commands::shell::show_in_folder,
            synctools_lib::commands::shell::rename_file,
            synctools_lib::commands::shell::delete_file,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // 运行应用并处理退出事件
    app.run(move |_app_handle, event| {
        if let RunEvent::Exit = event {
            // 应用退出时清理资源
            let state = state_for_cleanup.clone();
            // 使用 block_on 同步执行异步清理
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    state.cleanup().await;
                });
            });
        }
    });
}
