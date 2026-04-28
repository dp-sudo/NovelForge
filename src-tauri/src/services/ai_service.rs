use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::adapters::{
    anthropic::AnthropicAdapter, gemini::GeminiAdapter, llm_service::LlmService, llm_types::*,
    openai_compatible::OpenAiCompatibleAdapter,
};
use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::infra::{app_database, credential_manager};

#[derive(Clone)]
pub struct AiService {
    adapters: Arc<RwLock<HashMap<String, Box<dyn LlmService>>>>,
}

impl Default for AiService {
    fn default() -> Self {
        Self {
            adapters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AiService {
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
        let route = routes.into_iter().find(|r| r.task_type == task_type);

        match route {
            Some(r) => Ok((r.provider_id.clone(), r.model_id.clone(), Some(r))),
            None => Err(AppErrorDto::new(
                "TASK_ROUTE_NOT_FOUND",
                &format!("No route configured for task type '{}'", task_type),
                true,
            )),
        }
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

    /// Execute text generation with task routing + fallback.
    pub async fn generate_text(
        &self,
        req: UnifiedGenerateRequest,
    ) -> Result<UnifiedGenerateResponse, AppErrorDto> {
        let (provider_id, model, route) = Self::resolve_request_target(&req)?;

        let retries = route.as_ref().map(|r| r.max_retries).unwrap_or(1);
        let fallback_ids = route.as_ref().and_then(|r| {
            r.fallback_provider_id
                .as_ref()
                .map(|fp| (fp.clone(), r.fallback_model_id.clone().unwrap_or_default()))
        });

        let mut last_error = None;
        for attempt in 0..retries.max(1) {
            let current_pid = if attempt == 0 {
                &provider_id
            } else {
                match &fallback_ids {
                    Some((fp, _)) => fp,
                    None => break,
                }
            };
            let current_model = if attempt == 0 {
                &model
            } else {
                match &fallback_ids {
                    Some((_, ref fm)) if !fm.is_empty() => fm,
                    _ => &model,
                }
            };

            if self.ensure_provider_registered(current_pid).await.is_err() {
                last_error = Some(LlmError::ModelNotFound);
                continue;
            }

            let guard = self.adapters.read().await;
            if let Some(adapter) = guard.get(current_pid) {
                let mut attempt_req = req.clone();
                attempt_req.provider_id = Some(current_pid.clone());
                attempt_req.model = current_model.clone();
                match adapter.generate_text(attempt_req).await {
                    Ok(resp) => return Ok(resp),
                    Err(e) => {
                        last_error = Some(e);
                        continue;
                    }
                }
            } else {
                last_error = Some(LlmError::ModelNotFound);
                continue;
            }
        }

        Err(match last_error {
            Some(e) => AppErrorDto::from(e),
            None => AppErrorDto::new("LLM_GENERATE_FAILED", "All providers failed", false),
        })
    }

    /// Start streaming generation with task routing.
    /// Returns an mpsc receiver that yields StreamChunks.
    pub async fn stream_generate(
        &self,
        req: UnifiedGenerateRequest,
    ) -> Result<mpsc::Receiver<StreamChunk>, AppErrorDto> {
        let (provider_id, model, route) = Self::resolve_request_target(&req)?;
        let fallback = route.as_ref().and_then(|r| {
            r.fallback_provider_id
                .as_ref()
                .map(|fp| (fp.clone(), r.fallback_model_id.clone()))
        });

        let (tx, rx) = mpsc::channel(256);
        let service = self.clone();

        tokio::spawn(async move {
            let mut attempts: Vec<(String, String)> = vec![(provider_id.clone(), model.clone())];
            if let Some((fallback_provider, fallback_model)) = fallback {
                let fallback_model_id = fallback_model.unwrap_or_else(|| model.clone());
                attempts.push((fallback_provider, fallback_model_id));
            }

            for (attempt_provider, attempt_model) in &attempts {
                // Try to ensure provider is registered; send error if not
                if let Err(e) = service.ensure_provider_registered(attempt_provider).await {
                    let _ = tx
                        .send(StreamChunk {
                            content: String::new(),
                            finish_reason: None,
                            request_id: String::new(),
                            error: Some(e.message),
                        })
                        .await;
                    continue;
                }

                let guard = service.adapters.read().await;
                if let Some(adapter) = guard.get(attempt_provider) {
                    let mut attempt_req = req.clone();
                    attempt_req.provider_id = Some(attempt_provider.clone());
                    attempt_req.model = attempt_model.clone();
                    match adapter.stream_text(attempt_req, tx.clone()).await {
                        Ok(()) => return,
                        Err(e) => {
                            let _ = tx
                                .send(StreamChunk {
                                    content: String::new(),
                                    finish_reason: None,
                                    request_id: String::new(),
                                    error: Some(e.user_message()),
                                })
                                .await;
                            continue;
                        }
                    }
                } else {
                    let _ = tx
                        .send(StreamChunk {
                            content: String::new(),
                            finish_reason: None,
                            request_id: String::new(),
                            error: Some(format!("Provider '{}' 未注册", attempt_provider)),
                        })
                        .await;
                }
            }
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

    // ── AI request logging ──

    /// Record an AI request in the project database for traceability.
    pub fn log_ai_request(
        &self,
        project_root: &str,
        task_type: &str,
        provider: Option<&str>,
        model: Option<&str>,
        prompt_preview: &str,
        status: &str,
    ) -> Result<String, AppErrorDto> {
        let conn = open_database(std::path::Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(err.to_string())
        })?;

        let project_id = crate::services::project_service::get_project_id(&conn)?;
        let request_id = Uuid::new_v4().to_string();
        let now = now_iso();

        // Truncate prompt preview for logging
        let preview: String = prompt_preview.chars().take(240).collect();

        conn.execute(
            "INSERT INTO ai_requests(id, project_id, task_type, provider, model, prompt_preview, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![request_id, project_id, task_type, provider, model, preview, status, now],
        ).map_err(|err| {
            AppErrorDto::new("AI_LOG_FAILED", "记录 AI 请求失败", false)
                .with_detail(err.to_string())
        })?;

        Ok(request_id)
    }

    /// Update an AI request record with completion info.
    pub fn complete_ai_request(
        &self,
        project_root: &str,
        request_id: &str,
        status: &str,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) {
        if let Ok(conn) = open_database(std::path::Path::new(project_root)) {
            let now = now_iso();
            let _ = conn.execute(
                "UPDATE ai_requests SET status = ?1, error_code = ?2, error_message = ?3, completed_at = ?4 WHERE id = ?5",
                rusqlite::params![status, error_code, error_message, now, request_id],
            );
        }
    }
}

// ── Legacy preview types (kept for backward compatibility) ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPreviewResult {
    pub request_id: String,
    pub preview: String,
    pub used_context: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePreviewInput {
    pub task_type: String,
    pub user_instruction: String,
    pub chapter_id: Option<String>,
    pub selected_text: Option<String>,
}
