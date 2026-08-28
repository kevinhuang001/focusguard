use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vram_mb: Option<u64>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendResult {
    pub gpu: GpuInfo,
    pub model: String,
    pub interval_secs: u64,
    pub note: String,
}

/// 检测 GPU：优先 nvidia-smi（三平台通用），再按平台回退。
pub fn detect_gpu() -> GpuInfo {
    if let Some(info) = detect_nvidia() {
        return info;
    }
    #[cfg(target_os = "macos")]
    if let Some(info) = detect_macos() {
        return info;
    }
    #[cfg(target_os = "windows")]
    if let Some(info) = detect_windows() {
        return info;
    }
    #[cfg(target_os = "linux")]
    if let Some(info) = detect_linux() {
        return info;
    }
    GpuInfo {
        name: "未知".into(),
        vram_mb: None,
        source: "none".into(),
    }
}

fn detect_nvidia() -> Option<GpuInfo> {
    let out = Command::new("nvidia-smi")
        .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?.trim();
    let mut parts = line.splitn(2, ',');
    let name = parts.next()?.trim().to_string();
    let vram = parts.next()?.trim().parse::<u64>().ok();
    Some(GpuInfo {
        name,
        vram_mb: vram,
        source: "nvidia-smi".into(),
    })
}

#[cfg(target_os = "macos")]
fn detect_macos() -> Option<GpuInfo> {
    let out = Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut name: Option<String> = None;
    let mut vram: Option<u64> = None;
    for line in text.lines() {
        let l = line.trim();
        if let Some(v) = l.strip_prefix("Chipset Model:") {
            name = Some(v.trim().to_string());
        }
        if l.starts_with("VRAM") {
            if let Some(v) = l.split(':').nth(1) {
                let v = v.trim();
                let factor = if v.contains("GB") { 1024.0 } else { 1.0 };
                let num = v
                    .replace("GB", "")
                    .replace("MB", "")
                    .trim()
                    .parse::<f64>()
                    .ok();
                if let Some(n) = num {
                    vram = Some((n * factor) as u64);
                }
            }
        }
    }
    Some(GpuInfo {
        name: name?,
        vram_mb: vram,
        source: "system_profiler".into(),
    })
}

#[cfg(target_os = "windows")]
fn detect_windows() -> Option<GpuInfo> {
    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object -First 1 Name,AdapterRAM | ConvertTo-Csv -NoTypeInformation",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut name: Option<String> = None;
    let mut vram: Option<u64> = None;
    for line in text.lines() {
        let l = line.trim().trim_matches('"');
        if l.starts_with("NVIDIA") || l.starts_with("AMD") || l.starts_with("Intel") || l.contains("Radeon") || l.contains("GeForce") {
            name = Some(l.to_string());
        }
        if let Ok(bytes) = l.parse::<u64>() {
            // AdapterRAM 单位是字节，且可能被 32 位字段截断
            if bytes > 0 {
                vram = Some(bytes / (1024 * 1024));
            }
        }
    }
    Some(GpuInfo {
        name: name?,
        vram_mb: vram,
        source: "powershell".into(),
    })
}

#[cfg(target_os = "linux")]
fn detect_linux() -> Option<GpuInfo> {
    let out = Command::new("lspci").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| {
        let lower = l.to_lowercase();
        lower.contains("vga")
            || lower.contains("3d controller")
            || lower.contains("display controller")
    })?;
    let name = line
        .split(": ")
        .nth(1)
        .unwrap_or(line)
        .trim()
        .to_string();
    Some(GpuInfo {
        name,
        vram_mb: None,
        source: "lspci".into(),
    })
}

/// 根据 GPU 显存推荐模型与检测间隔（用户可手动修改）。
pub fn recommend(gpu: &GpuInfo) -> RecommendResult {
    let vram = gpu.vram_mb.unwrap_or(0);
    let (model, interval_secs, note) = match vram {
        v if v >= 8000 => (
            "qwen3-vl:8b",
            10,
            "显存充足：推荐 Qwen3-VL 8B 视觉语言模型，检测间隔 10 秒（约占用 6~7GB 显存）。",
        ),
        v if v >= 4000 => (
            "qwen3-vl:4b",
            15,
            "显存中等：推荐 Qwen3-VL 4B 视觉语言模型，检测间隔 15 秒（Q4 量化约占用 3GB 显存）。",
        ),
        v if v >= 2000 => (
            "qwen3-vl:2b",
            20,
            "显存较小：推荐 Qwen3-VL 2B 轻量视觉模型，检测间隔 20 秒。",
        ),
        _ => (
            "qwen3-vl:2b",
            30,
            "未检测到独立显卡 / 显存不足：只能 CPU 推理，建议使用 Qwen3-VL 2B 并拉长检测间隔；也可勾选「演示模式」体验完整流程。",
        ),
    };
    RecommendResult {
        gpu: gpu.clone(),
        model: model.into(),
        interval_secs,
        note: note.into(),
    }
}
