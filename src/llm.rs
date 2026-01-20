// src/llm.rs

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize; // 如果需要 Serialize 也要加
use serde_json::json;

// --- 1. LLM 相關的設定結構 (搬移至此並設為 pub) ---

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    pub provider: String, // "gemini" or "ollama"
    pub gemini: Option<GeminiConfig>,
    pub ollama: Option<OllamaConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

// --- 2. 定義 Trait ---

#[async_trait]
pub trait LlmClient: Send + Sync {
    /// json_mode: 用來告訴 LLM 是否強制輸出 JSON 格式
    async fn generate(
        &self,
        system_prompt: &str,
        user_content: &str,
        json_mode: bool,
    ) -> Result<String>;
}

// --- 3. Gemini 實作 ---

struct GeminiClient {
    client: Client,
    config: GeminiConfig,
}

#[async_trait]
impl LlmClient for GeminiClient {
    async fn generate(
        &self,
        system_prompt: &str,
        user_content: &str,
        json_mode: bool,
    ) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.config.model, self.config.api_key
        );

        let generation_config = if json_mode {
            json!({
                "temperature": 0.2,
                "responseMimeType": "application/json"
            })
        } else {
            json!({
                "temperature": 0.2
            })
        };

        let payload = json!({
            "system_instruction": {
            "parts": [{"text": system_prompt}]
            },
            "contents": [{
                "parts": [{ "text": user_content }]
            }],
            "generationConfig": generation_config
        });

        let res = self.client.post(&url).json(&payload).send().await?;

        if !res.status().is_success() {
            let err_text = res.text().await?;
            bail!("Gemini API Error: {}", err_text);
        }

        let body: serde_json::Value = res.json().await?;
        let text = body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .context("無法解析 Gemini 回傳內容")?
            .to_string();

        Ok(text)
    }
}

// --- 4. Ollama 實作 ---

struct OllamaClient {
    client: Client,
    config: OllamaConfig,
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn generate(
        &self,
        system_prompt: &str,
        user_content: &str,
        json_mode: bool,
    ) -> Result<String> {
        let url = format!("{}/api/chat", self.config.base_url.trim_end_matches('/'));

        let mut payload = json!({
            "model": self.config.model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_content }
            ],
            "stream": false,
            "options": {
                "temperature": 0.2,
                "num_ctx": 4096
            }
        });

        if json_mode {
            payload
                .as_object_mut()
                .unwrap()
                .insert("format".to_string(), json!("json"));
        }

        let res = self.client.post(&url).json(&payload).send().await?;

        if !res.status().is_success() {
            let err_text = res.text().await?;
            bail!("Ollama API Error: {}", err_text);
        }

        let body: serde_json::Value = res.json().await?;

        let text = body["message"]["content"]
            .as_str()
            .context("無法解析 Ollama 回傳內容")?
            .to_string();

        Ok(text)
    }
}

// --- 5. 工廠模式 (Factory) ---

pub fn create_llm_client(config: &LlmConfig) -> Result<Box<dyn LlmClient>> {
    let client = Client::new();
    match config.provider.as_str() {
        "gemini" => {
            let conf = config.gemini.as_ref().context("未設定 gemini 區塊")?;
            Ok(Box::new(GeminiClient {
                client,
                config: conf.clone(),
            }))
        }
        "ollama" => {
            let conf = config.ollama.as_ref().context("未設定 ollama 區塊")?;
            Ok(Box::new(OllamaClient {
                client,
                config: conf.clone(),
            }))
        }
        _ => bail!("未知的 LLM Provider: {}", config.provider),
    }
}
