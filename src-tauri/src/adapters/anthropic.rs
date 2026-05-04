use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::llm_service::LlmService;
use super::llm_types::*;

pub struct AnthropicAdapter {
    pub config: ProviderConfig,
    client: Client,
}

impl AnthropicAdapter {
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

    fn endpoint_url(&self) -> String {
        let path = self
            .config
            .endpoint_path
            .as_deref()
            .unwrap_or("/v1/messages");
        Self::join_url(&self.config.base_url, path)
    }

    fn join_url(base: &str, raw_path: &str) -> String {
        let base = base.trim_end_matches('/');
        let mut path = raw_path.trim().to_string();
        if !path.starts_with('/') {
            path = format!("/{}", path);
        }
        let mut url = format!("{}{}", base, path);
        // Deduplicate /v1/v1 when base already contains /v1 and endpoint also starts with /v1/
        while url.contains("/v1/v1") {
            url = url.replace("/v1/v1", "/v1");
        }
        url
    }

    fn build_headers(&self) -> Result<Vec<(String, String)>, LlmError> {
        let key = self
            .config
            .api_key
            .as_deref()
            .ok_or(LlmError::MissingApiKey)?;
        let mut headers = match self.config.auth_mode.as_str() {
            "bearer" => vec![("Authorization".to_string(), format!("Bearer {}", key))],
            "x-api-key" | "x_api_key" => vec![("x-api-key".to_string(), key.to_string())],
            "custom" => {
                let header_name = self
                    .config
                    .auth_header_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| {
                        LlmError::ProviderError(
                            "custom auth mode requires auth_header_name".to_string(),
                        )
                    })?;
                vec![(header_name.to_string(), key.to_string())]
            }
            _ => vec![("x-api-key".to_string(), key.to_string())],
        };
        headers.push((
            "anthropic-version".to_string(),
            self.config
                .anthropic_version
                .clone()
                .unwrap_or_else(|| "2023-06-01".to_string()),
        ));
        if let Some(ref betas) = self.config.beta_headers {
            let joined: Vec<String> = betas.values().cloned().collect();
            if !joined.is_empty() {
                headers.push(("anthropic-beta".to_string(), joined.join(",")));
            }
        }

        if let Some(ref custom_headers) = self.config.custom_headers {
            for (name, value) in custom_headers {
                let key = name.trim();
                if !key.is_empty() {
                    headers.push((key.to_string(), value.clone()));
                }
            }
        }
        Ok(headers)
    }

    fn build_request_body(&self, req: &UnifiedGenerateRequest) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": req.model,
            "max_tokens": req.max_tokens.unwrap_or(4096),
            "messages": req.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content.iter().map(|c| {
                        serde_json::json!({
                            "type": c.block_type,
                            "text": c.text
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });

        if let Some(ref system) = req.system_prompt {
            body["system"] = serde_json::json!(system);
        }
        if let Some(t) = req.temperature {
            body["temperature"] = serde_json::json!(t);
        }
        if let Some(p) = req.top_p {
            body["top_p"] = serde_json::json!(p);
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

        let text = body
            .get("content")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let finish_reason = body
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let usage = body.get("usage").map(|u| Usage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: 0,
            prompt_tokens_details: None,
        });

        let choice = Choice {
            index: 0,
            message: Message {
                role: "assistant".to_string(),
                content: vec![ContentBlock {
                    block_type: "text".to_string(),
                    text: Some(text.to_string()),
                }],
            },
            finish_reason: finish_reason.clone(),
        };

        Ok(UnifiedGenerateResponse {
            id,
            model,
            provider_id: self.config.id.clone(),
            choices: vec![choice],
            usage,
            created_at: None,
            finish_reason,
            raw: Some(body.clone()),
            request_id,
            metadata: None,
        })
    }

    fn parse_stream_event(&self, event_type: &str, data: &str) -> Option<StreamChunk> {
        let request_id = Uuid::new_v4().to_string();
        match event_type {
            "content_block_delta" => {
                let value: serde_json::Value = serde_json::from_str(data).ok()?;
                let delta = value.get("delta")?;
                let delta_type = delta
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("text_delta");
                match delta_type {
                    "thinking_delta" => {
                        let reasoning =
                            delta.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
                        if reasoning.is_empty() {
                            return None;
                        }
                        Some(StreamChunk {
                            content: String::new(),
                            finish_reason: None,
                            request_id,
                            error: None,
                            reasoning: Some(reasoning.to_string()),
                        })
                    }
                    _ => {
                        let text = delta.get("text").and_then(|v| v.as_str()).unwrap_or("");
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
                }
            }
            "message_stop" => Some(StreamChunk {
                content: String::new(),
                finish_reason: Some("end_turn".to_string()),
                request_id,
                error: None,
                reasoning: None,
            }),
            _ => None,
        }
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
            400 if msg.contains("too many tokens")
                || msg.contains("context_length")
                || msg.contains("maximum context") =>
            {
                LlmError::ContextLengthExceeded
            }
            404 => LlmError::ModelNotFound,
            400 if msg.contains("not found") || msg.contains("not support") => {
                LlmError::ModelNotFound
            }
            _ => LlmError::ProviderError(msg),
        }
    }

    async fn test_streaming(&self, model: &str, provider_id: &str) -> Result<bool, LlmError> {
        use tokio::sync::mpsc;
        let (tx, mut rx) = mpsc::channel(8);
        let stream_req = UnifiedGenerateRequest {
            model: model.to_string(),
            stream: true,
            max_tokens: Some(5),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![ContentBlock {
                    block_type: "text".to_string(),
                    text: Some("Hi".to_string()),
                }],
            }],
            provider_id: Some(provider_id.to_string()),
            ..Default::default()
        };
        let adapter_clone = AnthropicAdapter::new(self.config.clone());
        tokio::spawn(async move {
            let _ = adapter_clone.stream_text(stream_req, tx).await;
        });
        match tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv()).await {
            Ok(Some(chunk)) => Ok(!chunk.content.is_empty()),
            _ => Ok(false),
        }
    }

    async fn test_anthropic_tools(
        &self,
        model: &str,
        _provider_id: &str,
    ) -> Result<bool, LlmError> {
        let url = self.endpoint_url();
        let headers = match self.build_headers() {
            Ok(h) => h,
            Err(_) => return Ok(false),
        };
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 50,
            "messages": [{"role": "user", "content": [{"type": "text", "text": "What is the weather?"}]}],
            "tools": [{
                "name": "get_weather",
                "description": "Get weather for a location",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string", "description": "City name"}
                    },
                    "required": ["location"]
                }
            }]
        });
        let mut request = self.client.post(&url).json(&body);
        for (name, value) in &headers {
            request = request.header(name, value);
        }
        let response = request.send().await;
        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(val) = resp.json::<serde_json::Value>().await {
                    let has_tool = val
                        .get("content")
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter().any(|block| {
                                block.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                            })
                        })
                        .unwrap_or(false);
                    Ok(has_tool)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    async fn test_thinking(&self, model: &str, _provider_id: &str) -> Result<bool, LlmError> {
        let url = self.endpoint_url();
        let headers = match self.build_headers() {
            Ok(h) => h,
            Err(_) => return Ok(false),
        };
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 50,
            "messages": [{"role": "user", "content": [{"type": "text", "text": "1+1=?"}]}],
            "thinking": {"type": "enabled", "budget_tokens": 30}
        });
        let mut request = self.client.post(&url).json(&body);
        for (name, value) in &headers {
            request = request.header(name, value);
        }
        let response = request.send().await;
        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[async_trait]
impl LlmService for AnthropicAdapter {
    async fn generate_text(
        &self,
        req: UnifiedGenerateRequest,
    ) -> Result<UnifiedGenerateResponse, LlmError> {
        let url = self.endpoint_url();
        let headers = self.build_headers()?;
        let body = self.build_request_body(&req);

        let mut request = self.client.post(&url).json(&body);
        for (name, value) in &headers {
            request = request.header(name, value);
        }

        let response = request.send().await.map_err(|e| {
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
        let headers = self.build_headers()?;
        let mut body = self.build_request_body(&req);
        body["stream"] = serde_json::json!(true);

        let mut request = self.client.post(&url).json(&body);
        for (name, value) in &headers {
            request = request.header(name, value);
        }

        let response = request.send().await.map_err(|e| {
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
        let mut event_type = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|_| LlmError::StreamInterrupted)?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if let Some(event) = line.strip_prefix("event: ") {
                    event_type = event.to_string();
                } else if let Some(data) = line.strip_prefix("data: ") {
                    if let Some(chunk) = self.parse_stream_event(&event_type, data) {
                        if tx.send(chunk).await.is_err() {
                            return Ok(());
                        }
                    }
                    event_type.clear();
                }
            }
        }

        Ok(())
    }

    async fn test_connection(&self) -> Result<(), LlmError> {
        let url = self.endpoint_url();
        let headers = match self.build_headers() {
            Ok(h) => h,
            Err(e) => {
                return Err(LlmError::ProviderError(format!(
                    "Auth error: {}",
                    crate::errors::AppErrorDto::from(e).message
                )))
            }
        };

        log::info!(
            "[TEST_CONNECTION] Testing {} with model={}",
            url,
            self.config.default_model.as_deref().unwrap_or("?")
        );

        let mut request = self.client.post(&url).json(&serde_json::json!({
            "model": self.config.default_model.as_deref().unwrap_or("claude-3-haiku-20240307"),
            "max_tokens": 10,
            "messages": [{"role": "user", "content": "ping"}]
        }));
        for (name, value) in &headers {
            request = request.header(name, value);
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                LlmError::NetworkTimeout
            } else {
                LlmError::ProviderError(format!("连接失败 ({}): {}", url, e))
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
        let provider_id = self.config.id.clone();
        let model = self
            .config
            .default_model
            .as_deref()
            .unwrap_or("claude-sonnet-4-6");

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

        // 1. Test basic text via test_connection
        if self.test_connection().await.is_ok() {
            report.text_response = true;
        } else {
            report.error = Some("Connection failed".to_string());
            return Ok(report);
        }

        // 2. Test streaming — attempt a short streaming call
        let streaming = self
            .test_streaming(model, &provider_id)
            .await
            .unwrap_or(false);
        report.streaming = streaming;

        // 3. Test tools — Anthropic natively supports tool use
        let tools = self
            .test_anthropic_tools(model, &provider_id)
            .await
            .unwrap_or(true); // optimistic; most Anthropic models support tools
        report.tools = tools;

        // 4. Test thinking — Anthropic supports thinking on most recent models
        let thinking = self
            .test_thinking(model, &provider_id)
            .await
            .unwrap_or(true);
        report.thinking = thinking;

        // Anthropic has no built-in JSON mode; keep both false
        report.json_object = false;
        report.json_schema = false;

        Ok(report)
    }

    async fn fetch_models(&self) -> Result<Vec<String>, LlmError> {
        if let Some(models_path) = self
            .config
            .models_path
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            let url = Self::join_url(&self.config.base_url, models_path);
            let headers = self.build_headers()?;

            let mut request = self.client.get(&url);
            for (name, value) in &headers {
                request = request.header(name, value);
            }

            let response = request.send().await.map_err(|e| {
                if e.is_timeout() {
                    LlmError::NetworkTimeout
                } else {
                    LlmError::NetworkError
                }
            })?;

            if response.status().is_success() {
                let body: serde_json::Value = response
                    .json()
                    .await
                    .map_err(|_| LlmError::InvalidJsonResponse)?;
                let from_data = body
                    .get("data")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| m.get("id").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();
                if !from_data.is_empty() {
                    return Ok(from_data);
                }

                let from_models = body
                    .get("models")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| {
                                m.get("name")
                                    .and_then(|n| n.as_str())
                                    .or_else(|| m.get("id").and_then(|n| n.as_str()))
                            })
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();
                if !from_models.is_empty() {
                    return Ok(from_models);
                }
            }
        }

        Ok(self
            .config
            .default_model
            .as_ref()
            .map(|m| vec![m.clone()])
            .unwrap_or_default())
    }
}
