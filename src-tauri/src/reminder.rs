#[cfg(target_os = "windows")]
use base64::Engine;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

/// 系统通知（Windows 通知中心 / macOS 通知 / Linux libnotify）
pub fn system_notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// 语音播报（同步播放，播完才返回）。
/// - Windows：PowerShell + System.Speech（SAPI，一定有默认语音）
/// - macOS：`say`
/// - Linux：`spd-say`（speech-dispatcher）
pub fn speak(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // PowerShell 字符串内单引号用 '' 转义
        let safe = text.replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.Speak('{safe}')"
        );
        // 用 -EncodedCommand（UTF-16LE base64）避免任何引号/编码问题
        let utf16: Vec<u8> = script
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&utf16);
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("启动语音播报失败: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("语音播报失败（退出码 {:?}），请检查系统语音设置", status.code()))
        }
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("say")
            .arg(text)
            .status()
            .map_err(|e| format!("语音播报失败: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        match std::process::Command::new("spd-say").arg(text).status() {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err(
                "speech-dispatcher 未运行或播报失败（可执行: systemctl --user start speech-dispatcher 或安装 speech-dispatcher 后重试）"
                    .into(),
            ),
            Err(e) => Err(format!("找不到 spd-say，请安装 speech-dispatcher: {e}")),
        }
    }
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
