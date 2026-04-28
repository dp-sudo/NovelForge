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
            LlmError::MissingApiKey => (
                "LLM_MISSING_API_KEY",
                "API key not configured for this provider",
                true,
            ),
            LlmError::InvalidApiKey => (
                "LLM_INVALID_API_KEY",
                "Authentication failed — invalid API key",
                true,
            ),
            LlmError::InsufficientQuota => (
                "LLM_INSUFFICIENT_QUOTA",
                "Insufficient quota from provider",
                false,
            ),
            LlmError::RateLimited => ("LLM_RATE_LIMITED", "Rate-limited by provider", true),
            LlmError::ModelNotFound => (
                "LLM_MODEL_NOT_FOUND",
                "Specified model is not available",
                true,
            ),
            LlmError::ContextLengthExceeded => (
                "LLM_CONTEXT_LENGTH_EXCEEDED",
                "Input exceeds model context window",
                false,
            ),
            LlmError::MaxOutputExceeded => (
                "LLM_MAX_OUTPUT_EXCEEDED",
                "Output exceeded maximum tokens",
                false,
            ),
            LlmError::ContentPolicyViolation => (
                "LLM_CONTENT_POLICY_VIOLATION",
                "Content policy violation",
                false,
            ),
            LlmError::NetworkTimeout => ("LLM_NETWORK_TIMEOUT", "Request timed out", true),
            LlmError::NetworkError => (
                "LLM_NETWORK_ERROR",
                "Network error communicating with provider",
                true,
            ),
            LlmError::StreamInterrupted => (
                "LLM_STREAM_INTERRUPTED",
                "Stream interrupted unexpectedly",
                true,
            ),
            LlmError::InvalidJsonResponse => (
                "LLM_INVALID_JSON_RESPONSE",
                "Invalid JSON from provider",
                false,
            ),
            LlmError::UnsupportedFeature => (
                "LLM_UNSUPPORTED_FEATURE",
                "Feature not supported by provider",
                false,
            ),
            LlmError::ProviderError(_) => {
                ("LLM_PROVIDER_ERROR", "Provider returned an error", false)
            }
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
pub struct TaskRoute {
    pub id: String,
    pub task_type: String,
    pub provider_id: String,
    pub model_id: String,
    pub fallback_provider_id: Option<String>,
    pub fallback_model_id: Option<String>,
    pub max_retries: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ── ModelInfo (registry) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub model_id: String,
    pub provider: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_streaming: bool,
    pub supports_tools: bool,
}
