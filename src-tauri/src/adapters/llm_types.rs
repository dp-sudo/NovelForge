use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::errors::AppErrorDto;

// ── LlmError (14 spec-aligned variants) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LlmError {
    MissingApiKey,
    InvalidApiKey,
    InsufficientQuota,
    RateLimited,
    ModelNotFound,
    ContextLengthExceeded,
    MaxOutputExceeded,
    ContentPolicyViolation,
    NetworkTimeout,
    NetworkError,
    StreamInterrupted,
    InvalidJsonResponse,
    UnsupportedFeature,
    ProviderError(String),
}

impl From<LlmError> for AppErrorDto {
    fn from(e: LlmError) -> Self {
        let (code, message, recoverable) = match &e {
            LlmError::MissingApiKey => {
                ("LLM_MISSING_API_KEY", "请先在模型设置中填写 API Key", true)
            }
            LlmError::InvalidApiKey => (
                "LLM_INVALID_API_KEY",
                "API Key 认证失败，请检查密钥是否正确",
                true,
            ),
            LlmError::InsufficientQuota => (
                "LLM_INSUFFICIENT_QUOTA",
                "API 额度不足，请检查账户余额",
                false,
            ),
            LlmError::RateLimited => ("LLM_RATE_LIMITED", "请求频率过高，请稍后重试", true),
            LlmError::ModelNotFound => (
                "LLM_MODEL_NOT_FOUND",
                "当前模型名不可用，请刷新模型列表或检查拼写",
                true,
            ),
            LlmError::ContextLengthExceeded => (
                "LLM_CONTEXT_LENGTH_EXCEEDED",
                "当前上下文过长，建议减少章节范围或启用摘要压缩",
                false,
            ),
            LlmError::MaxOutputExceeded => (
                "LLM_MAX_OUTPUT_EXCEEDED",
                "输出超过最大 Token 限制，建议增大最大输出 Token",
                false,
            ),
            LlmError::ContentPolicyViolation => (
                "LLM_CONTENT_POLICY_VIOLATION",
                "内容安全策略拒绝，请调整输入内容",
                false,
            ),
            LlmError::NetworkTimeout => ("LLM_NETWORK_TIMEOUT", "连接超时，请检查网络连接", true),
            LlmError::NetworkError => ("LLM_NETWORK_ERROR", "网络错误，请检查网络连接", true),
            LlmError::StreamInterrupted => {
                ("LLM_STREAM_INTERRUPTED", "流式输出意外中断，请重试", true)
            }
            LlmError::InvalidJsonResponse => (
                "LLM_INVALID_JSON_RESPONSE",
                "模型返回的结构化数据不合法，已保留原始结果",
                false,
            ),
            LlmError::UnsupportedFeature => (
                "LLM_UNSUPPORTED_FEATURE",
                "该功能不被当前 Provider 支持",
                false,
            ),
            LlmError::ProviderError(ref msg) => ("LLM_PROVIDER_ERROR", msg.as_str(), false),
        };
        let detail = match &e {
            LlmError::ProviderError(msg) => Some(msg.clone()),
            _ => None,
        };
        AppErrorDto {
            code: code.to_string(),
            message: message.to_string(),
            detail,
            recoverable,
            suggested_action: None,
        }
    }
}

impl LlmError {
    /// Returns a user-facing error message (used by stream error chunks).
    pub fn user_message(&self) -> String {
        match self {
            LlmError::MissingApiKey => "API Key 未配置".to_string(),
            LlmError::InvalidApiKey => "API Key 认证失败".to_string(),
            LlmError::InsufficientQuota => "API 额度不足".to_string(),
            LlmError::RateLimited => "请求频率过高，请稍后重试".to_string(),
            LlmError::ModelNotFound => "模型不可用或不存在".to_string(),
            LlmError::ContextLengthExceeded => "上下文超过模型长度限制".to_string(),
            LlmError::MaxOutputExceeded => "输出超过最大 Token 限制".to_string(),
            LlmError::ContentPolicyViolation => "内容安全策略拒绝".to_string(),
            LlmError::NetworkTimeout => "连接超时，请检查网络".to_string(),
            LlmError::NetworkError => "网络错误，请检查网络连接".to_string(),
            LlmError::StreamInterrupted => "流式输出意外中断".to_string(),
            LlmError::InvalidJsonResponse => "模型返回了无效的 JSON".to_string(),
            LlmError::UnsupportedFeature => "该功能不被当前 Provider 支持".to_string(),
            LlmError::ProviderError(msg) => format!("AI 服务错误: {}", msg),
        }
    }
}

// ── Message types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: String,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

// ── UnifiedGenerateRequest (18 fields) ──

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedGenerateRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub stop: Option<Vec<String>>,
    pub stream: bool,
    pub structured_output_schema: Option<serde_json::Value>,
    pub user_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub provider_id: Option<String>,
    pub provider_config_override: Option<serde_json::Value>,
    pub timeout_ms: Option<u64>,
    pub model_parameters: Option<HashMap<String, serde_json::Value>>,
    /// Task type for route resolution (e.g. "chapter_draft", "consistency_scan")
    pub task_type: Option<String>,
}

// ── UnifiedGenerateResponse (10 fields) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedGenerateResponse {
    pub id: String,
    pub model: String,
    pub provider_id: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    pub created_at: Option<String>,
    pub finish_reason: Option<String>,
    pub raw: Option<serde_json::Value>,
    pub request_id: String,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: Option<serde_json::Value>,
}

// ── StreamChunk ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
    pub request_id: String,
    /// Error message forwarded to the frontend (non-None signals failure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Reasoning/thinking content (to be folded in UI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

// ── ProviderConfig ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub display_name: String,
    pub vendor: String,
    pub protocol: String,
    pub base_url: String,
    pub endpoint_path: Option<String>,
    pub api_key: Option<String>,
    pub auth_mode: String,
    pub auth_header_name: Option<String>,
    pub anthropic_version: Option<String>,
    pub beta_headers: Option<HashMap<String, String>>,
    pub custom_headers: Option<HashMap<String, String>>,
    pub default_model: Option<String>,
    pub timeout_ms: u64,
    pub connect_timeout_ms: u64,
    pub max_retries: u32,
    pub model_refresh_mode: Option<String>,
    pub models_path: Option<String>,
    pub last_model_refresh_at: Option<String>,
}

// ── ModelRecord (app-level llm_models table) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRecord {
    pub id: String,
    pub provider_id: String,
    pub model_name: String,
    pub display_name: Option<String>,
    pub context_window_tokens: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_json_object: bool,
    pub supports_json_schema: bool,
    pub supports_thinking: bool,
    pub supports_reasoning_effort: bool,
    pub supports_prompt_cache: bool,
    pub status: String,
    pub source: Option<String>,
    pub user_overridden: bool,
    pub last_seen_at: Option<String>,
    pub registry_version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── CapabilityReport ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityReport {
    pub provider_id: String,
    pub text_response: bool,
    pub streaming: bool,
    pub json_object: bool,
    pub json_schema: bool,
    pub tools: bool,
    pub thinking: bool,
    pub error: Option<String>,
}

// ── RefreshLog ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshLog {
    pub id: String,
    pub provider_id: String,
    pub refresh_type: String,
    pub status: String,
    pub models_added: i64,
    pub models_updated: i64,
    pub models_removed: i64,
    pub error_message: Option<String>,
    pub created_at: String,
}

// ── TaskRoute ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPoolEntry {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPoolRecord {
    pub id: String,
    pub display_name: String,
    pub role: String,
    pub enabled: bool,
    pub entries: Vec<ModelPoolEntry>,
    pub fallback_pool_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRoute {
    pub id: String,
    pub task_type: String,
    pub provider_id: String,
    pub model_id: String,
    pub fallback_provider_id: Option<String>,
    pub fallback_model_id: Option<String>,
    pub model_pool_id: Option<String>,
    pub fallback_model_pool_id: Option<String>,
    #[serde(default)]
    pub post_tasks: Vec<String>,
    pub max_retries: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
