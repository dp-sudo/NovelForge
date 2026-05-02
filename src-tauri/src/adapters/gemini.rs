use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::llm_service::LlmService;
use super::llm_types::*;

pub struct GeminiAdapter {
    pub config: ProviderConfig,
    client: Client,
}

impl GeminiAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let timeout = std::time::Duration::from_millis(config.timeout_ms);
        let connect_timeout = std::time::Duration::from_millis(config.connect_timeout_ms);
        let client = Client::builder()
            .timeout(timeout)
            .connect_timeout(connect_timeout)
            .build()
            .unwrap_or_default();
        Self { config, client }
    }

    fn endpoint_url(&self, model: &str, stream: bool) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        let action = if stream {
            "streamGenerateContent"
        } else {
            "generateContent"
        };
        format!("{}/models/{}:{}", base, model, action)
    }

    fn models_endpoint_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{}/models", base)
    }

    fn with_api_key(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let key = self
            .config
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(value) = key {
            request.header("x-goog-api-key", value)
        } else {
            request
        }
    }

    fn build_request_body(&self, req: &UnifiedGenerateRequest) -> serde_json::Value {
        let mut contents = Vec::new();
        for msg in &req.messages {
            let parts: Vec<serde_json::Value> = msg
                .content
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "text": c.text
                    })
                })
                .collect();
            contents.push(serde_json::json!({
                "role": msg.role,
                "parts": parts
            }));
        }

        let mut body = serde_json::json!({
            "contents": contents,
        });

        let mut config = serde_json::Map::new();
        if let Some(t) = req.temperature {
            config.insert("temperature".to_string(), serde_json::json!(t));
        }
        if let Some(m) = req.max_tokens {
            config.insert("maxOutputTokens".to_string(), serde_json::json!(m));
        }
        if let Some(p) = req.top_p {
            config.insert("topP".to_string(), serde_json::json!(p));
        }
        if let Some(ref stop) = req.stop {
            config.insert("stopSequences".to_string(), serde_json::json!(stop));
        }
        if !config.is_empty() {
            body["generationConfig"] = serde_json::Value::Object(config);
        }

        if let Some(ref system) = req.system_prompt {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": system}]
            });
        }

        body
    }

    fn parse_response(
        &self,
        body: &serde_json::Value,
        req: &UnifiedGenerateRequest,
    ) -> Result<UnifiedGenerateResponse, LlmError> {
        let request_id = Uuid::new_v4().to_string();
        let model = req.model.clone();

        let choices = body
            .get("candidates")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let text = c
                            .get("content")
                            .and_then(|ct| ct.get("parts"))
                            .and_then(|p| p.as_array())
                            .and_then(|parts| parts.first())
                            .and_then(|part| part.get("text"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let finish = c.get("finishReason").and_then(|v| v.as_str());
                        Choice {
                            index: i as u32,
                            message: Message {
                                role: "assistant".to_string(),
                                content: vec![ContentBlock {
                                    block_type: "text".to_string(),
                                    text: Some(text.to_string()),
                                }],
                            },
                            finish_reason: finish.map(|s| s.to_string()),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = body.get("usageMetadata").map(|u| Usage {
            prompt_tokens: u
                .get("promptTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            completion_tokens: u
                .get("candidatesTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u
                .get("totalTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            prompt_tokens_details: None,
        });

        Ok(UnifiedGenerateResponse {
            id: request_id.clone(),
            model,
            provider_id: self.config.id.clone(),
            choices,
            usage,
            created_at: None,
            finish_reason: None,
            raw: Some(body.clone()),
            request_id,
            metadata: None,
        })
    }

    fn parse_stream_chunk(&self, data: &str) -> Option<StreamChunk> {
        let request_id = Uuid::new_v4().to_string();
        if data == "[DONE]" {
            return Some(StreamChunk {
                content: String::new(),
                finish_reason: Some("stop".to_string()),
                request_id,
                error: None,
                reasoning: None,
            });
        }
        let value: serde_json::Value = serde_json::from_str(data).ok()?;
        let text = value
            .get("candidates")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("content"))
            .and_then(|ct| ct.get("parts"))
            .and_then(|p| p.as_array())
            .and_then(|parts| parts.first())
            .and_then(|part| part.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if text.is_empty() {
            return None;
        }
        Some(StreamChunk {
            content: text.to_string(),
            finish_reason: None,
            request_id,
            error: None,
            reasoning: None,
        })
    }

    fn map_http_error(&self, status: reqwest::StatusCode, body: &serde_json::Value) -> LlmError {
        let msg = body
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        match status.as_u16() {
            401 | 403 => LlmError::InvalidApiKey,
            429 => LlmError::RateLimited,
            404 => LlmError::ModelNotFound,
            400 if msg.contains("not found") || msg.contains("not support") => {
                LlmError::ModelNotFound
            }
            _ => LlmError::ProviderError(msg),
        }
    }
}

#[async_trait]
impl LlmService for GeminiAdapter {
    async fn generate_text(
        &self,
        req: UnifiedGenerateRequest,
    ) -> Result<UnifiedGenerateResponse, LlmError> {
        let url = self.endpoint_url(&req.model, false);
        let body = self.build_request_body(&req);

        let response = self
            .with_api_key(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::NetworkTimeout
                } else {
                    LlmError::NetworkError
                }
            })?;

        let status = response.status();
        let response_body: serde_json::Value = response
            .json()
            .await
            .map_err(|_| LlmError::InvalidJsonResponse)?;

        if !status.is_success() {
            return Err(self.map_http_error(status, &response_body));
        }

        self.parse_response(&response_body, &req)
    }

    async fn stream_text(
        &self,
        req: UnifiedGenerateRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<(), LlmError> {
        let url = self.endpoint_url(&req.model, true);
        let body = self.build_request_body(&req);

        let response = self
            .with_api_key(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::NetworkTimeout
                } else {
                    LlmError::NetworkError
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body: serde_json::Value = response.json().await.unwrap_or_default();
            return Err(self.map_http_error(status, &body));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|_| LlmError::StreamInterrupted)?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if let Some(chunk) = self.parse_stream_chunk(data.trim()) {
                        if tx.send(chunk).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn test_connection(&self) -> Result<(), LlmError> {
        let url = self.models_endpoint_url();

        let response = self
            .with_api_key(self.client.get(&url))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::NetworkTimeout
                } else {
                    LlmError::NetworkError
                }
            })?;

        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_default();
        Err(self.map_http_error(status, &body))
    }

    async fn detect_capabilities(&self) -> Result<CapabilityReport, LlmError> {
        let text_ok = self.test_connection().await.is_ok();
        Ok(CapabilityReport {
            provider_id: self.config.id.clone(),
            text_response: text_ok,
            streaming: text_ok,
            json_object: text_ok,
            json_schema: text_ok,
            tools: false,
            thinking: text_ok,
            error: if text_ok {
                None
            } else {
                Some("Connection failed".to_string())
            },
        })
    }

    async fn fetch_models(&self) -> Result<Vec<String>, LlmError> {
        let url = self.models_endpoint_url();

        let response = self
            .with_api_key(self.client.get(&url))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::NetworkTimeout
                } else {
                    LlmError::NetworkError
                }
            })?;

        if !response.status().is_success() {
            return Ok(self
                .config
                .default_model
                .as_ref()
                .map(|m| vec![m.clone()])
                .unwrap_or_default());
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|_| LlmError::InvalidJsonResponse)?;
        let names = body
            .get("models")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(names)
    }
}
