use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use uuid::Uuid;

use crate::adapters::llm_types::*;
use crate::errors::AppErrorDto;
use crate::infra::{app_database, credential_manager};

pub struct ModelRegistryService;

const MAX_REGISTRY_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;

impl Default for ModelRegistryService {
    fn default() -> Self {
        Self
    }
}

impl ModelRegistryService {
    /// Refresh models for a provider: fetch live models + detect capabilities.
    pub async fn refresh_provider_models(
        &self,
        provider_id: &str,
    ) -> Result<RefreshResult, AppErrorDto> {
        let now = crate::infra::time::now_iso();
        let conn = app_database::open_or_create()?;

        let mut config = app_database::load_provider(&conn, provider_id)?.ok_or_else(|| {
            AppErrorDto::new(
                "PROVIDER_NOT_FOUND",
                &format!("未找到供应商 '{}'", provider_id),
                true,
            )
        })?;

        if let Ok(Some(key)) = credential_manager::load_api_key(provider_id) {
            config.api_key = Some(key);
        }

        let adapter = build_adapter(&config)?;

        let live_model_names = adapter.fetch_models().await.map_err(|e| {
            AppErrorDto::new("MODEL_FETCH_FAILED", "无法从供应商获取模型列表", true)
                .with_detail(format!("{:?}", e))
        })?;

        let capabilities = adapter
            .detect_capabilities()
            .await
            .unwrap_or(CapabilityReport {
                provider_id: provider_id.to_string(),
                text_response: true,
                streaming: false,
                json_object: false,
                json_schema: false,
                tools: false,
                thinking: false,
                error: Some("能力检测失败".to_string()),
            });

        let existing = app_database::load_models(&conn, provider_id)?;
        let existing_names: std::collections::HashSet<String> =
            existing.iter().map(|m| m.model_name.clone()).collect();
        let live_names: std::collections::HashSet<String> =
            live_model_names.iter().cloned().collect();

        let mut added = 0i64;
        let mut updated = 0i64;
        let mut removed = 0i64;

        for model_name in &live_model_names {
            let record = ModelRecord {
                id: Uuid::new_v4().to_string(),
                provider_id: provider_id.to_string(),
                model_name: model_name.clone(),
                display_name: None,
                context_window_tokens: None,
                max_output_tokens: None,
                supports_streaming: capabilities.streaming,
                supports_tools: capabilities.tools,
                supports_json_object: capabilities.json_object,
                supports_json_schema: capabilities.json_schema,
                supports_thinking: capabilities.thinking,
                supports_reasoning_effort: false,
                supports_prompt_cache: false,
                status: "available".to_string(),
                source: Some("provider_live".to_string()),
                user_overridden: false,
                last_seen_at: Some(now.clone()),
                registry_version: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            let is_new = app_database::upsert_model(&conn, &record)?;
            if is_new {
                added += 1;
            } else {
                updated += 1;
            }
        }

        for existing_name in &existing_names {
            if !live_names.contains(existing_name) {
                let _ = conn.execute(
                    "UPDATE llm_models SET status='deprecated', updated_at=?1 WHERE provider_id=?2 AND model_name=?3",
                    rusqlite::params![now, provider_id, existing_name],
                );
                removed += 1;
            }
        }

        let log = RefreshLog {
            id: Uuid::new_v4().to_string(),
            provider_id: provider_id.to_string(),
            refresh_type: "manual".to_string(),
            status: "completed".to_string(),
            models_added: added,
            models_updated: updated,
            models_removed: removed,
            error_message: None,
            created_at: now.clone(),
        };
        app_database::insert_refresh_log(&conn, &log)?;

        let _ = conn.execute(
            "UPDATE llm_providers SET last_model_refresh_at=?1, updated_at=?1 WHERE id=?2",
            rusqlite::params![now, provider_id],
        );

        Ok(RefreshResult {
            added,
            updated,
            removed,
            capabilities,
        })
    }

    /// Get models for a provider.
    pub fn get_models(&self, provider_id: &str) -> Result<Vec<ModelRecord>, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        app_database::load_models(&conn, provider_id)
    }

    /// Check remote model registry for updates.
    pub async fn check_remote_registry(
        &self,
        url: &str,
    ) -> Result<RegistryCheckResult, AppErrorDto> {
        let now = crate::infra::time::now_iso();
        let registry = load_verified_registry(url).await?;

        let registry_version = registry.registry_version.clone();
        let registry_updated_at = registry.updated_at.clone();

        // Load existing registry state
        let conn = app_database::open_or_create()?;
        let existing_version: Option<String> = conn
            .query_row(
                "SELECT registry_version FROM llm_model_registry_state WHERE id='default'",
                [],
                |row| row.get(0),
            )
            .ok();

        let has_update = match (&existing_version, &registry_version) {
            (Some(old), new) if old != new => true,
            (None, _) => true,
            _ => false,
        };

        // Store the registry state
        let _ = conn.execute(
            "INSERT INTO llm_model_registry_state
             (id, registry_version, registry_updated_at, last_checked_at, source, signature_valid, error_code, error_message)
             VALUES ('default', ?1, ?2, ?3, 'remote', 1, NULL, NULL)
             ON CONFLICT(id) DO UPDATE SET
             registry_version=excluded.registry_version,
             registry_updated_at=excluded.registry_updated_at,
             last_checked_at=excluded.last_checked_at,
             source=excluded.source,
             signature_valid=excluded.signature_valid,
             error_code=NULL,
             error_message=NULL",
            rusqlite::params![registry_version, registry_updated_at, now],
        );

        Ok(RegistryCheckResult {
            current_version: existing_version.unwrap_or_default(),
            remote_version: registry_version,
            has_update,
            checked_at: now,
        })
    }

    /// Apply a remote registry update to the llm_models table.
    pub async fn apply_registry_update(
        &self,
        url: &str,
    ) -> Result<RegistryApplyResult, AppErrorDto> {
        let now = crate::infra::time::now_iso();
        let registry = load_verified_registry(url).await?;

        let mut conn = app_database::open_or_create()?;
        let tx = conn.transaction().map_err(|e| {
            AppErrorDto::new("DB_TRANSACTION_FAILED", "无法启动注册表更新事务", false)
                .with_detail(e.to_string())
        })?;

        let mut total_added = 0i64;
        let mut total_updated = 0i64;

        for provider_entry in &registry.providers {
            for model_entry in &provider_entry.models {
                if model_entry.model_name.trim().is_empty() {
                    continue;
                }

                let record = ModelRecord {
                    id: Uuid::new_v4().to_string(),
                    provider_id: provider_entry.vendor.clone(),
                    model_name: model_entry.model_name.clone(),
                    display_name: model_entry.display_name.clone(),
                    context_window_tokens: model_entry.context_window_tokens,
                    max_output_tokens: model_entry.max_output_tokens,
                    supports_streaming: model_entry.supports_streaming,
                    supports_tools: model_entry.supports_tools,
                    supports_json_object: model_entry.supports_json_object,
                    supports_json_schema: model_entry.supports_json_schema,
                    supports_thinking: model_entry.supports_thinking,
                    supports_reasoning_effort: model_entry.supports_reasoning_effort,
                    supports_prompt_cache: model_entry.supports_prompt_cache,
                    status: model_entry
                        .status
                        .clone()
                        .unwrap_or_else(|| "available".to_string()),
                    source: Some("registry".to_string()),
                    user_overridden: false,
                    last_seen_at: Some(now.clone()),
                    registry_version: Some(registry.registry_version.clone()),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };

                let is_new = app_database::upsert_model(&tx, &record)?;
                if is_new {
                    total_added += 1;
                } else {
                    total_updated += 1;
                }
            }
        }

        let _ = tx.execute(
            "INSERT INTO llm_model_registry_state
             (id, registry_version, registry_updated_at, last_checked_at, last_applied_at, source, signature_valid, error_code, error_message)
             VALUES ('default', ?1, ?2, ?3, ?3, 'remote', 1, NULL, NULL)
             ON CONFLICT(id) DO UPDATE SET
             registry_version=excluded.registry_version,
             registry_updated_at=excluded.registry_updated_at,
             last_checked_at=excluded.last_checked_at,
             last_applied_at=excluded.last_applied_at,
             source=excluded.source,
             signature_valid=excluded.signature_valid,
             error_code=NULL,
             error_message=NULL",
            rusqlite::params![registry.registry_version, registry.updated_at, now],
        );

        tx.commit().map_err(|e| {
            AppErrorDto::new("DB_TRANSACTION_FAILED", "无法提交注册表更新事务", false)
                .with_detail(e.to_string())
        })?;

        Ok(RegistryApplyResult {
            added: total_added,
            updated: total_updated,
            version: registry.registry_version,
            applied_at: now,
        })
    }

    /// Get the refresh log for a provider.
    pub fn get_refresh_logs(&self, provider_id: &str) -> Result<Vec<RefreshLog>, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        app_database::load_refresh_logs(&conn, provider_id, 20)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    pub added: i64,
    pub updated: i64,
    pub removed: i64,
    pub capabilities: CapabilityReport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCheckResult {
    pub current_version: String,
    pub remote_version: String,
    pub has_update: bool,
    pub checked_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryApplyResult {
    pub added: i64,
    pub updated: i64,
    pub version: String,
    pub applied_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryDocument {
    schema_version: String,
    registry_version: String,
    updated_at: Option<String>,
    providers: Vec<RegistryProviderEntry>,
    signing: RegistrySigning,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryProviderEntry {
    vendor: String,
    models: Vec<RegistryModelEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryModelEntry {
    model_name: String,
    display_name: Option<String>,
    context_window_tokens: Option<i64>,
    max_output_tokens: Option<i64>,
    #[serde(default)]
    supports_streaming: bool,
    #[serde(default)]
    supports_tools: bool,
    #[serde(default)]
    supports_json_object: bool,
    #[serde(default)]
    supports_json_schema: bool,
    #[serde(default)]
    supports_thinking: bool,
    #[serde(default)]
    supports_reasoning_effort: bool,
    #[serde(default)]
    supports_prompt_cache: bool,
    status: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistrySigning {
    algorithm: String,
    signature: String,
    #[serde(default)]
    public_key: Option<String>,
}

fn validate_registry_url(url: &str) -> Result<reqwest::Url, AppErrorDto> {
    let parsed = reqwest::Url::parse(url).map_err(|e| {
        AppErrorDto::new("INVALID_REGISTRY_URL", "注册表地址无效", true).with_detail(e.to_string())
    })?;

    let scheme = parsed.scheme();
    if scheme == "https" {
        return Ok(parsed);
    }

    if scheme == "http" && is_loopback_host(parsed.host_str()) {
        return Ok(parsed);
    }

    Err(AppErrorDto::new(
        "INSECURE_REGISTRY_URL",
        "注册表地址必须使用 HTTPS（localhost/loopback 可使用 HTTP）",
        true,
    ))
}

async fn load_verified_registry(url: &str) -> Result<RegistryDocument, AppErrorDto> {
    let parsed_url = validate_registry_url(url)?;
    let payload = fetch_registry_payload(parsed_url.as_str()).await?;
    let (registry, registry_json) = parse_registry_payload(&payload)?;
    validate_registry_document(&registry)?;
    verify_registry_signature(&registry, &registry_json, parsed_url.host_str())?;
    Ok(registry)
}

async fn fetch_registry_payload(url: &str) -> Result<String, AppErrorDto> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| {
            AppErrorDto::new("HTTP_CLIENT_FAILED", "无法创建 HTTP 客户端", false)
                .with_detail(e.to_string())
        })?;

    let response = client.get(url).send().await.map_err(|e| {
        AppErrorDto::new("REGISTRY_FETCH_FAILED", "无法获取远程注册表", true)
            .with_detail(e.to_string())
    })?;

    if !response.status().is_success() {
        return Err(AppErrorDto::new(
            "REGISTRY_HTTP_ERROR",
            &format!("注册表返回 HTTP {}", response.status().as_u16()),
            true,
        ));
    }

    if let Some(content_length) = response.content_length() {
        if content_length > MAX_REGISTRY_PAYLOAD_BYTES as u64 {
            return Err(AppErrorDto::new(
                "REGISTRY_PAYLOAD_TOO_LARGE",
                "注册表内容过大，请检查源地址",
                true,
            ));
        }
    }

    let bytes = response.bytes().await.map_err(|e| {
        AppErrorDto::new("REGISTRY_FETCH_FAILED", "无法读取注册表内容", true)
            .with_detail(e.to_string())
    })?;
    decode_registry_payload_bytes(bytes.as_ref())
}

fn decode_registry_payload_bytes(bytes: &[u8]) -> Result<String, AppErrorDto> {
    if bytes.len() > MAX_REGISTRY_PAYLOAD_BYTES {
        return Err(AppErrorDto::new(
            "REGISTRY_PAYLOAD_TOO_LARGE",
            "注册表内容过大，请检查源地址",
            true,
        ));
    }

    String::from_utf8(bytes.to_vec()).map_err(|e| {
        AppErrorDto::new("REGISTRY_PARSE_FAILED", "注册表内容不是有效 UTF-8", false)
            .with_detail(e.to_string())
    })
}

fn parse_registry_payload(
    payload: &str,
) -> Result<(RegistryDocument, serde_json::Value), AppErrorDto> {
    let json = serde_json::from_str::<serde_json::Value>(payload).map_err(|e| {
        AppErrorDto::new("REGISTRY_PARSE_FAILED", "注册表 JSON 结构无效", false)
            .with_detail(e.to_string())
    })?;
    let registry = serde_json::from_value::<RegistryDocument>(json.clone()).map_err(|e| {
        AppErrorDto::new("REGISTRY_PARSE_FAILED", "注册表 JSON 结构无效", false)
            .with_detail(e.to_string())
    })?;
    Ok((registry, json))
}

fn validate_registry_document(registry: &RegistryDocument) -> Result<(), AppErrorDto> {
    if registry.schema_version.trim().is_empty() {
        return Err(AppErrorDto::new(
            "REGISTRY_SCHEMA_INVALID",
            "schemaVersion 不能为空",
            false,
        ));
    }
    if registry.registry_version.trim().is_empty() {
        return Err(AppErrorDto::new(
            "REGISTRY_SCHEMA_INVALID",
            "registryVersion 不能为空",
            false,
        ));
    }
    if registry.providers.is_empty() {
        return Err(AppErrorDto::new(
            "REGISTRY_SCHEMA_INVALID",
            "providers 不能为空",
            false,
        ));
    }

    for provider in &registry.providers {
        if provider.vendor.trim().is_empty() {
            return Err(AppErrorDto::new(
                "REGISTRY_SCHEMA_INVALID",
                "provider.vendor 不能为空",
                false,
            ));
        }
        for model in &provider.models {
            if model.model_name.trim().is_empty() {
                return Err(AppErrorDto::new(
                    "REGISTRY_SCHEMA_INVALID",
                    "model.modelName 不能为空",
                    false,
                ));
            }
        }
    }
    Ok(())
}

fn verify_registry_signature(
    registry: &RegistryDocument,
    registry_json: &serde_json::Value,
    host: Option<&str>,
) -> Result<(), AppErrorDto> {
    let algorithm = registry.signing.algorithm.trim().to_lowercase();
    let signature = registry.signing.signature.trim();
    if signature.is_empty() {
        return Err(AppErrorDto::new(
            "REGISTRY_SIGNATURE_INVALID",
            "注册表签名缺失",
            false,
        ));
    }

    match algorithm.as_str() {
        "ed25519" => {
            let signature_bytes = base64::engine::general_purpose::STANDARD
                .decode(signature)
                .map_err(|e| {
                    AppErrorDto::new(
                        "REGISTRY_SIGNATURE_INVALID",
                        "注册表签名不是有效的 base64",
                        false,
                    )
                    .with_detail(e.to_string())
                })?;
            if signature_bytes.len() != 64 {
                return Err(AppErrorDto::new(
                    "REGISTRY_SIGNATURE_INVALID",
                    "注册表签名长度不符合 ed25519 要求",
                    false,
                ));
            }
            let signature = Signature::from_slice(&signature_bytes).map_err(|e| {
                AppErrorDto::new("REGISTRY_SIGNATURE_INVALID", "注册表签名无法解析", false)
                    .with_detail(e.to_string())
            })?;

            let public_key_b64 = resolve_registry_public_key(registry, host).ok_or_else(|| {
                AppErrorDto::new(
                    "REGISTRY_SIGNATURE_INVALID",
                    "缺少受信任的注册表公钥",
                    false,
                )
            })?;
            let public_key_bytes = base64::engine::general_purpose::STANDARD
                .decode(public_key_b64.trim())
                .map_err(|e| {
                    AppErrorDto::new(
                        "REGISTRY_SIGNATURE_INVALID",
                        "注册表公钥不是有效的 base64",
                        false,
                    )
                    .with_detail(e.to_string())
                })?;
            if public_key_bytes.len() != 32 {
                return Err(AppErrorDto::new(
                    "REGISTRY_SIGNATURE_INVALID",
                    "注册表公钥长度不符合 ed25519 要求",
                    false,
                ));
            }

            let key_array: [u8; 32] = public_key_bytes.as_slice().try_into().map_err(|_| {
                AppErrorDto::new(
                    "REGISTRY_SIGNATURE_INVALID",
                    "注册表公钥长度不符合 ed25519 要求",
                    false,
                )
            })?;
            let verifying_key = VerifyingKey::from_bytes(&key_array).map_err(|e| {
                AppErrorDto::new("REGISTRY_SIGNATURE_INVALID", "注册表公钥无法解析", false)
                    .with_detail(e.to_string())
            })?;

            let signed_payload = canonical_unsigned_registry_payload(registry_json)?;
            verifying_key
                .verify(signed_payload.as_bytes(), &signature)
                .map_err(|e| {
                    AppErrorDto::new("REGISTRY_SIGNATURE_INVALID", "注册表签名校验失败", false)
                        .with_detail(e.to_string())
                })
        }
        "none" => {
            if is_loopback_host(host) {
                Ok(())
            } else {
                Err(AppErrorDto::new(
                    "REGISTRY_SIGNATURE_INVALID",
                    "仅 localhost/loopback 允许无签名注册表",
                    false,
                ))
            }
        }
        _ => Err(AppErrorDto::new(
            "REGISTRY_SIGNATURE_INVALID",
            "不支持的注册表签名算法",
            false,
        )),
    }
}

fn canonical_unsigned_registry_payload(
    registry_json: &serde_json::Value,
) -> Result<String, AppErrorDto> {
    let mut unsigned = registry_json.clone();
    let signing = unsigned
        .as_object_mut()
        .and_then(|root| root.get_mut("signing"))
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| {
            AppErrorDto::new("REGISTRY_SIGNATURE_INVALID", "注册表签名载荷缺失", false)
        })?;
    signing.insert(
        "signature".to_string(),
        serde_json::Value::String(String::new()),
    );

    unsigned.sort_all_objects();
    serde_json::to_string(&unsigned).map_err(|e| {
        AppErrorDto::new("REGISTRY_SIGNATURE_INVALID", "注册表载荷序列化失败", false)
            .with_detail(e.to_string())
    })
}

fn resolve_registry_public_key(registry: &RegistryDocument, host: Option<&str>) -> Option<String> {
    if let Some(host_str) = host {
        let env_name = format!(
            "NOVELFORGE_REGISTRY_ED25519_PUBLIC_KEY_{}",
            host_str
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() {
                        ch.to_ascii_uppercase()
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
        );
        if let Ok(value) = std::env::var(&env_name) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }

    if let Ok(value) = std::env::var("NOVELFORGE_REGISTRY_ED25519_PUBLIC_KEY") {
        if !value.trim().is_empty() {
            return Some(value);
        }
    }

    if is_loopback_host(host) {
        if let Some(value) = registry.signing.public_key.clone() {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn is_loopback_host(host: Option<&str>) -> bool {
    matches!(
        host.unwrap_or("").to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}

fn build_adapter(
    config: &ProviderConfig,
) -> Result<Box<dyn crate::adapters::LlmService>, AppErrorDto> {
    let is_anthropic_protocol = matches!(
        config.protocol.as_str(),
        "anthropic_messages" | "custom_anthropic_compatible"
    );
    let is_gemini_protocol = matches!(config.protocol.as_str(), "gemini_generate_content");

    match config.vendor.as_str() {
        "anthropic" | "minimax" => Ok(Box::new(crate::adapters::anthropic::AnthropicAdapter::new(
            config.clone(),
        ))),
        "gemini" => Ok(Box::new(crate::adapters::gemini::GeminiAdapter::new(
            config.clone(),
        ))),
        _ if is_anthropic_protocol => Ok(Box::new(
            crate::adapters::anthropic::AnthropicAdapter::new(config.clone()),
        )),
        _ if is_gemini_protocol => Ok(Box::new(crate::adapters::gemini::GeminiAdapter::new(
            config.clone(),
        ))),
        _ => Ok(Box::new(
            crate::adapters::openai_compatible::OpenAiCompatibleAdapter::new(config.clone()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn sample_registry(
        algorithm: &str,
        signature: &str,
        public_key: Option<String>,
    ) -> RegistryDocument {
        RegistryDocument {
            schema_version: "1.0.0".to_string(),
            registry_version: "2026.04.27.001".to_string(),
            updated_at: Some("2026-04-27T12:00:00+08:00".to_string()),
            providers: vec![RegistryProviderEntry {
                vendor: "deepseek".to_string(),
                models: vec![RegistryModelEntry {
                    model_name: "deepseek-v4-flash".to_string(),
                    display_name: None,
                    context_window_tokens: None,
                    max_output_tokens: None,
                    supports_streaming: true,
                    supports_tools: true,
                    supports_json_object: true,
                    supports_json_schema: false,
                    supports_thinking: true,
                    supports_reasoning_effort: true,
                    supports_prompt_cache: false,
                    status: Some("available".to_string()),
                }],
            }],
            signing: RegistrySigning {
                algorithm: algorithm.to_string(),
                signature: signature.to_string(),
                public_key,
            },
        }
    }

    fn signed_registry_for_tests() -> (RegistryDocument, serde_json::Value) {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().as_bytes());
        let mut registry = sample_registry("ed25519", "", Some(public_key));
        let base_json = serde_json::to_value(&registry).expect("serialize registry");
        let canonical = canonical_unsigned_registry_payload(&base_json).expect("canonical payload");
        let signature = signing_key.sign(canonical.as_bytes());
        registry.signing.signature =
            base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        let signed_json = serde_json::to_value(&registry).expect("serialize signed registry");
        (registry, signed_json)
    }

    #[test]
    fn reject_non_https_registry_url() {
        let err = validate_registry_url("http://example.com/registry.json")
            .expect_err("public http registry should be rejected");
        assert_eq!(err.code, "INSECURE_REGISTRY_URL");
    }

    #[test]
    fn allow_loopback_http_registry_url() {
        let parsed = validate_registry_url("http://localhost:8080/registry.json")
            .expect("loopback http should be allowed");
        assert_eq!(parsed.scheme(), "http");
    }

    #[test]
    fn verify_ed25519_signature_checks_cryptographic_validity() {
        let (registry, registry_json) = signed_registry_for_tests();
        verify_registry_signature(&registry, &registry_json, Some("localhost"))
            .expect("valid signature should pass");

        let mut tampered_json = registry_json.clone();
        tampered_json["registryVersion"] = serde_json::Value::String("2026.04.27.002".to_string());
        let err = verify_registry_signature(&registry, &tampered_json, Some("localhost"))
            .expect_err("tampered payload should fail");
        assert_eq!(err.code, "REGISTRY_SIGNATURE_INVALID");
    }

    #[test]
    fn verify_ed25519_signature_rejects_wrong_length() {
        let bad_sig = base64::engine::general_purpose::STANDARD.encode([0u8; 16]);
        let registry = sample_registry("ed25519", &bad_sig, None);
        let registry_json = serde_json::to_value(&registry).expect("serialize registry");
        let err = verify_registry_signature(&registry, &registry_json, Some("localhost"))
            .expect_err("short ed25519 signature should fail");
        assert_eq!(err.code, "REGISTRY_SIGNATURE_INVALID");
    }

    #[test]
    fn verify_ed25519_signature_requires_trusted_public_key_on_non_loopback() {
        let (registry, registry_json) = signed_registry_for_tests();
        std::env::remove_var("NOVELFORGE_REGISTRY_ED25519_PUBLIC_KEY");
        let err =
            verify_registry_signature(&registry, &registry_json, Some("updates.novelforge.app"))
                .expect_err("public host should require trusted env key");
        assert_eq!(err.code, "REGISTRY_SIGNATURE_INVALID");
    }

    #[test]
    fn reject_registry_payload_when_too_large() {
        let bytes = vec![b'a'; MAX_REGISTRY_PAYLOAD_BYTES + 1];
        let err = decode_registry_payload_bytes(&bytes).expect_err("oversized payload should fail");
        assert_eq!(err.code, "REGISTRY_PAYLOAD_TOO_LARGE");
    }

    #[test]
    fn reject_registry_payload_when_not_utf8() {
        let err =
            decode_registry_payload_bytes(&[0xFF, 0xFE]).expect_err("invalid utf8 should fail");
        assert_eq!(err.code, "REGISTRY_PARSE_FAILED");
    }
}
