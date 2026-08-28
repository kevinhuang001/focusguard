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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderConfig {
    /// none | system | voice | both
    pub kind: String,
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
            voice_text: "注意！检测到你似乎没有在专注当前任务，请尽快回到正轨！".into(),
            cooldown_secs: 30,
            miss_threshold: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub screen: SourceConfig,
    pub camera: SourceConfig,
    /// ollama | mock
    pub backend: String,
    pub model: String,
    pub interval_secs: u64,
    /// 送入模型的图片最大宽度（按比例缩放，节省显存/算力）
    pub image_max_width: u32,
    pub ollama_url: String,
    pub reminder: ReminderConfig,
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
            backend: "ollama".into(),
            model: "qwen2.5vl:3b".into(),
            interval_secs: 15,
            image_max_width: 640,
            ollama_url: "http://127.0.0.1:11434".into(),
            reminder: ReminderConfig::default(),
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
