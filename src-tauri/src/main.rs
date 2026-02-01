// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use synctools_lib::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
                .add_directive("tao=error".parse().unwrap()), // 隐藏 tao 的警告
        )
        .init();

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
