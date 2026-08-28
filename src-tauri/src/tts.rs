use crate::config::Config;
use std::process::Command;
use tauri::AppHandle;

/// 统一语音播报入口：按配置引擎（ai / system）朗读文本。
/// - ai：调 OpenAI 兼容 `/audio/speech`（AI 神经语音，可指向本地 Kokoro 等服务）
/// - system：系统语音（Windows SAPI / macOS say / Linux spd-say）兜底
pub fn speak(app: &AppHandle, cfg: &Config, text: &str) -> Result<(), String> {
    if cfg.tts.engine == "ai" {
        speak_ai(app, cfg, text)
    } else {
        speak_system(&cfg.tts.voice, text)
    }
}

/// AI 合成语音：POST {base}/audio/speech -> mp3 -> rodio 直接播放（无黑框、跨平台）。
fn speak_ai(_app: &AppHandle, cfg: &Config, text: &str) -> Result<(), String> {
    let base = cfg.model_api.api_url.trim_end_matches('/');
    let url = format!("{base}/audio/speech");
    let body = serde_json::json!({
        "model": cfg.tts.model,
        "voice": cfg.tts.voice,
        "input": text,
        "response_format": "mp3"
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP 客户端初始化失败: {e}"))?;
    let mut req = client.post(&url).json(&body);
    if !cfg.model_api.api_key.is_empty() {
        req = req.bearer_auth(&cfg.model_api.api_key);
    }
    let resp = req.send().map_err(|e| {
        format!(
            "无法连接 AI 语音服务（{base}）。若该服务不支持 TTS，请改用「系统语音」: {e}"
        )
    })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let eb: serde_json::Value = resp.json().unwrap_or_default();
        let msg = eb["error"]["message"].as_str().unwrap_or("").trim().to_string();
        return Err(format!("AI 语音服务返回错误: HTTP {} {}", status, msg).trim_end().to_string());
    }
    let bytes = resp.bytes().map_err(|e| format!("读取音频失败: {e}"))?;
    if bytes.is_empty() {
        return Err("AI 语音服务返回了空音频".into());
    }
    play_mp3(&bytes)
}

/// 用 rodio 播放 mp3（写临时文件后播放，无窗口子进程，三平台一致）。
fn play_mp3(bytes: &[u8]) -> Result<(), String> {
    use rodio::Source;
    let path = std::env::temp_dir().join(format!("focusguard_tts_{}.mp3", std::process::id()));
    std::fs::write(&path, bytes).map_err(|e| format!("写入音频失败: {e}"))?;
    let (_stream, handle) = rodio::OutputStream::try_default()
        .map_err(|e| format!("无法打开音频输出设备: {e}"))?;
    let file = std::fs::File::open(&path).map_err(|e| format!("打开音频失败: {e}"))?;
    let source = rodio::Decoder::new(std::io::BufReader::new(file))
        .map_err(|e| format!("解码音频失败（请确认服务返回的是 mp3）: {e}"))?;
    let dur = source.total_duration().unwrap_or(std::time::Duration::from_secs(3));
    handle
        .play_raw(source.convert_samples())
        .map_err(|e| format!("播放失败: {e}"))?;
    std::thread::sleep(dur + std::time::Duration::from_millis(300));
    let _ = std::fs::remove_file(&path);
    Ok(())
}

/// system 引擎：Windows SAPI / macOS say / Linux spd-say（三平台兜底，保证出声）。
fn speak_system(voice: &str, text: &str) -> Result<(), String> {
    let _ = voice; // 部分系统音色由引擎默认，不强制选择
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let safe = text.replace('\'', "''");
        let voice_sel = if voice.is_empty() {
            String::new()
        } else {
            format!("try {{ $s.SelectVoice('{}') }} catch {{}};", voice.replace('\'', "''"))
        };
        // 显式设置默认音频输出 + 优先选中文语音，避免中文被英文音色读出或无声
        let script = format!(
            "Add-Type -AssemblyName System.Speech;\
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer;\
             try {{ $s.SetOutputToDefaultAudioDevice() }} catch {{}};\
             try {{ $v = $s.GetInstalledVoices() | ForEach-Object {{ $_.VoiceInfo }} | Where-Object {{ $_.Culture.Name -like 'zh*' }} | Select-Object -First 1; if ($v) {{ $s.SelectVoice($v.Name) }} else {{ Write-Output 'NO_ZH_VOICE' }} }} catch {{}};\
             {voice_sel}\
             $s.Speak('{safe}');\
             $s.Dispose()"
        );
        let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&utf16);
        let out = Command::new("powershell")
            .args(["-NoProfile", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("启动系统语音失败: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(format!(
                "系统语音播报失败（退出码 {:?}）：{err}。请确认系统已安装语音包（中文语音需中文语言包）且音频设备正常。",
                out.status.code()
            ))
        }
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("say");
        if !voice.is_empty() {
            cmd.args(["-v", voice]);
        }
        cmd.arg(text);
        let status = cmd.status().map_err(|e| format!("启动系统语音失败: {e}"))?;
        if status.success() { Ok(()) } else { Err(format!("系统语音播报失败（退出码 {:?}）。", status.code())) }
    }
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("spd-say").arg(text).status();
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err("speech-dispatcher 未运行或播报失败（请安装 speech-dispatcher 并启动服务）".into()),
            Err(e) => Err(format!("找不到 spd-say，请安装 speech-dispatcher: {e}")),
        }
    }
}

#[cfg(target_os = "windows")]
use base64::Engine;
