use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

/// 系统通知（Windows 通知中心 / macOS 通知 / Linux libnotify）
pub fn system_notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// 语音播报（Windows SAPI / macOS say / Linux speech-dispatcher）
pub fn speak(text: &str) -> Result<(), String> {
    let mut tts = tts::Tts::default().map_err(|e| format!("语音引擎初始化失败: {e}"))?;
    tts.speak(text, true)
        .map_err(|e| format!("语音播报失败: {e}"))?;
    Ok(())
}

/// 按提醒类型触发提醒，并向前端广播事件。
pub async fn fire(app: AppHandle, kind: &str, voice_text: &str, title: &str, body: &str) {
    let _ = app.emit(
        "monitor://reminder",
        serde_json::json!({ "kind": kind, "title": title, "text": body }),
    );
    match kind {
        "system" => system_notify(&app, title, body),
        "voice" => {
            let text = voice_text.to_string();
            tokio::task::spawn_blocking(move || {
                let _ = speak(&text);
            });
        }
        "both" => {
            system_notify(&app, title, body);
            let text = voice_text.to_string();
            tokio::task::spawn_blocking(move || {
                let _ = speak(&text);
            });
        }
        _ => {}
    }
}
