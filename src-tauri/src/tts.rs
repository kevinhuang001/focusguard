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
        PiperVoice { id: "zh_CN-huayan-medium".into(), label: "中文 · 华妍（女）".into(), lang: "zh_CN".into(), path: "zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx".into() },
        PiperVoice { id: "zh_CN-huayan-x_low".into(), label: "中文 · 华妍（女，轻量）".into(), lang: "zh_CN".into(), path: "zh/zh_CN/huayan/x_low/zh_CN-huayan-x_low.onnx".into() },
        PiperVoice { id: "zh_CN-xiaobei-medium".into(), label: "中文 · 晓北（女）".into(), lang: "zh_CN".into(), path: "zh/zh_CN/xiaobei/medium/zh_CN-xiaobei-medium.onnx".into() },
        PiperVoice { id: "zh_CN-xiaoxiao-medium".into(), label: "中文 · 晓晓（女）".into(), lang: "zh_CN".into(), path: "zh/zh_CN/xiaoxiao/medium/zh_CN-xiaoxiao-medium.onnx".into() },
        PiperVoice { id: "en_US-lessac-medium".into(), label: "英文 · Lessac".into(), lang: "en_US".into(), path: "en/en_US/lessac/medium/en_US-lessac-medium.onnx".into() },
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
    let exe = tts_dir(app)?.join(if cfg!(windows) { "piper.exe" } else { "piper" });
    Ok(exe)
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
        .filter_map(|v| {
            let onnx = voices_dir(app).ok()?.join(format!("{}.onnx", v.id));
            let json = voices_dir(app).ok()?.join(format!("{}.onnx.json", v.id));
            (onnx.exists() && json.exists()).then(|| v.id.clone())
        })
        .collect();
    PiperStatus { engine_installed, installed_voices }
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
pub async fn download_piper_voice(app: &AppHandle, id: &str) -> Result<(), String> {
    let voice = piper_voices()
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("未知音色: {id}"))?;
    let base = "https://huggingface.co/rhasspy/piper-voices/resolve/main/";
    let dir = voices_dir(app)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    for ext in ["onnx", "onnx.json"] {
        let url = format!("{base}{}.{}", voice.path, ext);
        let resp = client.get(&url).send().await
            .map_err(|e| format!("下载音色失败（{ext}）: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("下载音色失败（{ext}）: HTTP {}", resp.status()));
        }
        let bytes = resp.bytes().await.map_err(|e| format!("读取音色失败: {e}"))?;
        let dest = dir.join(format!("{}.{}", voice.id, ext));
        std::fs::write(&dest, &bytes).map_err(|e| format!("写入音色失败: {e}"))?;
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
        let mut script = format!(
            "Add-Type -AssemblyName System.Speech; $s=New-Object System.Speech.Synthesis.SpeechSynthesizer;"
        );
        if !voice.is_empty() {
            script.push_str(&format!("try {{ $s.SelectVoice('{}') }} catch {{}};", voice.replace('\'', "''")));
        }
        script.push_str(&format!("$s.Speak('{safe}')"));
        let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&utf16);
        let status = Command::new("powershell")
            .args(["-NoProfile", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("启动系统语音失败: {e}"))?;
        if status.success() { Ok(()) } else {
            Err(format!("系统语音播报失败（退出码 {:?}）。请确认系统已安装语音包（中文语音需中文语言包）。", status.code()))
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
        return Err("未找到 Piper 引擎。请到 GitHub 下载对应系统版本放入应用数据目录的 tts/ 文件夹，或改用「系统语音」。".into());
    }
    let vdir = voices_dir(app)?;
    let model = vdir.join(format!("{voice_id}.onnx"));
    let conf = vdir.join(format!("{voice_id}.onnx.json"));
    if !model.exists() || !conf.exists() {
        return Err(format!("音色「{voice_id}」未下载，请先在设置里下载/试听。"));
    }
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
