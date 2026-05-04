use crate::adapters::llm_types::{ProviderConfig, TaskRoute};
use crate::errors::AppErrorDto;
use crate::infra::{app_database, credential_manager};
use crate::services::task_routing;
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
            config.api_key = load_masked_api_key(&config.id)?;
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
            let trimmed = key.trim();
            if trimmed.is_empty() {
                credential_manager::delete_api_key(&config.id)?;
            } else {
                credential_manager::save_api_key(&config.id, trimmed)?;
            }
        }

        app_database::upsert_provider(&conn, &config, &now)?;

        let mut result = config;
        result.api_key = load_masked_api_key(&result.id)?;
        Ok(result)
    }


    /// Delete a provider and its API key.
    pub fn delete_provider(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        let conn = app_database::open_or_create()?;
        app_database::delete_provider(&conn, provider_id)?;
        credential_manager::delete_api_key(provider_id)?;
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

        if let Some(key) = credential_manager::load_api_key(provider_id)? {
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
            Some(json) => serde_json::from_str(&json).map_err(|e| {
                AppErrorDto::new("DESERIALIZE_FAILED", "Cannot parse editor settings", true)
                    .with_detail(e.to_string())
            }),
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

fn load_masked_api_key(provider_id: &str) -> Result<Option<String>, AppErrorDto> {
    credential_manager::load_api_key(provider_id).map(|value| value.map(|key| mask_api_key(&key)))
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
    if parsed.scheme() == "https" {
        // secure default
    } else if parsed.scheme() == "http" && is_loopback_host(parsed.host_str()) {
        // allow local development endpoints
    } else {
        return Err(AppErrorDto::new(
            "INVALID_BASE_URL_SCHEME",
            "Provider base URL must use https:// (http:// is allowed only for localhost/loopback)",
            true,
        ));
    }

    if config.timeout_ms == 0 {
        config.timeout_ms = 120_000;
    }
    if config.connect_timeout_ms == 0 {
        config.connect_timeout_ms = 15_000;
    }

    if config.protocol == "custom_anthropic_compatible"
        && config
            .endpoint_path
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        config.endpoint_path = Some("/messages".to_string());
    }
    if config.protocol == "custom_openai_compatible"
        && config
            .endpoint_path
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        config.endpoint_path = Some("/chat/completions".to_string());
    }

    if config.auth_mode.is_empty() {
        config.auth_mode = "bearer".to_string();
    }

    Ok(())
}

fn build_adapter(config: ProviderConfig) -> Box<dyn crate::adapters::LlmService> {
    crate::adapters::build_adapter(config)
}

fn is_loopback_host(host: Option<&str>) -> bool {
    matches!(
        host.unwrap_or("").to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}

// ── Task route business logic (moved from commands layer) ──

fn normalize_task_routes(routes: Vec<TaskRoute>) -> Vec<TaskRoute> {
    use std::collections::BTreeMap;
    let mut dedup: BTreeMap<String, (bool, TaskRoute)> = BTreeMap::new();
    for mut route in routes {
        let canonical = task_routing::canonical_task_type(&route.task_type).into_owned();
        let is_exact_canonical = route.task_type == canonical;
        route.task_type = canonical.clone();

        let should_replace = match dedup.get(&canonical) {
            None => true,
            Some((existing_exact, _)) => is_exact_canonical && !*existing_exact,
        };
        if should_replace {
            dedup.insert(canonical, (is_exact_canonical, route));
        }
    }
    dedup.into_values().map(|(_, route)| route).collect()
}

fn pick_primary_route_seed(
    routes: &[TaskRoute],
    providers: &[ProviderConfig],
) -> Option<(String, String)> {
    const PROVIDER_SEED_PRIORITY: &[&str] = &[
        "deepseek",
        "kimi",
        "zhipu",
        "minimax",
        "openai",
        "anthropic",
        "gemini",
        "custom",
    ];

    if let Some(existing) = routes
        .iter()
        .find(|route| !route.provider_id.trim().is_empty() && !route.model_id.trim().is_empty())
    {
        return Some((
            existing.provider_id.trim().to_string(),
            existing.model_id.trim().to_string(),
        ));
    }

    for provider_id in PROVIDER_SEED_PRIORITY {
        if let Some(provider) = providers
            .iter()
            .find(|provider| provider.id == *provider_id)
        {
            let model_id = provider.default_model.as_deref().unwrap_or("").trim();
            if !model_id.is_empty() {
                return Some((provider.id.clone(), model_id.to_string()));
            }
        }
    }

    providers.iter().find_map(|provider| {
        let model_id = provider.default_model.as_deref().unwrap_or("").trim();
        if provider.id.trim().is_empty() || model_id.is_empty() {
            None
        } else {
            Some((provider.id.clone(), model_id.to_string()))
        }
    })
}

/// Load task routes and fill in defaults if the table is empty.
pub fn ensure_default_task_routes(
    conn: &rusqlite::Connection,
    providers: &[ProviderConfig],
    routes: Vec<TaskRoute>,
) -> Result<Vec<TaskRoute>, AppErrorDto> {
    let normalized_routes = normalize_task_routes(routes);
    if !normalized_routes.is_empty() {
        return Ok(normalized_routes);
    }

    let Some((provider_id, model_id)) = pick_primary_route_seed(&normalized_routes, providers)
    else {
        return Ok(normalized_routes);
    };

    let now = crate::infra::time::now_iso();
    for task_type in &task_routing::task_route_types_with_custom() {
        let route = TaskRoute {
            id: uuid::Uuid::new_v4().to_string(),
            task_type: (*task_type).to_string(),
            provider_id: provider_id.clone(),
            model_id: model_id.clone(),
            fallback_provider_id: None,
            fallback_model_id: None,
            max_retries: 1,
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };
        app_database::upsert_task_route(conn, &route, &now)?;
    }

    let refreshed = app_database::load_task_routes(conn)?;
    Ok(normalize_task_routes(refreshed))
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
    fn accept_custom_anthropic_default_endpoint() {
        let mut cfg = base_custom_config();
        cfg.protocol = "custom_anthropic_compatible".to_string();
        cfg.endpoint_path = None;
        validate_provider_config(&mut cfg).expect("custom anthropic config should be valid");
        assert_eq!(cfg.endpoint_path.as_deref(), Some("/messages"));
    }

    #[test]
    fn reject_non_loopback_http_provider_base_url() {
        let mut cfg = base_custom_config();
        cfg.base_url = "http://api.example.com/v1".to_string();
        let err = validate_provider_config(&mut cfg).expect_err("public http should be rejected");
        assert_eq!(err.code, "INVALID_BASE_URL_SCHEME");
    }

    #[test]
    fn allow_loopback_http_provider_base_url() {
        let mut cfg = base_custom_config();
        cfg.base_url = "http://127.0.0.1:11434/v1".to_string();
        validate_provider_config(&mut cfg).expect("loopback http should remain allowed");
    }
}
