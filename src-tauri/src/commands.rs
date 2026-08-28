use crate::config::{self, Config, TtsConfig};
use crate::gpu;
use crate::model::{self, OpenAiClient};
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
    let mut cfg = cfg;
    cfg.configured = true;
    config::save(&app, &cfg)
}

/// 连接测试结果。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    pub ok: bool,
    pub message: String,
    pub models: Vec<String>,
}

/// 测试 OpenAI 兼容模型服务连接（列出可用模型）。
#[tauri::command]
pub async fn test_connection(api_url: String, api_key: String) -> ConnectionTest {
    let client = OpenAiClient::new(api_url, api_key);
    match client.list_models().await {
        Ok(models) => ConnectionTest {
            ok: true,
            message: format!("连接成功，可用模型 {} 个", models.len()),
            models: models.into_iter().map(|m| m.id).collect(),
        },
        Err(e) => ConnectionTest {
            ok: false,
            message: e,
            models: Vec::new(),
        },
    }
}

/// 校验配置是否完整（前端先校验，这里是后端兜底）。
fn validate_config(cfg: &Config) -> Result<(), String> {
    if !cfg.screen.enabled && !cfg.camera.enabled {
        return Err("请至少开启一个采集源（屏幕或摄像头）".into());
    }
    if cfg.screen.enabled && cfg.screen.prompt.trim().is_empty() {
        return Err("请填写屏幕监控的提示词，说明你要专注的任务".into());
    }
    if cfg.camera.enabled && cfg.camera.prompt.trim().is_empty() {
        return Err("请填写摄像头监控的提示词，说明专注状态".into());
    }
    if !cfg.demo_mode {
        if cfg.model_api.api_url.trim().is_empty() {
            return Err("请填写模型服务 URL（OpenAI 兼容地址）".into());
        }
        if cfg.model_api.model.trim().is_empty() {
            return Err("请填写要使用的模型名".into());
        }
    }
    Ok(())
}

async fn ensure_model_available(cfg: &Config) -> Result<(), String> {
    if cfg.demo_mode {
        return Ok(());
    }
    let client = OpenAiClient::new(cfg.model_api.api_url.clone(), cfg.model_api.api_key.clone());
    let models = client.list_models().await?;
    let target = cfg.model_api.model.trim();
    let exists = models.iter().any(|m| m.id == target);
    if !exists {
        return Err(format!(
            "模型「{}」在服务中不可用。可用模型：{}。请检查模型名，或把 URL 指向正确的 `/v1` 服务。",
            target,
            if models.is_empty() {
                "（服务未返回任何模型）".to_string()
            } else {
                models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(", ")
            }
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn start_monitoring(
    app: AppHandle,
    state: State<'_, AppState>,
    cfg: Config,
) -> Result<(), String> {
    validate_config(&cfg)?;
    ensure_model_available(&cfg).await?;

    let mut cfg = cfg;
    cfg.configured = true;
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

/// 单次测试检测（屏幕 / 摄像头）
#[tauri::command]
pub async fn detect_once(app: AppHandle, source: String) -> Result<model::DetectionResult, String> {
    let cfg = config::load(&app);
    monitor::detect_once_impl(&app, &cfg, &source).await
}

#[tauri::command]
pub async fn send_test_reminder(
    app: AppHandle,
    kind: String,
    voice_text: String,
) -> Result<String, String> {
    let cfg = config::load(&app);
    let title = "FocusGuard 测试提醒";
    let body = "这是一条测试提醒：检测到开小差时会这样提醒你。";
    match kind.as_str() {
        "system" => {
            crate::reminder::system_notify(&app, title, body);
            Ok("已发送系统通知".into())
        }
        "voice" => {
            let a = app.clone();
            let t = voice_text.clone();
            let c = cfg.tts.clone();
            tokio::task::spawn_blocking(move || crate::tts::speak(&a, &c, &t))
                .await
                .map_err(|e| format!("语音线程异常: {e}"))??;
            Ok("语音已播报".into())
        }
        "both" => {
            crate::reminder::system_notify(&app, title, body);
            let a = app.clone();
            let t = voice_text.clone();
            let c = cfg.tts.clone();
            tokio::task::spawn_blocking(move || crate::tts::speak(&a, &c, &t))
                .await
                .map_err(|e| format!("语音线程异常: {e}"))??;
            Ok("系统通知 + 语音已触发".into())
        }
        _ => Ok("已忽略（未选择提醒方式）".into()),
    }
}

#[tauri::command]
pub fn list_monitors() -> Result<Vec<String>, String> {
    crate::capture::list_monitors()
}

#[tauri::command]
pub fn list_cameras() -> Result<Vec<String>, String> {
    crate::capture::list_cameras()
}

/// 读取历史检测画面（返回 data URI，前端直接显示）。
#[tauri::command]
pub fn read_history_image(path: String) -> Result<String, String> {
    use base64::Engine;
    let bytes = std::fs::read(&path).map_err(|e| format!("读取历史截图失败: {e}"))?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

// ---------- TTS ----------

#[tauri::command]
pub fn list_piper_voices() -> Vec<crate::tts::PiperVoice> {
    crate::tts::piper_voices()
}

#[tauri::command]
pub fn piper_status(app: AppHandle) -> crate::tts::PiperStatus {
    crate::tts::piper_status(&app)
}

#[tauri::command]
pub fn open_piper_download() -> Result<(), String> {
    crate::tts::open_piper_download()
}

#[tauri::command]
pub fn open_tts_dir(app: AppHandle) -> Result<(), String> {
    crate::tts::open_tts_dir(&app)
}

#[tauri::command]
pub async fn download_piper_voice(app: AppHandle, id: String) -> Result<(), String> {
    crate::tts::download_piper_voice(&app, &id).await
}

/// 用指定 TTS 配置试听一句示例。
#[tauri::command]
pub async fn tts_preview(app: AppHandle, tts: TtsConfig) -> Result<String, String> {
    let text = "这是语音播报试听：专注，是一种力量。";
    let a = app.clone();
    let t = tts.clone();
    tokio::task::spawn_blocking(move || crate::tts::speak(&a, &t, text))
        .await
        .map_err(|e| format!("试听线程异常: {e}"))??;
    Ok("试听播放完成".into())
}
