use std::collections::HashMap;
use std::sync::{Arc, RwLock as StdRwLock};

use tokio::sync::{mpsc, RwLock};

use crate::adapters::{
    anthropic::AnthropicAdapter, gemini::GeminiAdapter, llm_service::LlmService, llm_types::*,
    openai_compatible::OpenAiCompatibleAdapter,
};
use crate::errors::AppErrorDto;
use crate::infra::{app_database, credential_manager};
use crate::services::skill_registry::SkillRegistry;
use crate::services::task_routing;

const PIPELINE_STREAM_ERROR_PREFIX: &str = "__NF_PIPELINE_ERROR__:";

#[derive(Clone)]
pub struct AiService {
    adapters: Arc<RwLock<HashMap<String, Box<dyn LlmService>>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRouteAttempt {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRouteResolution {
    pub canonical_task_type: String,
    pub provider_id: String,
    pub model_id: String,
    pub attempts: Vec<TaskRouteAttempt>,
}

impl Default for AiService {
    fn default() -> Self {
        Self {
            adapters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AiService {
    fn is_usable_route(route: &TaskRoute) -> bool {
        !route.provider_id.trim().is_empty() && !route.model_id.trim().is_empty()
    }

    fn pick_task_route<'a>(routes: &'a [TaskRoute], task_type: &str) -> Option<&'a TaskRoute> {
        let canonical_task = task_routing::canonical_task_type(task_type);
        if let Some(route) = routes
            .iter()
            .find(|r| r.task_type == canonical_task.as_ref() && Self::is_usable_route(r))
        {
            return Some(route);
        }

        routes.iter().find(|r| {
            Self::is_usable_route(r)
                && task_routing::canonical_task_type(&r.task_type).as_ref()
                    == canonical_task.as_ref()
        })
    }

    fn build_attempt_chain(
        provider_id: &str,
        model: &str,
        route: Option<&TaskRoute>,
    ) -> Vec<(String, String)> {
        let max_attempts = route
            .map(|r| r.max_retries.clamp(1, 8) as usize)
            .unwrap_or(1);
        let fallback = route.and_then(|r| {
            r.fallback_provider_id.as_ref().map(|fp| {
                let fallback_model = r
                    .fallback_model_id
                    .as_ref()
                    .map(|m| m.trim().to_string())
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| "default".to_string());
                (fp.clone(), fallback_model)
            })
        });

        let mut attempts = Vec::with_capacity(max_attempts);
        for idx in 0..max_attempts {
            if idx == 0 {
                attempts.push((provider_id.to_string(), model.to_string()));
            } else if let Some((ref fp, ref fm)) = fallback {
                attempts.push((fp.clone(), fm.clone()));
            } else {
                attempts.push((provider_id.to_string(), model.to_string()));
            }
        }
        attempts
    }

    /// Register a provider adapter at runtime.
    pub async fn register_provider(&self, config: ProviderConfig) {
        let id = config.id.clone();
        let is_anthropic_protocol = matches!(
            config.protocol.as_str(),
            "anthropic_messages" | "custom_anthropic_compatible"
        );
        let is_gemini_protocol = matches!(config.protocol.as_str(), "gemini_generate_content");

        let adapter: Box<dyn LlmService> = match config.vendor.as_str() {
            "anthropic" | "minimax" => Box::new(AnthropicAdapter::new(config)),
            "gemini" => Box::new(GeminiAdapter::new(config)),
            _ if is_anthropic_protocol => Box::new(AnthropicAdapter::new(config)),
            _ if is_gemini_protocol => Box::new(GeminiAdapter::new(config)),
            _ => Box::new(OpenAiCompatibleAdapter::new(config)),
        };
        self.adapters.write().await.insert(id, adapter);
    }

    /// Reload one provider adapter from app DB + credential store.
    pub async fn reload_provider(&self, provider_id: &str) -> Result<(), AppErrorDto> {
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
        self.register_provider(config).await;
        Ok(())
    }

    /// Remove one provider adapter from runtime cache.
    pub async fn unregister_provider(&self, provider_id: &str) {
        self.adapters.write().await.remove(provider_id);
    }

    async fn ensure_provider_registered(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        if self.adapters.read().await.contains_key(provider_id) {
            return Ok(());
        }
        self.reload_provider(provider_id).await
    }

    fn resolve_request_model(
        provider_id: &str,
        requested_model: &str,
    ) -> Result<String, AppErrorDto> {
        if !requested_model.trim().is_empty() && requested_model != "default" {
            return Ok(requested_model.to_string());
        }

        let conn = app_database::open_or_create()?;
        let provider = app_database::load_provider(&conn, provider_id)?.ok_or_else(|| {
            AppErrorDto::new(
                "PROVIDER_NOT_FOUND",
                &format!("Provider '{}' not found", provider_id),
                true,
            )
        })?;

        provider.default_model.ok_or_else(|| {
            AppErrorDto::new(
                "MODEL_NOT_CONFIGURED",
                &format!("Provider '{}' has no default model configured", provider_id),
                true,
            )
        })
    }

    /// Resolve task_type to a provider_id + model_id from llm_task_routes.
    fn resolve_route(task_type: &str) -> Result<(String, String, Option<TaskRoute>), AppErrorDto> {
        let conn = app_database::open_or_create()?;
        let routes = app_database::load_task_routes(&conn)?;
        let canonical_task = task_routing::canonical_task_type(task_type).into_owned();

        if let Some(r) = Self::pick_task_route(&routes, task_type) {
            return Ok((r.provider_id.clone(), r.model_id.clone(), Some(r.clone())));
        }

        // Unknown skill/task types can explicitly fallback to a dedicated "custom" route.
        if !task_routing::is_core_task_type(&canonical_task) {
            if let Some(custom_route) = Self::pick_task_route(&routes, "custom") {
                let mut inferred = custom_route.clone();
                inferred.task_type = canonical_task.clone();
                return Ok((
                    custom_route.provider_id.clone(),
                    custom_route.model_id.clone(),
                    Some(inferred),
                ));
            }
        }

        // Compatibility for old behavior: `custom` can still fallback to first route.
        if canonical_task == "custom" {
            if let Some(first) = routes.iter().find(|r| Self::is_usable_route(r)) {
                return Ok((
                    first.provider_id.clone(),
                    first.model_id.clone(),
                    Some(first.clone()),
                ));
            }
        }

        Err(AppErrorDto::new(
            "TASK_ROUTE_NOT_FOUND",
            &format!(
                "No route configured for task type '{}'. 请在「设置 > 任务路由」配置该任务，或配置 'custom' 作为兜底路由。",
                canonical_task
            ),
            true,
        ))
    }

    fn resolve_request_target(
        req: &UnifiedGenerateRequest,
    ) -> Result<(String, String, Option<TaskRoute>), AppErrorDto> {
        let provider_hint = req.provider_id.as_deref().map(str::trim).unwrap_or("");
        let should_route_by_task = provider_hint.is_empty() || provider_hint == "default";

        if should_route_by_task {
            if let Some(ref task_type) = req.task_type {
                return Self::resolve_route(task_type);
            }
            return Err(AppErrorDto::new(
                "LLM_NO_PROVIDER",
                "No provider specified and no task type for route resolution",
                true,
            ));
        }

        let provider_id = provider_hint.to_string();
        let model = Self::resolve_request_model(&provider_id, &req.model)?;
        Ok((provider_id, model, None))
    }

    /// Inspect task route resolution and computed retry/fallback chain for diagnostics.
    #[allow(dead_code)]
    pub fn inspect_task_route(task_type: &str) -> Result<TaskRouteResolution, AppErrorDto> {
        let canonical_task_type = task_routing::canonical_task_type(task_type).into_owned();
        let (provider_id, model_id, route) = Self::resolve_route(&canonical_task_type)?;
        let attempts = Self::build_attempt_chain(&provider_id, &model_id, route.as_ref())
            .into_iter()
            .map(|(provider_id, model_id)| TaskRouteAttempt {
                provider_id,
                model_id,
            })
            .collect::<Vec<_>>();
        Ok(TaskRouteResolution {
            canonical_task_type,
            provider_id,
            model_id,
            attempts,
        })
    }

    pub fn inspect_task_route_with_skill_registry(
        task_type: &str,
        skill_registry: &SkillRegistry,
    ) -> Result<TaskRouteResolution, AppErrorDto> {
        let canonical_task_type = task_routing::canonical_task_type(task_type).into_owned();
        let req = UnifiedGenerateRequest {
            model: "default".to_string(),
            task_type: Some(canonical_task_type.clone()),
            ..Default::default()
        };
        let (provider_id, model_id, route) =
            Self::resolve_with_skill_override(&req, skill_registry)?;
        let attempts = Self::build_attempt_chain(&provider_id, &model_id, route.as_ref())
            .into_iter()
            .map(|(provider_id, model_id)| TaskRouteAttempt {
                provider_id,
                model_id,
            })
            .collect::<Vec<_>>();
        Ok(TaskRouteResolution {
            canonical_task_type,
            provider_id,
            model_id,
            attempts,
        })
    }

    /// Resolve target with skill taskRoute override support.
    /// If the req.task_type matches a skill with a taskRoute override,
    /// the override's provider_id / model_id take precedence.
    fn resolve_with_skill_override(
        req: &UnifiedGenerateRequest,
        skill_registry: &SkillRegistry,
    ) -> Result<(String, String, Option<TaskRoute>), AppErrorDto> {
        if let Some(task_type) = req.task_type.as_deref() {
            let selected = skill_registry.select_skills_for_task(task_type)?;
            if let Some(route_override) = selected.route_override {
                let provider = route_override.provider.trim().to_string();
                let model = route_override.model.trim().to_string();
                if !provider.is_empty() {
                    let resolved_model = if model.is_empty() {
                        "default".to_string()
                    } else {
                        model.clone()
                    };
                    log::info!(
                        "[SKILL_ROUTE_OVERRIDE] task={} provider={} model={} reason={}",
                        task_type,
                        provider,
                        resolved_model,
                        route_override.reason
                    );
                    let route = Self::build_override_route(task_type, &provider, &resolved_model);
                    return Ok((provider, resolved_model, Some(route)));
                }
                if !model.is_empty() {
                    let (default_provider, _default_model, _default_route) =
                        Self::resolve_request_target(req)?;
                    log::info!(
                        "[SKILL_ROUTE_OVERRIDE] task={} provider={} model={} reason={}",
                        task_type,
                        default_provider,
                        model,
                        route_override.reason
                    );
                    let route = Self::build_override_route(task_type, &default_provider, &model);
                    return Ok((default_provider, model, Some(route)));
                }
            }
        }
        Self::resolve_request_target(req)
    }

    fn build_override_route(task_type: &str, provider: &str, model: &str) -> TaskRoute {
        TaskRoute {
            id: String::new(),
            task_type: task_routing::canonical_task_type(task_type).into_owned(),
            provider_id: provider.to_string(),
            model_id: model.to_string(),
            fallback_provider_id: None,
            fallback_model_id: None,
            max_retries: 1,
            created_at: None,
            updated_at: None,
        }
    }

    fn encode_pipeline_stream_error(error: &AppErrorDto) -> String {
        format!(
            "{}{}::{}",
            PIPELINE_STREAM_ERROR_PREFIX, error.code, error.message
        )
    }

    pub(crate) fn decode_pipeline_stream_error(raw: &str) -> Option<(String, String)> {
        let encoded = raw.strip_prefix(PIPELINE_STREAM_ERROR_PREFIX)?;
        let (code, message) = encoded.split_once("::")?;
        let code = code.trim();
        if code.is_empty() {
            return None;
        }
        Some((code.to_string(), message.to_string()))
    }

    /// Start streaming generation with diagnostic error envelope for pipeline consumers.
    pub async fn stream_generate_for_pipeline(
        &self,
        req: UnifiedGenerateRequest,
        skill_registry: Option<&Arc<StdRwLock<SkillRegistry>>>,
    ) -> Result<mpsc::Receiver<StreamChunk>, AppErrorDto> {
        self.stream_generate_inner(req, skill_registry, true).await
    }

    async fn stream_generate_inner(
        &self,
        req: UnifiedGenerateRequest,
        skill_registry: Option<&Arc<StdRwLock<SkillRegistry>>>,
        with_pipeline_error_envelope: bool,
    ) -> Result<mpsc::Receiver<StreamChunk>, AppErrorDto> {
        let (provider_id, model, route) = match skill_registry {
            Some(registry) => {
                let guard = registry.read().map_err(|err| {
                    AppErrorDto::new("SKILLS_LOCK_FAILED", "skill registry lock failed", false)
                        .with_detail(err.to_string())
                })?;
                Self::resolve_with_skill_override(&req, &guard)?
            }
            None => Self::resolve_request_target(&req)?,
        };
        let attempts = Self::build_attempt_chain(&provider_id, &model, route.as_ref());

        let (tx, rx) = mpsc::channel(256);
        let service = self.clone();

        tokio::spawn(async move {
            let mut last_error: Option<AppErrorDto> = None;

            for (attempt_provider, attempt_model_hint) in attempts {
                let resolved_model =
                    match Self::resolve_request_model(&attempt_provider, &attempt_model_hint) {
                        Ok(model_id) => model_id,
                        Err(e) => {
                            last_error = Some(e);
                            continue;
                        }
                    };

                if let Err(e) = service.ensure_provider_registered(&attempt_provider).await {
                    last_error = Some(e);
                    continue;
                }

                let guard = service.adapters.read().await;
                if let Some(adapter) = guard.get(&attempt_provider) {
                    let mut attempt_req = req.clone();
                    attempt_req.provider_id = Some(attempt_provider.clone());
                    attempt_req.model = resolved_model;
                    match adapter.stream_text(attempt_req, tx.clone()).await {
                        Ok(()) => return,
                        Err(e) => {
                            last_error = Some(AppErrorDto::from(e));
                            continue;
                        }
                    }
                } else {
                    last_error = Some(AppErrorDto::new(
                        "LLM_ADAPTER_NOT_FOUND",
                        &format!("Provider '{}' 未注册", attempt_provider),
                        true,
                    ));
                    continue;
                }
            }

            let terminal_error = last_error.unwrap_or_else(|| {
                AppErrorDto::new("LLM_GENERATE_FAILED", "All providers failed", false)
            });
            let error_text = if with_pipeline_error_envelope {
                Self::encode_pipeline_stream_error(&terminal_error)
            } else {
                terminal_error.message.clone()
            };
            let _ = tx
                .send(StreamChunk {
                    content: String::new(),
                    finish_reason: None,
                    request_id: String::new(),
                    error: Some(error_text),
                    reasoning: None,
                })
                .await;
        });

        Ok(rx)
    }

    /// Test a provider's connection.
    pub async fn test_connection(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        self.ensure_provider_registered(provider_id).await?;
        let guard = self.adapters.read().await;
        let adapter = guard.get(provider_id).ok_or_else(|| {
            AppErrorDto::new(
                "LLM_ADAPTER_NOT_FOUND",
                &format!("Provider '{}' not registered", provider_id),
                true,
            )
        })?;
        adapter.test_connection().await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::AiService;
    use crate::errors::AppErrorDto;
    use crate::services::skill_registry::{SkillManifest, SkillRegistry, SkillTaskRouteOverride};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_registry(name: &str) -> SkillRegistry {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root: PathBuf =
            std::env::temp_dir().join(format!("novelforge-ai-service-test-{name}-{unique}"));
        let skills_dir = root.join("skills");
        let builtin_dir = root.join("builtin");
        std::fs::create_dir_all(&skills_dir).expect("create skills dir");
        std::fs::create_dir_all(&builtin_dir).expect("create builtin dir");
        SkillRegistry::new(skills_dir, builtin_dir)
    }

    fn build_manifest(id: &str, class: &str) -> SkillManifest {
        SkillManifest {
            id: id.to_string(),
            name: format!("{id} name"),
            description: "desc".to_string(),
            version: 1,
            source: "user".to_string(),
            category: "utility".to_string(),
            tags: Vec::new(),
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: serde_json::json!({"type":"object"}),
            requires_user_confirmation: true,
            writes_to_project: false,
            author: None,
            icon: None,
            created_at: "2026-04-30T00:00:00Z".to_string(),
            updated_at: "2026-04-30T00:00:00Z".to_string(),
            skill_class: Some(class.to_string()),
            bundle_ids: Vec::new(),
            always_on: false,
            trigger_conditions: Vec::new(),
            required_contexts: Vec::new(),
            state_writes: Vec::new(),
            automation_tier: None,
            scene_tags: Vec::new(),
            affects_layers: Vec::new(),
            task_route: None,
        }
    }

    #[test]
    fn pipeline_stream_error_roundtrip() {
        let encoded = AiService::encode_pipeline_stream_error(&AppErrorDto::new(
            "TASK_ROUTE_NOT_FOUND",
            "route missing",
            true,
        ));
        let parsed = AiService::decode_pipeline_stream_error(&encoded).expect("decode");
        assert_eq!(parsed.0, "TASK_ROUTE_NOT_FOUND");
        assert_eq!(parsed.1, "route missing");
    }

    #[test]
    fn decode_pipeline_stream_error_rejects_plain_message() {
        assert!(AiService::decode_pipeline_stream_error("just message").is_none());
    }

    #[test]
    fn stream_generate_for_pipeline_uses_skill_route_override() {
        let registry = create_test_registry("route-override");
        let mut workflow = build_manifest("workflow.scene.render", "workflow");
        workflow.trigger_conditions = vec!["custom.scene.render".to_string()];
        workflow.task_route = Some(SkillTaskRouteOverride {
            task_type: "custom.scene.render".to_string(),
            provider_id: "provider-override".to_string(),
            model_id: "model-override".to_string(),
            reason: Some("precision scene rendering".to_string()),
        });
        registry
            .create_skill(&workflow, "workflow body")
            .expect("create workflow skill");

        let resolution =
            AiService::inspect_task_route_with_skill_registry("custom.scene.render", &registry)
                .expect("resolve route with skill override");
        assert_eq!(resolution.provider_id, "provider-override");
        assert_eq!(resolution.model_id, "model-override");
    }
}
