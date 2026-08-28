use crate::config::{self, Config};
use crate::gpu;
use crate::model::{self, OllamaClient};
use crate::monitor::{self, MonitorSnapshot, MonitorState};
use crate::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_config(app: AppHandle) -> Config {
    config::load(&app)
}

#[tauri::command]
pub fn save_config(app: AppHandle, cfg: Config) -> Result<(), String> {
    config::save(&app, &cfg)
}

#[tauri::command]
pub async fn get_gpu_info() -> gpu::GpuInfo {
    tokio::time::timeout(
        std::time::Duration::from_secs(8),
        tokio::task::spawn_blocking(gpu::detect_gpu),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
    .unwrap_or(gpu::GpuInfo {
        name: "未知".into(),
        vram_mb: None,
        source: "timeout".into(),
    })
}

#[tauri::command]
pub async fn get_recommendation() -> gpu::RecommendResult {
    let gpu = get_gpu_info().await;
    gpu::recommend(&gpu)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaInfo {
    pub installed: bool,
    pub running: bool,
    pub models: Vec<model::OllamaModel>,
}

#[tauri::command]
pub async fn ollama_info(cfg_url: String) -> OllamaInfo {
    let client = OllamaClient::new(cfg_url);
    let installed = model::ollama_installed();
    let running = client.is_running().await;
    let models = if running {
        client.list_models().await.unwrap_or_default()
    } else {
        Vec::new()
    };
    OllamaInfo {
        installed,
        running,
        models,
    }
}

#[tauri::command]
pub fn start_ollama() -> Result<(), String> {
    model::spawn_ollama_serve()
}

#[tauri::command]
pub async fn pull_model(app: AppHandle, model: String) -> Result<(), String> {
    model::pull_model_async(&app, &model).await
}

/// 单次测试检测（屏幕 / 摄像头）
#[tauri::command]
pub async fn detect_once(app: AppHandle, source: String) -> Result<model::DetectionResult, String> {
    let cfg = config::load(&app);
    monitor::detect_once_impl(&cfg, &source).await
}

#[tauri::command]
pub fn start_monitoring(
    app: AppHandle,
    state: State<'_, AppState>,
    cfg: Config,
) -> Result<(), String> {
    config::save(&app, &cfg)?;
    let monitor_state: Arc<MonitorState> = state.monitor.clone();
    monitor::start(&app, &monitor_state, cfg)
}

#[tauri::command]
pub fn stop_monitoring(app: AppHandle, state: State<'_, AppState>) {
    let monitor_state: Arc<MonitorState> = state.monitor.clone();
    monitor::stop(&monitor_state);
    let _ = app.emit("monitor://state", serde_json::json!({ "running": false }));
}

#[tauri::command]
pub fn get_monitor_state(state: State<'_, AppState>) -> MonitorSnapshot {
    state.monitor.snapshot.lock().unwrap().clone()
}

#[tauri::command]
pub async fn send_test_reminder(
    app: AppHandle,
    kind: String,
    voice_text: String,
) -> Result<(), String> {
    crate::reminder::fire(
        app,
        &kind,
        &voice_text,
        "FocusGuard 测试提醒",
        "这是一条测试提醒：检测到开小差时会这样提醒你。",
    )
    .await;
    Ok(())
}

#[tauri::command]
pub fn list_monitors() -> Result<Vec<String>, String> {
    crate::capture::list_monitors()
}

#[tauri::command]
pub fn list_cameras() -> Result<Vec<String>, String> {
    crate::capture::list_cameras()
}
