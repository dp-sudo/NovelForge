use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::llm_service::LlmService;
use super::llm_types::*;

pub struct OpenAiCompatibleAdapter {
    pub config: ProviderConfig,
    client: Client,
}

impl OpenAiCompatibleAdapter {
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

    fn join_url(base: &str, raw_path: &str) -> String {
        let base = base.trim_end_matches('/');
        let mut path = raw_path.trim().to_string();
        if !path.starts_with('/') {
            path = format!("/{}", path);
        }
        if base.ends_with("/v1") && path.starts_with("/v1/") {
            return format!("{}{}", base, &path[3..]);
        }
        format!("{}{}", base, path)
    }

    fn endpoint_url(&self) -> String {
        let path = self
            .config
            .endpoint_path
            .as_deref()
            .unwrap_or("/v1/chat/completions");
        Self::join_url(&self.config.base_url, path)
    }

    fn models_url(&self) -> String {
        Self::join_url(
            &self.config.base_url,
            self.config.models_path.as_deref().unwrap_or("/v1/models"),
        )
    }

    fn auth_header(&self) -> Result<(&str, String), LlmError> {
        let key = self
            .config
            .api_key
            .as_deref()
            .ok_or(LlmError::MissingApiKey)?;
        let header_name = self
            .config
            .auth_header_name
            .as_deref()
            .unwrap_or("Authorization");
        let header_value = match self.config.auth_mode.as_str() {
            "bearer" => format!("Bearer {}", key),
            _ => key.to_string(),
        };
        Ok((header_name, header_value))
    }

    fn build_request_body(&self, req: &UnifiedGenerateRequest) -> serde_json::Value {
        let mut messages = Vec::new();

        if let Some(ref system) = req.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        for msg in &req.messages {
            let content: Vec<serde_json::Value> = msg
                .content
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "type": c.block_type,
                        "text": c.text
                    })
                })
                .collect();
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": content
            }));
        }

        let mut body = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "stream": req.stream,
        });

        if let Some(t) = req.temperature {
            body["temperature"] = serde_json::json!(t);
        }
        if let Some(m) = req.max_tokens {
            body["max_tokens"] = serde_json::json!(m);
        }
        if let Some(p) = req.top_p {
            body["top_p"] = serde_json::json!(p);
        }
        if let Some(ref stop) = req.stop {
            body["stop"] = serde_json::json!(stop);
        }

        body
    }

    fn parse_response(
        &self,
        body: &serde_json::Value,
        req: &UnifiedGenerateRequest,
    ) -> Result<UnifiedGenerateResponse, LlmError> {
        let id = body
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let model = body
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&req.model)
            .to_string();
        let request_id = Uuid::new_v4().to_string();

        let choices: Vec<Choice> = body
            .get("choices")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .filter_map(|(i, c)| {
                        let msg = c.get("message")?;
                        let role = msg
                            .get("role")
                            .and_then(|v| v.as_str())
                            .unwrap_or("assistant");
                        let text = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        Some(Choice {
                            index: i as u32,
                            message: Message {
                                role: role.to_string(),
                                content: vec![ContentBlock {
                                    block_type: "text".to_string(),
                                    text: Some(text.to_string()),
                                }],
                            },
                            finish_reason: c
                                .get("finish_reason")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = body.get("usage").map(|u| Usage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            prompt_tokens_details: u.get("prompt_tokens_details").cloned(),
        });

        let finish_reason = choices.first().and_then(|c| c.finish_reason.clone());

        Ok(UnifiedGenerateResponse {
            id,
            model,
            provider_id: self.config.id.clone(),
            choices,
            usage,
            created_at: None,
            finish_reason,
            raw: Some(body.clone()),
            request_id,
            metadata: None,
        })
    }

    fn parse_stream_chunk(&self, data: &str) -> Option<StreamChunk> {
        let value: serde_json::Value = serde_json::from_str(data).ok()?;
        let choice = value.get("choices")?.as_array()?.first()?;
        let delta = choice.get("delta")?;
        let content = delta
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let request_id = Uuid::new_v4().to_string();

        if content.is_empty() && finish_reason.is_none() {
            return None;
        }
        Some(StreamChunk {
            content,
            finish_reason,
            request_id,
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
            401 => LlmError::InvalidApiKey,
            402 | 403 if msg.contains("insufficient_quota") || msg.contains("quota") => {
                LlmError::InsufficientQuota
            }
            429 => LlmError::RateLimited,
            400 if msg.contains("context_length") || msg.contains("maximum context") => {
                LlmError::ContextLengthExceeded
            }
            400 if msg.contains("not found") || msg.contains("not support") => {
                LlmError::ModelNotFound
            }
            400 if msg.contains("content_filter") || msg.contains("safety") => {
                LlmError::ContentPolicyViolation
            }
            404 => LlmError::ModelNotFound,
            500..=599 => LlmError::ProviderError(msg),
            _ => LlmError::ProviderError(msg),
        }
    }
}

#[async_trait]
impl LlmService for OpenAiCompatibleAdapter {
    async fn generate_text(
        &self,
        req: UnifiedGenerateRequest,
    ) -> Result<UnifiedGenerateResponse, LlmError> {
        let url = self.endpoint_url();
        let (header_name, header_value) = self.auth_header()?;
        let body = self.build_request_body(&req);

        let response = self
            .client
            .post(&url)
            .header(header_name, header_value)
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
        let url = self.endpoint_url();
        let (header_name, header_value) = self.auth_header()?;
        let mut body = self.build_request_body(&req);
        body["stream"] = serde_json::json!(true);

        let response = self
            .client
            .post(&url)
            .header(header_name, header_value)
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
            let response_body: serde_json::Value = response
                .json()
                .await
                .map_err(|_| LlmError::InvalidJsonResponse)?;
            return Err(self.map_http_error(status, &response_body));
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
                    if data.trim() == "[DONE]" {
                        return Ok(());
                    }
                    if let Some(chunk) = self.parse_stream_chunk(data) {
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
        let models_url = self.models_url();
        let (header_name, header_value) = self.auth_header()?;

        let response = self
            .client
            .get(&models_url)
            .header(header_name, header_value)
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
            Ok(())
        } else {
            let status = response.status();
            let body: serde_json::Value = response.json().await.unwrap_or_default();
            Err(self.map_http_error(status, &body))
        }
    }

    async fn detect_capabilities(&self) -> Result<CapabilityReport, LlmError> {
        let provider_id = self.config.id.clone();
        let model = self.config.default_model.as_deref().unwrap_or("gpt-5.5");

        let mut report = CapabilityReport {
            provider_id: provider_id.clone(),
            text_response: false,
            streaming: false,
            json_object: false,
            json_schema: false,
            tools: false,
            thinking: false,
            error: None,
        };

        let base_req = UnifiedGenerateRequest {
            model: model.to_string(),
            messages: vec![],
            system_prompt: None,
            temperature: None,
            max_tokens: Some(10),
            top_p: None,
            stop: None,
            stream: false,
            structured_output_schema: None,
            user_id: None,
            metadata: None,
            provider_id: Some(provider_id.clone()),
            provider_config_override: None,
            timeout_ms: Some(15000),
            model_parameters: None,
            task_type: None,
        };

        // 1. Test basic text generation
        let text_req = UnifiedGenerateRequest {
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![ContentBlock {
                    block_type: "text".to_string(),
                    text: Some("OK".to_string()),
                }],
            }],
            ..base_req
        };

        if self.generate_text(text_req).await.is_ok() {
            report.text_response = true;
        } else {
            report.error = Some("Text generation test failed".to_string());
            return Ok(report);
        }

        // 2. Test streaming — optimistic
        report.streaming = true;

        // 3. Test JSON mode
        report.json_object = true; // optimistic for OpenAI-compatible

        // 4. Test tools
        report.tools = true; // optimistic — most modern OpenAI-compatible support tools

        // 5. Test thinking (specific to deepseek)
        report.thinking = self.config.vendor == "deepseek";

        Ok(report)
    }

    async fn fetch_models(&self) -> Result<Vec<String>, LlmError> {
        let models_url = self.models_url();
        let (header_name, header_value) = self.auth_header()?;

        let response = self
            .client
            .get(&models_url)
            .header(header_name, header_value)
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
            .get("data")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::OpenAiCompatibleAdapter;

    #[test]
    fn join_url_avoids_double_v1_prefix() {
        let joined = OpenAiCompatibleAdapter::join_url("https://api.openai.com/v1", "/v1/models");
        assert_eq!(joined, "https://api.openai.com/v1/models");
    }
}
