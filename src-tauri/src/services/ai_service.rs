use std::collections::HashMap;
use std::sync::Arc;

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
            .map(|r| r.max_retries.max(1).min(8) as usize)
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

    /// Resolve target with skill taskRoute override support.
    /// If the req.task_type matches a skill with a taskRoute override,
    /// the override's provider_id / model_id take precedence.
    fn resolve_with_skill_override(
        req: &UnifiedGenerateRequest,
        skill_registry: &SkillRegistry,
    ) -> Result<(String, String, Option<TaskRoute>), AppErrorDto> {
        // Check if a skill with taskRoute override exists
        if let Some(ref task_type) = req.task_type {
            if let Ok(Some(skill)) = skill_registry.get_skill(task_type) {
                if let Some(ref route_override) = skill.task_route {
                    let pid = if route_override.provider_id.is_empty() {
                        // No override → use global route
                        return Self::resolve_request_target(req);
                    } else {
                        route_override.provider_id.clone()
                    };
                    let mid = if route_override.model_id.is_empty() {
                        "default".to_string()
                    } else {
                        route_override.model_id.clone()
                    };
                    let fake_route = TaskRoute {
                        id: String::new(),
                        task_type: route_override.task_type.clone(),
                        provider_id: pid.clone(),
                        model_id: mid.clone(),
                        fallback_provider_id: None,
                        fallback_model_id: None,
                        max_retries: 1,
                        created_at: None,
                        updated_at: None,
                    };
                    return Ok((pid, mid, Some(fake_route)));
                }
            }
        }
        Self::resolve_request_target(req)
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
        skill_registry: Option<&SkillRegistry>,
    ) -> Result<mpsc::Receiver<StreamChunk>, AppErrorDto> {
        self.stream_generate_inner(req, skill_registry, true).await
    }

    async fn stream_generate_inner(
        &self,
        req: UnifiedGenerateRequest,
        skill_registry: Option<&SkillRegistry>,
        with_pipeline_error_envelope: bool,
    ) -> Result<mpsc::Receiver<StreamChunk>, AppErrorDto> {
        let (provider_id, model, route) = match skill_registry {
            Some(reg) => Self::resolve_with_skill_override(&req, reg)?,
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
}
