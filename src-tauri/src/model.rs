use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub focused: bool,
    pub reason: String,
    pub source: String,
    pub model: String,
    pub backend: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
}

/// Ollama 本地推理客户端（默认 http://127.0.0.1:11434）。
/// 所有截图/摄像头帧都只发送到本机 Ollama，绝不上传云端。
#[derive(Debug, Clone)]
pub struct OllamaClient {
    pub url: String,
}

impl OllamaClient {
    pub fn new(url: String) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn detect(
        &self,
        model: &str,
        user_prompt: &str,
        image_b64: &str,
    ) -> Result<DetectionResult, String> {
        let started = Instant::now();
        let prompt = format!(
            "你是「专注状态检测助手」。观察用户当前画面，判断用户是否在专注地做以下任务：\n{}\n\n\
             请只输出一个 JSON 对象（不要输出任何其他文字），格式：{{\"focused\": true 或 false, \"reason\": \"不超过30字的中文原因\"}}。\n\
             判断标准：画面内容与任务一致 = focused=true；画面明显与任务无关（刷手机、看视频、聊天、离开等）= focused=false；无法确定时倾向于 focused=true。",
            user_prompt
        );
        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "images": [image_b64],
            "stream": false,
            "format": "json",
            "options": { "temperature": 0.2, "num_predict": 256 }
        });
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(240))
            .build()
            .map_err(|e| format!("HTTP 客户端初始化失败: {e}"))?;
        let url = format!("{}/api/generate", self.url);
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("无法连接 Ollama（{}），请确认 Ollama 已启动: {e}", self.url))?;
        if !resp.status().is_success() {
            return Err(format!("Ollama 返回错误: HTTP {}", resp.status()));
        }
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("解析 Ollama 响应失败: {e}"))?;
        let text = json["response"].as_str().unwrap_or("").trim().to_string();
        let (focused, reason) = parse_detection(&text);
        Ok(DetectionResult {
            focused,
            reason,
            source: String::new(),
            model: model.to_string(),
            backend: "ollama".into(),
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    pub async fn list_models(&self) -> Result<Vec<OllamaModel>, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;
        let url = format!("{}/api/tags", self.url);
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("连接 Ollama 失败: {e}"))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("解析失败: {e}"))?;
        let mut models = Vec::new();
        if let Some(arr) = json["models"].as_array() {
            for m in arr {
                models.push(OllamaModel {
                    name: m["name"].as_str().unwrap_or("").to_string(),
                    size: m["size"].as_u64().unwrap_or(0),
                    digest: m["digest"].as_str().unwrap_or("").to_string(),
                    modified_at: m["modified_at"].as_str().unwrap_or("").to_string(),
                });
            }
        }
        Ok(models)
    }

    pub async fn is_running(&self) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };
        let url = format!("{}/api/tags", self.url);
        matches!(client.get(&url).send().await, Ok(r) if r.status().is_success())
    }
}

pub fn ollama_installed() -> bool {
    let exe = if cfg!(windows) { "ollama.exe" } else { "ollama" };
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|dir| dir.join(exe).exists()))
        .unwrap_or(false)
}

/// 后台启动 `ollama serve`。
pub fn spawn_ollama_serve() -> Result<(), String> {
    if !ollama_installed() {
        return Err("未在 PATH 中找到 ollama，请先安装：https://ollama.com/download".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut child = std::process::Command::new("ollama");
        child.arg("serve").creation_flags(CREATE_NO_WINDOW);
        child.spawn().map_err(|e| format!("启动 Ollama 失败: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("ollama")
            .arg("serve")
            .spawn()
            .map_err(|e| format!("启动 Ollama 失败: {e}"))?;
        Ok(())
    }
}

/// 拉取模型，逐行输出进度事件 `ollama://pull`。
pub async fn pull_model_async(app: &tauri::AppHandle, model: &str) -> Result<(), String> {
    use tauri::Emitter;
    use tokio::io::AsyncBufReadExt;

    if !ollama_installed() {
        return Err("未在 PATH 中找到 ollama，请先安装：https://ollama.com/download".into());
    }
    let mut child = tokio::process::Command::new("ollama")
        .args(["pull", model])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 ollama pull 失败: {e}"))?;
    let stdout = child.stdout.take().ok_or("无法读取输出")?;
    let stderr = child.stderr.take().ok_or("无法读取错误输出")?;

    let app2 = app.clone();
    let model2 = model.to_string();
    let t1 = tokio::task::spawn(async move {
        let mut lines = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app2.emit(
                "ollama://pull",
                serde_json::json!({ "model": model2, "line": line }),
            );
        }
    });
    let app3 = app.clone();
    let model3 = model.to_string();
    let t2 = tokio::task::spawn(async move {
        let mut lines = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app3.emit(
                "ollama://pull",
                serde_json::json!({ "model": model3, "line": line }),
            );
        }
    });

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let _ = t1.await;
    let _ = t2.await;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ollama pull 退出码: {:?}", status.code()))
    }
}

fn parse_detection(text: &str) -> (bool, String) {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        let focused = v["focused"].as_bool();
        let reason = v["reason"].as_str().unwrap_or("").trim().to_string();
        if let Some(f) = focused {
            return (
                f,
                if reason.is_empty() {
                    "（模型未给出原因）".into()
                } else {
                    reason
                },
            );
        }
    }
    let t = text.to_lowercase();
    if let Some(pos) = t.find("\"focused\"") {
        let after = &t[pos..];
        if after.contains("true") {
            return (true, "模型输出无法解析，按专注处理".into());
        }
        if after.contains("false") {
            return (false, "模型输出无法解析，按开小差处理".into());
        }
    }
    (true, "模型输出无法解析，按专注处理".into())
}

/// 模拟检测：前 6 次「专注」，接着 2 次「开小差」，循环往复。
/// 用于在没有 GPU / 未装 Ollama 时体验完整流程。
pub fn mock_detect(source: &str, tick: u64) -> DetectionResult {
    let phase = tick % 8;
    let focused = phase < 6;
    DetectionResult {
        focused,
        reason: if focused {
            "（模拟）画面与任务一致，保持专注。".into()
        } else {
            "（模拟）检测到画面偏离任务，疑似开小差。".into()
        },
        source: source.into(),
        model: "mock".into(),
        backend: "mock".into(),
        duration_ms: 5,
    }
}
