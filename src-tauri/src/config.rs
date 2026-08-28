use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig {
    pub enabled: bool,
    pub prompt: String,
    pub monitor_index: usize,
    pub camera_index: usize,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prompt: String::new(),
            monitor_index: 0,
            camera_index: 0,
        }
    }
}

/// 模型服务配置：OpenAI 兼容接口（只需填一个兼容 URL）。
/// 应用不代理任何模型的下载/安装，用户自备模型服务。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    /// OpenAI 兼容 base_url（如 https://api.openai.com/v1 或 http://localhost:11434/v1）
    pub api_url: String,
    /// 可选；本地服务可留空
    pub api_key: String,
    /// 模型名，须支持图像输入
    pub model: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:11434/v1".into(),
            api_key: String::new(),
            model: "qwen3-vl:4b".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderConfig {
    /// none | system | voice | both（保留；提醒默认统一走 TTS 语音，可同时发系统通知）
    pub kind: String,
    /// 提醒内容类型 fixed | ai
    pub content_type: String,
    /// 固定提醒文案（content_type=fixed 时用）
    pub voice_text: String,
    /// 两次提醒之间的最小间隔（秒）
    pub cooldown_secs: u64,
    /// 连续 N 次判定为开小差才提醒
    pub miss_threshold: u32,
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            kind: "voice".into(),
            content_type: "fixed".into(),
            voice_text: "注意！检测到你似乎没有在专注当前任务，请尽快回到正轨！".into(),
            cooldown_secs: 30,
            miss_threshold: 1,
        }
    }
}

/// TTS 配置：ai（调 OpenAI 兼容 /audio/speech，AI 合成语音）或 system（系统语音兜底）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsConfig {
    /// ai | system
    pub engine: String,
    /// AI 音色名（如 alloy/nova，取决于服务）；system 时用作系统音色名（可留空）
    pub voice: String,
    /// AI 语音合成模型名（如 tts-1 / gpt-4o-mini-tts）
    pub model: String,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            engine: "ai".into(),
            voice: "alloy".into(),
            model: "tts-1".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub screen: SourceConfig,
    pub camera: SourceConfig,
    pub model_api: ModelConfig,
    /// 演示模式：无需模型即可模拟检测
    #[serde(default)]
    pub demo_mode: bool,
    pub interval_secs: u64,
    /// 送入模型的图片最大宽度（按比例缩放，节省显存/算力）
    pub image_max_width: u32,
    pub tts: TtsConfig,
    pub reminder: ReminderConfig,
    /// 是否已完成首次配置（旧配置无此字段时视为未配置，进入引导流程）
    #[serde(default)]
    pub configured: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            screen: SourceConfig {
                enabled: true,
                prompt: "写代码，推进当前的开发任务。不要浏览社交媒体、看视频或聊天。".into(),
                monitor_index: 0,
                camera_index: 0,
            },
            camera: SourceConfig {
                enabled: false,
                prompt: "专注地看着屏幕工作，不要玩手机、东张西望或离开座位。".into(),
                monitor_index: 0,
                camera_index: 0,
            },
            model_api: ModelConfig::default(),
            demo_mode: false,
            interval_secs: 15,
            image_max_width: 640,
            tts: TtsConfig::default(),
            reminder: ReminderConfig::default(),
            configured: false,
        }
    }
}

pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法获取配置目录: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建配置目录: {e}"))?;
    Ok(dir.join("config.json"))
}

pub fn load(app: &AppHandle) -> Config {
    let path = match config_path(app) {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| Config::default()),
        Err(_) => Config::default(),
    }
}

pub fn save(app: &AppHandle, cfg: &Config) -> Result<(), String> {
    let path = config_path(app)?;
    let json =
        serde_json::to_string_pretty(cfg).map_err(|e| format!("序列化配置失败: {e}"))?;
    fs::write(path, json).map_err(|e| format!("保存配置失败: {e}"))
}
