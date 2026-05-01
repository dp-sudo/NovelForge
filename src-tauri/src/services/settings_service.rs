use crate::adapters::anthropic::AnthropicAdapter;
use crate::adapters::gemini::GeminiAdapter;
use crate::adapters::llm_service::LlmService;
use crate::adapters::llm_types::ProviderConfig;
use crate::adapters::openai_compatible::OpenAiCompatibleAdapter;
use crate::errors::AppErrorDto;
use crate::infra::{app_database, credential_manager};
use rusqlite::Connection;
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

trait SecretStore {
    fn load_api_key(&self, provider_id: &str) -> Result<Option<String>, AppErrorDto>;
    fn save_api_key(&self, provider_id: &str, api_key: &str) -> Result<(), AppErrorDto>;
    fn delete_api_key(&self, provider_id: &str) -> Result<(), AppErrorDto>;
}

struct SystemSecretStore;

impl SecretStore for SystemSecretStore {
    fn load_api_key(&self, provider_id: &str) -> Result<Option<String>, AppErrorDto> {
        credential_manager::load_api_key(provider_id)
    }

    fn save_api_key(&self, provider_id: &str, api_key: &str) -> Result<(), AppErrorDto> {
        credential_manager::save_api_key(provider_id, api_key)
    }

    fn delete_api_key(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        credential_manager::delete_api_key(provider_id)
    }
}

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
        let mut conn = app_database::open_or_create()?;
        persist_provider_with_secret(&mut conn, &SystemSecretStore, config, api_key)
    }

    /// Load a single provider config (with masked API Key).
    pub fn load_provider(&self, provider_id: &str) -> Result<ProviderConfig, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        let mut config = app_database::load_provider(&conn, provider_id)?.ok_or_else(|| {
            AppErrorDto::new(
                "PROVIDER_NOT_FOUND",
                &format!("未找到供应商 '{}'", provider_id),
                true,
            )
        })?;

        config.api_key = load_masked_api_key(provider_id)?;
        Ok(config)
    }

    /// Delete a provider and its API key.
    pub fn delete_provider(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        let mut conn = app_database::open_or_create()?;
        delete_provider_with_secret(&mut conn, &SystemSecretStore, provider_id)
    }

    /// Test connection against the provider endpoint and return detailed status.
    pub async fn test_connection(&self, provider_id: &str) -> Result<String, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        let mut config = app_database::load_provider(&conn, provider_id)?.ok_or_else(|| {
            AppErrorDto::new(
                "PROVIDER_NOT_FOUND",
                &format!("未找到供应商 '{}'", provider_id),
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
                AppErrorDto::new("DESERIALIZE_FAILED", "无法解析编辑器设置", true)
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
                "无法序列化编辑器设置",
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
    load_masked_api_key_from_store(&SystemSecretStore, provider_id)
}

fn load_masked_api_key_from_store(
    secrets: &impl SecretStore,
    provider_id: &str,
) -> Result<Option<String>, AppErrorDto> {
    secrets
        .load_api_key(provider_id)
        .map(|value| value.map(|key| mask_api_key(&key)))
}

fn apply_secret_change(
    secrets: &impl SecretStore,
    provider_id: &str,
    api_key: Option<&str>,
) -> Result<(), AppErrorDto> {
    let Some(raw_key) = api_key else {
        return Ok(());
    };
    let trimmed = raw_key.trim();
    if trimmed.is_empty() {
        secrets.delete_api_key(provider_id)
    } else {
        secrets.save_api_key(provider_id, trimmed)
    }
}

fn restore_secret_state(
    secrets: &impl SecretStore,
    provider_id: &str,
    previous_api_key: Option<&str>,
) -> Result<(), AppErrorDto> {
    match previous_api_key {
        Some(value) => secrets.save_api_key(provider_id, value),
        None => secrets.delete_api_key(provider_id),
    }
}

fn persist_provider_with_secret(
    conn: &mut Connection,
    secrets: &impl SecretStore,
    config: ProviderConfig,
    api_key: Option<String>,
) -> Result<ProviderConfig, AppErrorDto> {
    let now = crate::infra::time::now_iso();
    let previous_api_key = secrets.load_api_key(&config.id)?;
    let tx = conn.transaction().map_err(|err| {
        AppErrorDto::new("DB_WRITE_FAILED", "无法保存供应商配置", true).with_detail(err.to_string())
    })?;

    app_database::upsert_provider(&tx, &config, &now)?;

    if let Err(err) = apply_secret_change(secrets, &config.id, api_key.as_deref()) {
        let _ = tx.rollback();
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        let rollback_error = restore_secret_state(secrets, &config.id, previous_api_key.as_deref())
            .err()
            .map(|item| format!(" secret rollback failed: {}", item.message))
            .unwrap_or_default();
        return Err(
            AppErrorDto::new("DB_WRITE_FAILED", "无法保存供应商配置", true)
                .with_detail(format!("{}{}", err, rollback_error)),
        );
    }

    let mut result = config;
    result.api_key = load_masked_api_key_from_store(secrets, &result.id)?;
    Ok(result)
}

fn delete_provider_with_secret(
    conn: &mut Connection,
    secrets: &impl SecretStore,
    provider_id: &str,
) -> Result<(), AppErrorDto> {
    let previous_api_key = secrets.load_api_key(provider_id)?;
    apply_secret_change(secrets, provider_id, Some(""))?;

    let tx = conn.transaction().map_err(|err| {
        AppErrorDto::new("DB_DELETE_FAILED", "无法删除供应商配置", true)
            .with_detail(err.to_string())
    })?;
    if let Err(err) = app_database::delete_provider(&tx, provider_id) {
        let _ = tx.rollback();
        restore_secret_state(secrets, provider_id, previous_api_key.as_deref())?;
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        restore_secret_state(secrets, provider_id, previous_api_key.as_deref())?;
        return Err(
            AppErrorDto::new("DB_DELETE_FAILED", "无法删除供应商配置", true)
                .with_detail(err.to_string()),
        );
    }

    Ok(())
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
            "供应商ID不能为空",
            true,
        ));
    }
    if config.display_name.is_empty() {
        return Err(AppErrorDto::new(
            "INVALID_PROVIDER_NAME",
            "供应商显示名称不能为空",
            true,
        ));
    }
    if config.base_url.is_empty() {
        return Err(AppErrorDto::new(
            "INVALID_BASE_URL",
            "供应商服务地址不能为空",
            true,
        ));
    }
    let parsed = reqwest::Url::parse(&config.base_url).map_err(|err| {
        AppErrorDto::new("INVALID_BASE_URL", "供应商服务地址无效", true)
            .with_detail(err.to_string())
    })?;
    if parsed.scheme() == "https" {
        // secure default
    } else if parsed.scheme() == "http" && is_loopback_host(parsed.host_str()) {
        // allow local development endpoints
    } else {
        return Err(AppErrorDto::new(
            "INVALID_BASE_URL_SCHEME",
            "供应商服务地址必须使用 https://（仅 localhost/loopback 允许 http://）",
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

fn is_loopback_host(host: Option<&str>) -> bool {
    matches!(
        host.unwrap_or("").to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FakeSecretStore {
        keys: Arc<Mutex<HashMap<String, String>>>,
    }

    impl FakeSecretStore {
        fn with_key(provider_id: &str, api_key: &str) -> Self {
            let store = Self::default();
            store
                .keys
                .lock()
                .expect("lock secret store")
                .insert(provider_id.to_string(), api_key.to_string());
            store
        }
    }

    impl SecretStore for FakeSecretStore {
        fn load_api_key(&self, provider_id: &str) -> Result<Option<String>, AppErrorDto> {
            Ok(self
                .keys
                .lock()
                .expect("lock secret store")
                .get(provider_id)
                .cloned())
        }

        fn save_api_key(&self, provider_id: &str, api_key: &str) -> Result<(), AppErrorDto> {
            self.keys
                .lock()
                .expect("lock secret store")
                .insert(provider_id.to_string(), api_key.to_string());
            Ok(())
        }

        fn delete_api_key(&self, provider_id: &str) -> Result<(), AppErrorDto> {
            self.keys
                .lock()
                .expect("lock secret store")
                .remove(provider_id);
            Ok(())
        }
    }

    fn setup_app_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory app db");
        crate::infra::migrator::run_app_pending(&conn).expect("run app migrations");
        conn
    }

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

    #[test]
    fn persist_provider_with_secret_skips_secret_write_when_db_write_fails() {
        let mut conn = setup_app_conn();
        conn.execute_batch(
            r#"
            CREATE TRIGGER fail_provider_insert
            BEFORE INSERT ON llm_providers
            BEGIN
              SELECT RAISE(FAIL, 'blocked');
            END;
            "#,
        )
        .expect("create trigger");
        let secrets = FakeSecretStore::default();
        let config = base_custom_config();

        let err = persist_provider_with_secret(
            &mut conn,
            &secrets,
            config.clone(),
            Some("sk-test-value".to_string()),
        )
        .expect_err("db write failure should bubble");
        assert_eq!(err.code, "DB_WRITE_FAILED");
        assert!(
            app_database::load_provider(&conn, &config.id)
                .expect("load provider")
                .is_none()
        );
        assert_eq!(
            secrets.load_api_key(&config.id).expect("load secret"),
            None
        );
    }

    #[test]
    fn delete_provider_with_secret_restores_secret_when_db_delete_fails() {
        let mut conn = setup_app_conn();
        let config = base_custom_config();
        let now = crate::infra::time::now_iso();
        app_database::upsert_provider(&conn, &config, &now).expect("seed provider");
        conn.execute_batch(
            r#"
            CREATE TRIGGER fail_provider_delete
            BEFORE DELETE ON llm_providers
            BEGIN
              SELECT RAISE(FAIL, 'blocked');
            END;
            "#,
        )
        .expect("create trigger");
        let secrets = FakeSecretStore::with_key(&config.id, "sk-existing");

        let err = delete_provider_with_secret(&mut conn, &secrets, &config.id)
            .expect_err("db delete failure should bubble");
        assert_eq!(err.code, "DB_DELETE_FAILED");
        assert!(
            app_database::load_provider(&conn, &config.id)
                .expect("load provider")
                .is_some()
        );
        assert_eq!(
            secrets.load_api_key(&config.id).expect("load secret"),
            Some("sk-existing".to_string())
        );
    }
}
