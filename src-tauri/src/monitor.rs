use crate::capture;
use crate::config::Config;
use crate::model::{self, DetectionResult, OllamaClient};
use crate::reminder;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

pub struct MonitorHandle {
    pub join: tauri::async_runtime::JoinHandle<()>,
    pub stop: tokio::sync::watch::Sender<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorTick {
    pub source: String,
    pub focused: bool,
    pub reason: String,
    pub model: String,
    pub backend: String,
    pub duration_ms: u64,
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub running: bool,
    pub started_at: Option<u64>,
    pub last_ticks: Vec<MonitorTick>,
    pub miss_count: u32,
    pub last_reminder_at: Option<u64>,
}

pub struct MonitorState {
    pub handle: Mutex<Option<MonitorHandle>>,
    pub snapshot: Mutex<MonitorSnapshot>,
    pub tick_counter: AtomicU64,
}

impl MonitorState {
    pub fn new() -> Self {
        Self {
            handle: Mutex::new(None),
            snapshot: Mutex::new(MonitorSnapshot {
                running: false,
                started_at: None,
                last_ticks: Vec::new(),
                miss_count: 0,
                last_reminder_at: None,
            }),
            tick_counter: AtomicU64::new(0),
        }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn start(app: &AppHandle, state: &Arc<MonitorState>, cfg: Config) -> Result<(), String> {
    // 停止已有监控
    let mut guard = state.handle.lock().unwrap();
    if let Some(h) = guard.take() {
        let _ = h.stop.send(true);
    }
    drop(guard);

    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let app2 = app.clone();
    let state2 = state.clone();
    let join = tauri::async_runtime::spawn(async move {
        run_loop(app2, state2, cfg, stop_rx).await;
    });
    *state.handle.lock().unwrap() = Some(MonitorHandle { join, stop: stop_tx });
    Ok(())
}

pub fn stop(state: &Arc<MonitorState>) {
    if let Some(h) = state.handle.lock().unwrap().take() {
        let _ = h.stop.send(true);
    }
    let mut snap = state.snapshot.lock().unwrap();
    snap.running = false;
    snap.started_at = None;
}

/// 单次检测（供「测试」按钮使用）。
pub async fn detect_once_impl(cfg: &Config, source: &str) -> Result<DetectionResult, String> {
    check_source(source, cfg, 0).await
}

async fn run_loop(
    app: AppHandle,
    state: Arc<MonitorState>,
    cfg: Config,
    mut stop_rx: tokio::sync::watch::Receiver<bool>,
) {
    let started = now_ms();
    {
        let mut snap = state.snapshot.lock().unwrap();
        snap.running = true;
        snap.started_at = Some(started);
        snap.last_ticks.clear();
        snap.miss_count = 0;
        snap.last_reminder_at = None;
    }
    let _ = app.emit("monitor://state", serde_json::json!({ "running": true }));

    let interval_secs = cfg.interval_secs.clamp(2, 3600);
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            _ = interval.tick() => {
                let tick_no = state.tick_counter.fetch_add(1, Ordering::Relaxed);
                let results = check_all(&app, &state, &cfg, tick_no).await;

                let now = now_ms();
                let mut should_fire = false;
                {
                    let mut snap = state.snapshot.lock().unwrap();
                    let mut any_distracted = false;
                    for (source, res) in results {
                        let t = match res {
                            Ok(det) => {
                                if !det.focused {
                                    any_distracted = true;
                                }
                                MonitorTick {
                                    source: source.clone(),
                                    focused: det.focused,
                                    reason: det.reason.clone(),
                                    model: det.model.clone(),
                                    backend: det.backend.clone(),
                                    duration_ms: det.duration_ms,
                                    ts: now_ms(),
                                    error: None,
                                }
                            }
                            Err(e) => MonitorTick {
                                source: source.clone(),
                                focused: true, // 出错不计为开小差
                                reason: String::new(),
                                model: cfg.model.clone(),
                                backend: cfg.backend.clone(),
                                duration_ms: 0,
                                ts: now_ms(),
                                error: Some(e),
                            },
                        };
                        snap.last_ticks.push(t.clone());
                        if snap.last_ticks.len() > 200 {
                            snap.last_ticks.remove(0);
                        }
                        let _ = app.emit(
                            "monitor://tick",
                            serde_json::to_value(&t).unwrap_or_default(),
                        );
                    }

                    // 提醒逻辑：连续 miss_threshold 次开小差 且 冷却期已过
                    let cooldown_ok = snap
                        .last_reminder_at
                        .map(|t| now.saturating_sub(t) >= cfg.reminder.cooldown_secs * 1000)
                        .unwrap_or(true);
                    if any_distracted {
                        snap.miss_count += 1;
                        if snap.miss_count >= cfg.reminder.miss_threshold.max(1) && cooldown_ok {
                            snap.last_reminder_at = Some(now);
                            should_fire = true;
                        }
                    } else {
                        snap.miss_count = 0;
                    }
                } // 释放锁，避免持有 MutexGuard 跨 await

                if should_fire {
                    reminder::fire(
                        app.clone(),
                        &cfg.reminder.kind,
                        &cfg.reminder.voice_text,
                            "专注监控提醒",
                            "检测到你似乎没有在专注当前任务，请回到正轨！",
                        )
                        .await;
                }
            }
        }
    }

    let mut snap = state.snapshot.lock().unwrap();
    snap.running = false;
    snap.started_at = None;
    let _ = app.emit("monitor://state", serde_json::json!({ "running": false }));
}

async fn check_all(
    _app: &AppHandle,
    _state: &Arc<MonitorState>,
    cfg: &Config,
    tick_no: u64,
) -> Vec<(String, Result<DetectionResult, String>)> {
    let mut tasks = Vec::new();
    if cfg.screen.enabled {
        let cfg2 = cfg.clone();
        tasks.push(tokio::task::spawn(async move {
            ("screen".to_string(), check_source("screen", &cfg2, tick_no).await)
        }));
    }
    if cfg.camera.enabled {
        let cfg2 = cfg.clone();
        tasks.push(tokio::task::spawn(async move {
            ("camera".to_string(), check_source("camera", &cfg2, tick_no).await)
        }));
    }
    let mut out = Vec::new();
    for t in tasks {
        if let Ok((s, r)) = t.await {
            out.push((s, r));
        }
    }
    out
}

async fn check_source(source: &str, cfg: &Config, tick_no: u64) -> Result<DetectionResult, String> {
    let monitor_index = cfg.screen.monitor_index;
    let camera_index = cfg.camera.camera_index;
    let max_width = cfg.image_max_width;
    let is_screen = source == "screen";

    let img = tokio::task::spawn_blocking(move || -> Result<image::RgbaImage, String> {
        if is_screen {
            capture::capture_screen(monitor_index, max_width)
        } else {
            capture::capture_camera(camera_index, max_width)
        }
    })
    .await
    .map_err(|e| format!("采集线程异常: {e}"))??;

    let b64 = capture::to_jpeg_base64(&img, 70)?;

    if cfg.backend == "mock" {
        return Ok(model::mock_detect(source, tick_no));
    }

    let prompt = if is_screen {
        cfg.screen.prompt.clone()
    } else {
        cfg.camera.prompt.clone()
    };
    let client = OllamaClient::new(cfg.ollama_url.clone());
    let mut det = client.detect(&cfg.model, &prompt, &b64).await?;
    det.source = source.to_string();
    Ok(det)
}
