use crate::config::TtsConfig;
use crate::tts;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

/// 系统通知（Windows 通知中心 / macOS 通知 / Linux libnotify）
pub fn system_notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// 按提醒类型触发提醒：语音统一走 tts 模块（system/piper），可叠加系统通知。
pub async fn fire(
    app: AppHandle,
    kind: &str,
    tts_cfg: &TtsConfig,
    text: &str,
    title: &str,
    body: &str,
) {
    let _ = app.emit(
        "monitor://reminder",
        serde_json::json!({ "kind": kind, "title": title, "text": text }),
    );
    match kind {
        "system" => system_notify(&app, title, body),
        "voice" => {
            let a = app.clone();
            let t = text.to_string();
            let c = tts_cfg.clone();
            tokio::task::spawn_blocking(move || {
                let _ = tts::speak(&a, &c, &t);
            });
        }
        "both" => {
            system_notify(&app, title, body);
            let a = app.clone();
            let t = text.to_string();
            let c = tts_cfg.clone();
            tokio::task::spawn_blocking(move || {
                let _ = tts::speak(&a, &c, &t);
            });
        }
        _ => {}
    }
}
