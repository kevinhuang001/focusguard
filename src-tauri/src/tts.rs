use crate::config::TtsConfig;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::io::Write;
use tauri::{AppHandle, Manager};

/// 预置 Piper 音色（本地开源 TTS，三平台一致）。
/// path 为 HuggingFace `rhasspy/piper-voices` 下的相对路径，用于下载 onnx 模型。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiperVoice {
    pub id: String,
    pub label: String,
    pub lang: String,
    pub path: String,
}

pub fn piper_voices() -> Vec<PiperVoice> {
    vec![
        PiperVoice { id: "zh_CN-huayan-medium".into(), label: "中文 · 华妍（女）".into(), lang: "zh_CN".into(), path: "zh/zh_CN/huayan/medium/zh_CN-huayan-medium".into() },
        PiperVoice { id: "zh_CN-huayan-x_low".into(), label: "中文 · 华妍（女，轻量）".into(), lang: "zh_CN".into(), path: "zh/zh_CN/huayan/x_low/zh_CN-huayan-x_low".into() },
        PiperVoice { id: "zh_CN-xiaobei-medium".into(), label: "中文 · 晓北（女）".into(), lang: "zh_CN".into(), path: "zh/zh_CN/xiaobei/medium/zh_CN-xiaobei-medium".into() },
        PiperVoice { id: "zh_CN-xiaoxiao-medium".into(), label: "中文 · 晓晓（女）".into(), lang: "zh_CN".into(), path: "zh/zh_CN/xiaoxiao/medium/zh_CN-xiaoxiao-medium".into() },
        PiperVoice { id: "en_US-lessac-medium".into(), label: "英文 · Lessac".into(), lang: "en_US".into(), path: "en/en_US/lessac/medium/en_US-lessac-medium".into() },
    ]
}

fn tts_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取数据目录: {e}"))?
        .join("tts");
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建 TTS 目录: {e}"))?;
    Ok(dir)
}

fn voices_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = tts_dir(app)?.join("voices");
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建音色目录: {e}"))?;
    Ok(dir)
}

fn piper_exe(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let name = if cfg!(windows) { "piper.exe" } else { "piper" };
    // 优先内置资源（打包内置的引擎），其次应用数据目录（用户手动放置的）
    if let Ok(dir) = app.path().resource_dir() {
        let p = dir.join("piper").join(name);
        if p.exists() {
            return Ok(p);
        }
    }
    Ok(tts_dir(app)?.join(name))
}

/// 递归查找音色文件：优先用户数据目录（任意子目录），其次内置资源。
fn voice_file(app: &AppHandle, id: &str, ext: &str) -> Option<std::path::PathBuf> {
    let target = format!("{id}.{ext}");
    if let Ok(d) = tts_dir(app) {
        if let Some(p) = find_in_dir(&d, &target) {
            return Some(p);
        }
    }
    if let Ok(dir) = app.path().resource_dir() {
        let p = dir
            .join("piper")
            .join("voices")
            .join(format!("{id}.{ext}"));
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// 在目录树中递归查找匹配文件名的文件（限制深度避免深陷）。
fn find_in_dir(dir: &std::path::Path, target: &str) -> Option<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, target: &str, depth: u32) -> Option<std::path::PathBuf> {
        if depth > 6 {
            return None;
        }
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let path = e.path();
                if path.is_dir() {
                    if let Some(p) = walk(&path, target, depth + 1) {
                        return Some(p);
                    }
                } else if path.file_name().and_then(|n| n.to_str()) == Some(target) {
                    return Some(path);
                }
            }
        }
        None
    }
    walk(dir, target, 0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiperStatus {
    pub engine_installed: bool,
    pub installed_voices: Vec<String>,
}

pub fn piper_status(app: &AppHandle) -> PiperStatus {
    let engine_installed = piper_exe(app).map(|e| e.exists()).unwrap_or(false);
    let installed_voices = piper_voices()
        .iter()
        .filter(|v| {
            voice_file(app, &v.id, "onnx").is_some() && voice_file(app, &v.id, "onnx.json").is_some()
        })
        .map(|v| v.id.clone())
        .collect();
    PiperStatus { engine_installed, installed_voices }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsPaths {
    pub tts_dir: String,
    pub voices_dir: String,
}

/// 返回应用实际查找的 TTS 目录（便于用户放置引擎/音色）。
pub fn tts_paths(app: &AppHandle) -> TtsPaths {
    TtsPaths {
        tts_dir: tts_dir(app).map(|d| d.to_string_lossy().to_string()).unwrap_or_default(),
        voices_dir: voices_dir(app).map(|d| d.to_string_lossy().to_string()).unwrap_or_default(),
    }
}

/// 用系统文件管理器打开 TTS 资源目录（方便手动放置 piper 引擎/音色）。
pub fn open_tts_dir(app: &AppHandle) -> Result<(), String> {
    let dir = tts_dir(app)?;
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(&dir).spawn()
            .map_err(|e| format!("无法打开目录: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&dir).spawn().map_err(|e| format!("无法打开目录: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(&dir).spawn().map_err(|e| format!("无法打开目录: {e}"))?;
    }
    Ok(())
}

/// Linux 下载 Piper 引擎（解压仍需用户完成）。
pub fn open_piper_download() -> Result<(), String> {
    let url = if cfg!(target_os = "linux") {
        "https://github.com/rhasspy/piper/releases/latest"
    } else if cfg!(target_os = "macos") {
        "https://github.com/rhasspy/piper/releases/latest"
    } else {
        "https://github.com/rhasspy/piper/releases/latest"
    };
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()
            .map_err(|e| format!("打不开浏览器: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn().map_err(|e| format!("打不开浏览器: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn().map_err(|e| format!("打不开浏览器: {e}"))?;
    }
    Ok(())
}

/// 下载指定 Piper 音色（onnx + json）到本地。
/// 优先从 ModelScope（国内），失败回退 HuggingFace；下载时发送进度事件。
pub async fn download_piper_voice(app: &AppHandle, id: &str) -> Result<(), String> {
    use futures_util::StreamExt;
    use std::io::Write;
    use tauri::Emitter;

    let voice = piper_voices()
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("未知音色: {id}"))?;
    let dir = voices_dir(app)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    // 双源：ModelScope（国内优先）→ HuggingFace 回退
    let sources = [
        format!("https://modelscope.cn/models/rhasspy/piper-voices/resolve/master/{}", voice.path),
        format!("https://huggingface.co/rhasspy/piper-voices/resolve/main/{}", voice.path),
    ];

    for ext in ["onnx", "onnx.json"] {
        let dest = dir.join(format!("{}.{}", voice.id, ext));
        let mut ok = false;
        let mut last_err = String::new();
        for base in &sources {
            let url = format!("{base}.{ext}");
            let resp = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => { last_err = e.to_string(); continue; }
            };
            if !resp.status().is_success() {
                last_err = format!("HTTP {}", resp.status());
                continue;
            }
            let total = resp.content_length();
            let mut stream = resp.bytes_stream();
            let mut f = std::fs::File::create(&dest).map_err(|e| format!("写入失败: {e}"))?;
            let mut written: u64 = 0;
            while let Some(chunk) = stream.next().await {
                let c = chunk.map_err(|e| format!("下载中断: {e}"))?;
                f.write_all(&c).map_err(|e| format!("写入失败: {e}"))?;
                written += c.len() as u64;
                let _ = app.emit(
                    "piper://download-progress",
                    serde_json::json!({ "id": voice.id, "bytes": written, "total": total }),
                );
            }
            ok = true;
            break;
        }
        if !ok {
            return Err(format!("下载音色（{ext}）失败：{last_err}。请检查网络后重试。"));
        }
    }
    Ok(())
}

/// 统一语音播报入口：按配置的引擎（system / piper）朗读文本。
pub fn speak(app: &AppHandle, tts: &TtsConfig, text: &str) -> Result<(), String> {
    match tts.engine.as_str() {
        "piper" => speak_piper(app, &tts.piper_voice, text),
        _ => speak_system(&tts.system_voice, text),
    }
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

/// piper 引擎：调用本地 piper 二进制合成 wav 并播放。
fn speak_piper(app: &AppHandle, voice_id: &str, text: &str) -> Result<(), String> {
    let exe = piper_exe(app)?;
    if !exe.exists() {
        return Err("未找到 Piper 引擎。请点「打开 TTS 文件夹」放入 piper.exe，或改用「系统语音」。".into());
    }
    let model = voice_file(app, voice_id, "onnx")
        .ok_or_else(|| format!("音色「{voice_id}」未找到。请下载该音色或选择内置音色「华妍·轻量」。"))?;
    let conf = voice_file(app, voice_id, "onnx.json")
        .ok_or_else(|| format!("音色「{voice_id}」的配置文件(.onnx.json)未找到。"))?;
    let out = std::env::temp_dir().join(format!("focusguard_tts_{}.wav", std::process::id()));

    let mut child = Command::new(&exe)
        .arg("-m").arg(&model)
        .arg("-c").arg(&conf)
        .arg("-f").arg(&out)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 Piper 失败: {e}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes()).map_err(|e| format!("写入文本失败: {e}"))?;
    }
    let status = child.wait().map_err(|e| format!("Piper 运行失败: {e}"))?;
    if !status.success() {
        return Err(format!("Piper 合成失败（退出码 {:?}）。", status.code()));
    }
    play_wav(&out)
}

fn play_wav(path: &std::path::Path) -> Result<(), String> {
    let p = path.to_string_lossy().replace('\'', "''");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let script = format!("(New-Object System.Media.SoundPlayer '{p}').PlaySync()");
        let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&utf16);
        let status = Command::new("powershell")
            .args(["-NoProfile", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("播放失败: {e}"))?;
        if status.success() { Ok(()) } else { Err("播放 wav 失败".into()) }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = &p;
        Command::new("afplay").arg(path).status().map_err(|e| format!("播放失败: {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let _ = &p;
        let status = Command::new("aplay").arg(path).status();
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err("aplay 播放失败（请安装 alsa-utils）".into()),
            Err(e) => Err(format!("找不到 aplay: {e}")),
        }
    }
}

// Windows 分支用到的 base64
#[cfg(target_os = "windows")]
use base64::Engine;
