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
pub struct ModelInfo {
    pub id: String,
}

/// OpenAI 兼容的视觉模型客户端。
/// 只需一个 base_url（如 https://api.openai.com/v1 或 http://localhost:11434/v1），
/// 通过 `/chat/completions` 多模态接口调用。应用不代理模型的下载/安装。
#[derive(Debug, Clone)]
pub struct OpenAiClient {
    pub base_url: String,
    pub api_key: String,
}

impl OpenAiClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    async fn client(&self, timeout: Duration) -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("HTTP 客户端初始化失败: {e}"))
    }

    async fn send(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<(reqwest::StatusCode, serde_json::Value), String> {
        let req = if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(self.api_key.clone())
        };
        let resp = req
            .send()
            .await
            .map_err(|e| format!("无法连接模型服务（{}），请确认服务已启动、URL 正确: {e}", self.base_url))?;
        let status = resp.status();
        if !status.is_success() {
            let eb: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = eb["error"]["message"]
                .as_str()
                .or_else(|| eb["error"].as_str())
                .unwrap_or("")
                .to_string();
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(format!(
                    "模型服务返回 404：{msg}。请检查 base_url 是否以 /v1 结尾、模型名是否正确。"
                ));
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(format!("模型服务返回 401：{msg}。请检查 API Key 是否正确。"));
            }
            return Err(format!("模型服务返回错误: HTTP {status} {msg}").trim_end().to_string());
        }
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("解析模型服务响应失败: {e}"))?;
        Ok((status, json))
    }

    /// 调用多模态 `/chat/completions` 判断是否专注。
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
        let image_data_url = format!("data:image/jpeg;base64,{image_b64}");
        let body = serde_json::json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": prompt },
                    { "type": "image_url", "image_url": { "url": image_data_url } }
                ]
            }],
            "max_tokens": 256,
            "temperature": 0.2,
            "stream": false
        });
        let url = format!("{}/chat/completions", self.base_url);
        let client = self.client(Duration::from_secs(240)).await?;
        let (_, json) = self.send(client.post(&url).json(&body)).await?;
        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
        if text.is_empty() {
            return Err("模型未返回内容，请检查模型是否支持图像输入".into());
        }
        let (focused, reason) = parse_detection(&text);
        Ok(DetectionResult {
            focused,
            reason,
            source: String::new(),
            model: model.to_string(),
            backend: "openai".into(),
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    /// 列出模型（GET /models），也用于测试连接。
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let client = self.client(Duration::from_secs(8)).await?;
        let url = format!("{}/models", self.base_url);
        let (_, json) = self.send(client.get(&url)).await?;
        let mut out = Vec::new();
        if let Some(arr) = json["data"].as_array() {
            for m in arr {
                if let Some(id) = m["id"].as_str() {
                    out.push(ModelInfo { id: id.to_string() });
                }
            }
        }
        Ok(out)
    }
}

/// 模拟检测：前 6 次「专注」，接着 2 次「开小差」，循环往复。
/// 用于没有可用模型时演示完整流程。
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
