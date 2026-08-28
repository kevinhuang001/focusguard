mod capture;
mod commands;
mod config;
mod gpu;
mod model;
mod monitor;
mod reminder;

use monitor::MonitorState;
use std::sync::Arc;

pub struct AppState {
    pub monitor: Arc<MonitorState>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            monitor: Arc::new(MonitorState::new()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::test_connection,
            commands::start_monitoring,
            commands::stop_monitoring,
            commands::get_monitor_state,
            commands::get_gpu_info,
            commands::get_recommendation,
            commands::detect_once,
            commands::send_test_reminder,
            commands::list_monitors,
            commands::list_cameras,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
