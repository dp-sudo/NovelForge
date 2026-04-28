use crate::adapters::anthropic::AnthropicAdapter;
use crate::adapters::gemini::GeminiAdapter;
use crate::adapters::llm_service::LlmService;
use crate::adapters::llm_types::ProviderConfig;
use crate::adapters::openai_compatible::OpenAiCompatibleAdapter;
use crate::errors::AppErrorDto;
use crate::infra::{app_database, credential_manager};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSettings {
    pub font_size: i32,
    pub line_height: f64,
    pub autosave_interval: i32,
    pub narrative_pov: String,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_size: 16,
            line_height: 1.75,
            autosave_interval: 5,
            narrative_pov: "third_limited".to_string(),
        }
    }
}

const EDITOR_SETTINGS_KEY: &str = "editor_settings";

#[derive(Default)]
pub struct SettingsService;

impl SettingsService {
    /// List all configured providers.
    pub fn list_providers(&self) -> Result<Vec<ProviderConfig>, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        let mut configs = app_database::load_all_providers(&conn)?;
        for config in &mut configs {
            if let Ok(Some(key)) = credential_manager::load_api_key(&config.id) {
                config.api_key = Some(mask_api_key(&key));
            }
        }
        Ok(configs)
    }

    /// Save a provider config and its API key.
    pub fn save_provider(
        &self,
        mut config: ProviderConfig,
        api_key: Option<String>,
    ) -> Result<ProviderConfig, AppErrorDto> {
        validate_provider_config(&mut config)?;

        let now = crate::infra::time::now_iso();
        let conn = app_database::open_or_create()?;

        if let Some(ref key) = api_key {
            if !key.is_empty() {
                credential_manager::save_api_key(&config.id, key)?;
            }
        }

        app_database::upsert_provider(&conn, &config, &now)?;

        let mut result = config;
        if let Ok(Some(key)) = credential_manager::load_api_key(&result.id) {
            result.api_key = Some(mask_api_key(&key));
        }
        Ok(result)
    }

    /// Load a single provider config (with masked API Key).
    pub fn load_provider(&self, provider_id: &str) -> Result<ProviderConfig, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        let mut config = app_database::load_provider(&conn, provider_id)?.ok_or_else(|| {
            AppErrorDto::new(
                "PROVIDER_NOT_FOUND",
                &format!("Provider '{}' not found", provider_id),
                true,
            )
        })?;

        if let Ok(Some(key)) = credential_manager::load_api_key(provider_id) {
            config.api_key = Some(mask_api_key(&key));
        }
        Ok(config)
    }

    /// Delete a provider and its API key.
    pub fn delete_provider(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        let conn = app_database::open_or_create()?;
        app_database::delete_provider(&conn, provider_id)?;
        let _ = credential_manager::delete_api_key(provider_id);
        Ok(())
    }

    /// Test connection against the provider endpoint and return detailed status.
    pub async fn test_connection(&self, provider_id: &str) -> Result<String, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        let mut config = app_database::load_provider(&conn, provider_id)?.ok_or_else(|| {
            AppErrorDto::new(
                "PROVIDER_NOT_FOUND",
                &format!("Provider '{}' not found", provider_id),
                true,
            )
        })?;

        if let Ok(Some(key)) = credential_manager::load_api_key(provider_id) {
            config.api_key = Some(key);
        }

        let adapter = build_adapter(config);
        adapter.test_connection().await.map_err(AppErrorDto::from)?;
        Ok("连接成功！".to_string())
    }

    /// Load editor settings. Returns defaults if none are saved.
    pub fn load_editor_settings(&self) -> Result<EditorSettings, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        match app_database::load_app_setting(&conn, EDITOR_SETTINGS_KEY)? {
            Some(json) => {
                serde_json::from_str(&json).map_err(|e| {
                    AppErrorDto::new(
                        "DESERIALIZE_FAILED",
                        "Cannot parse editor settings",
                        true,
                    )
                    .with_detail(e.to_string())
                })
            }
            None => Ok(EditorSettings::default()),
        }
    }

    /// Save editor settings.
    pub fn save_editor_settings(&self, settings: &EditorSettings) -> Result<(), AppErrorDto> {
        let json = serde_json::to_string(settings).map_err(|e| {
            AppErrorDto::new(
                "SERIALIZE_FAILED",
                "Cannot serialize editor settings",
                false,
            )
            .with_detail(e.to_string())
        })?;
        let now = crate::infra::time::now_iso();
        let conn = app_database::open_or_create()?;
        app_database::save_app_setting(&conn, EDITOR_SETTINGS_KEY, &json, &now)
    }
}

fn mask_api_key(key: &str) -> String {
    let len = key.len();
    if len > 12 {
        // e.g. "sk-proj-1234567890abcdefghijklmn" → "sk-proj-1234••••••••••••lmn"
        format!("{}••••••••••••{}", &key[..8], &key[len - 4..])
    } else if len > 8 {
        format!("{}••••{}", &key[..4], &key[len - 4..])
    } else if !key.is_empty() {
        "••••••••".to_string()
    } else {
        String::new()
    }
}

fn validate_provider_config(config: &mut ProviderConfig) -> Result<(), AppErrorDto> {
    config.id = config.id.trim().to_string();
    config.display_name = config.display_name.trim().to_string();
    config.vendor = config.vendor.trim().to_string();
    config.protocol = config.protocol.trim().to_string();
    config.base_url = config.base_url.trim().trim_end_matches('/').to_string();
    config.auth_mode = config.auth_mode.trim().to_string();

    if config.id.is_empty() {
        return Err(AppErrorDto::new(
            "INVALID_PROVIDER_ID",
            "Provider id cannot be empty",
            true,
        ));
    }
    if config.display_name.is_empty() {
        return Err(AppErrorDto::new(
            "INVALID_PROVIDER_NAME",
            "Provider display name cannot be empty",
            true,
        ));
    }
    if config.base_url.is_empty() {
        return Err(AppErrorDto::new(
            "INVALID_BASE_URL",
            "Provider base URL cannot be empty",
            true,
        ));
    }
    let parsed = reqwest::Url::parse(&config.base_url).map_err(|err| {
        AppErrorDto::new("INVALID_BASE_URL", "Provider base URL is invalid", true)
            .with_detail(err.to_string())
    })?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppErrorDto::new(
            "INVALID_BASE_URL_SCHEME",
            "Provider base URL must use http:// or https://",
            true,
        ));
    }

    if config.timeout_ms == 0 || config.connect_timeout_ms == 0 {
        return Err(AppErrorDto::new(
            "INVALID_TIMEOUT",
            "Timeout values must be greater than 0",
            true,
        ));
    }
    if config.connect_timeout_ms > config.timeout_ms {
        return Err(AppErrorDto::new(
            "INVALID_TIMEOUT",
            "Connect timeout cannot exceed total timeout",
            true,
        ));
    }
    if config.max_retries > 8 {
        return Err(AppErrorDto::new(
            "INVALID_MAX_RETRIES",
            "Max retries cannot exceed 8",
            true,
        ));
    }

    if config.protocol == "custom_anthropic_compatible" {
        let endpoint = config.endpoint_path.as_deref().map(str::trim).unwrap_or("");
        if endpoint.is_empty() {
            config.endpoint_path = Some("/messages".to_string());
        }
    }

    if config.protocol == "custom_openai_compatible" {
        let endpoint = config.endpoint_path.as_deref().map(str::trim).unwrap_or("");
        if endpoint.is_empty() {
            config.endpoint_path = Some("/chat/completions".to_string());
        }
    }

    if config.auth_mode.is_empty() {
        config.auth_mode = "bearer".to_string();
    }

    if config.auth_mode == "custom" {
        let custom_header = config
            .auth_header_name
            .as_deref()
            .map(str::trim)
            .unwrap_or("");
        if custom_header.is_empty() {
            return Err(AppErrorDto::new(
                "INVALID_AUTH_HEADER",
                "Custom auth mode requires authHeaderName",
                true,
            ));
        }
    }

    if config.vendor == "custom" {
        let model = config.default_model.as_deref().map(str::trim).unwrap_or("");
        if model.is_empty() {
            return Err(AppErrorDto::new(
                "INVALID_DEFAULT_MODEL",
                "Custom provider requires a default model",
                true,
            ));
        }
    }

    if let Some(path) = config
        .models_path
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        if !path.starts_with('/') {
            return Err(AppErrorDto::new(
                "INVALID_MODELS_PATH",
                "Model list path must start with '/'",
                true,
            ));
        }
    }

    Ok(())
}

fn build_adapter(config: ProviderConfig) -> Box<dyn LlmService> {
    let is_anthropic_protocol = matches!(
        config.protocol.as_str(),
        "anthropic_messages" | "custom_anthropic_compatible"
    );
    let is_gemini_protocol = matches!(config.protocol.as_str(), "gemini_generate_content");

    match config.vendor.as_str() {
        "anthropic" | "minimax" => Box::new(AnthropicAdapter::new(config)),
        "gemini" => Box::new(GeminiAdapter::new(config)),
        _ if is_anthropic_protocol => Box::new(AnthropicAdapter::new(config)),
        _ if is_gemini_protocol => Box::new(GeminiAdapter::new(config)),
        _ => Box::new(OpenAiCompatibleAdapter::new(config)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_custom_config() -> ProviderConfig {
        ProviderConfig {
            id: "custom".to_string(),
            display_name: "Custom".to_string(),
            vendor: "custom".to_string(),
            protocol: "custom_openai_compatible".to_string(),
            base_url: "http://localhost:8000/v1".to_string(),
            endpoint_path: Some("/chat/completions".to_string()),
            api_key: None,
            auth_mode: "bearer".to_string(),
            auth_header_name: None,
            anthropic_version: None,
            beta_headers: None,
            custom_headers: None,
            default_model: Some("qwen3".to_string()),
            timeout_ms: 120_000,
            connect_timeout_ms: 15_000,
            max_retries: 2,
            model_refresh_mode: Some("registry".to_string()),
            models_path: Some("/models".to_string()),
            last_model_refresh_at: None,
        }
    }

    #[test]
    fn reject_custom_auth_without_header_name() {
        let mut cfg = base_custom_config();
        cfg.auth_mode = "custom".to_string();
        cfg.auth_header_name = None;
        let err = validate_provider_config(&mut cfg).expect_err("expected invalid custom auth");
        assert_eq!(err.code, "INVALID_AUTH_HEADER");
    }

    #[test]
    fn accept_custom_anthropic_default_endpoint() {
        let mut cfg = base_custom_config();
        cfg.protocol = "custom_anthropic_compatible".to_string();
        cfg.endpoint_path = None;
        validate_provider_config(&mut cfg).expect("custom anthropic config should be valid");
        assert_eq!(cfg.endpoint_path.as_deref(), Some("/messages"));
    }
}
