use std::sync::{Arc, RwLock};

use serde_json::json;

use crate::adapters::llm_types::{ContentBlock, Message, UnifiedGenerateRequest};
use crate::errors::AppErrorDto;
use crate::services::ai_pipeline::audit_store::PipelineAuditStore;
use crate::services::ai_pipeline::prompt_resolver::PromptResolver;
use crate::services::ai_pipeline::task_handlers::TaskHandlers;
use crate::services::ai_pipeline_service::{
    AiPipelineEvent, AiPipelineService, PersistedRecord, RunAiTaskPipelineInput,
};
use crate::services::ai_service::{AiService, TaskRouteResolution};
use crate::services::context_service::ContextService;
use crate::services::skill_registry::SkillRegistry;

pub const PHASE_VALIDATE: &str = "validate";
pub const PHASE_CONTEXT: &str = "context";
pub const PHASE_ROUTE: &str = "route";
pub const PHASE_PROMPT: &str = "prompt";
pub const PHASE_GENERATE: &str = "generate";
pub const PHASE_POSTPROCESS: &str = "postprocess";
pub const PHASE_PERSIST: &str = "persist";
pub const PHASE_DONE: &str = "done";

#[derive(Debug)]
pub struct StageError {
    pub phase: &'static str,
    pub error: AppErrorDto,
}

#[derive(Debug)]
pub struct PipelineSuccess {
    pub output_text: String,
    pub route: TaskRouteResolution,
    pub persisted_records: Vec<PersistedRecord>,
}

pub struct PipelineOrchestrator<'a> {
    pub pipeline_service: &'a AiPipelineService,
    pub audit_store: &'a PipelineAuditStore,
    pub prompt_resolver: &'a PromptResolver,
    pub task_handlers: &'a TaskHandlers,
    pub app_handle: &'a tauri::AppHandle,
    pub ai_service: &'a AiService,
    pub context_service: &'a ContextService,
    pub skill_registry: &'a Arc<RwLock<SkillRegistry>>,
    pub request_id: &'a str,
    pub canonical_task: &'a str,
    pub input: &'a RunAiTaskPipelineInput,
}

impl<'a> PipelineOrchestrator<'a> {
    pub async fn run(&self) -> Result<PipelineSuccess, StageError> {
        self.audit_store
            .touch_pipeline_phase(&self.input.project_root, self.request_id, PHASE_VALIDATE);
        self.pipeline_service.emit_event(
            self.app_handle,
            AiPipelineEvent {
                request_id: self.request_id.to_string(),
                phase: PHASE_VALIDATE.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("validating input".to_string()),
                recoverable: None,
                meta: Some(json!({ "taskType": self.canonical_task })),
            },
        );
        self.validate_input(self.canonical_task, self.input)
            .map_err(|err| StageError {
                phase: PHASE_VALIDATE,
                error: err,
            })?;
        self.pipeline_service
            .check_cancelled(self.request_id)
            .map_err(|err| StageError {
                phase: PHASE_VALIDATE,
                error: err,
            })?;

        self.audit_store
            .touch_pipeline_phase(&self.input.project_root, self.request_id, PHASE_CONTEXT);
        self.pipeline_service.emit_event(
            self.app_handle,
            AiPipelineEvent {
                request_id: self.request_id.to_string(),
                phase: PHASE_CONTEXT.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("collecting context".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let chapter_id = self.input.chapter_id.as_deref().map(str::trim).unwrap_or("");
        let context = if PromptResolver::requires_global_only_context(self.canonical_task) {
            self.context_service
                .collect_global_context_only(&self.input.project_root)
                .map_err(|err| StageError {
                    phase: PHASE_CONTEXT,
                    error: err,
                })?
        } else {
            self.context_service
                .collect_chapter_context(&self.input.project_root, chapter_id)
                .map_err(|err| StageError {
                    phase: PHASE_CONTEXT,
                    error: err,
                })?
        };
        self.pipeline_service
            .check_cancelled(self.request_id)
            .map_err(|err| StageError {
                phase: PHASE_CONTEXT,
                error: err,
            })?;

        self.audit_store
            .touch_pipeline_phase(&self.input.project_root, self.request_id, PHASE_ROUTE);
        self.pipeline_service.emit_event(
            self.app_handle,
            AiPipelineEvent {
                request_id: self.request_id.to_string(),
                phase: PHASE_ROUTE.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("resolving task route".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let route = AiService::inspect_task_route(self.canonical_task).map_err(|err| StageError {
            phase: PHASE_ROUTE,
            error: err,
        })?;
        self.pipeline_service.emit_event(
            self.app_handle,
            AiPipelineEvent {
                request_id: self.request_id.to_string(),
                phase: PHASE_ROUTE.to_string(),
                event_type: "progress".to_string(),
                delta: None,
                error_code: None,
                message: Some("route resolved".to_string()),
                recoverable: None,
                meta: Some(json!({
                    "providerId": route.provider_id.clone(),
                    "modelId": route.model_id.clone(),
                    "attempts": route.attempts.clone(),
                })),
            },
        );
        self.pipeline_service
            .check_cancelled(self.request_id)
            .map_err(|err| StageError {
                phase: PHASE_ROUTE,
                error: err,
            })?;

        self.audit_store
            .touch_pipeline_phase(&self.input.project_root, self.request_id, PHASE_PROMPT);
        self.pipeline_service.emit_event(
            self.app_handle,
            AiPipelineEvent {
                request_id: self.request_id.to_string(),
                phase: PHASE_PROMPT.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("building prompt".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let prompt = self
            .prompt_resolver
            .resolve_or_build_prompt(
                self.skill_registry,
                &context,
                self.canonical_task,
                self.input,
            )
            .map_err(|err| StageError {
                phase: PHASE_PROMPT,
                error: err,
            })?;
        self.pipeline_service
            .check_cancelled(self.request_id)
            .map_err(|err| StageError {
                phase: PHASE_PROMPT,
                error: err,
            })?;

        self.audit_store
            .touch_pipeline_phase(&self.input.project_root, self.request_id, PHASE_GENERATE);
        self.pipeline_service.emit_event(
            self.app_handle,
            AiPipelineEvent {
                request_id: self.request_id.to_string(),
                phase: PHASE_GENERATE.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("streaming generate start".to_string()),
                recoverable: None,
                meta: Some(json!({
                    "providerId": route.provider_id.clone(),
                    "modelId": route.model_id.clone(),
                })),
            },
        );
        let req = UnifiedGenerateRequest {
            model: "default".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![ContentBlock {
                    block_type: "text".to_string(),
                    text: Some(PromptResolver::generate_user_message(self.canonical_task).to_string()),
                }],
            }],
            system_prompt: Some(prompt),
            stream: true,
            task_type: Some(self.canonical_task.to_string()),
            ..Default::default()
        };
        let mut rx = self
            .ai_service
            .stream_generate_for_pipeline(req, None)
            .await
            .map_err(|err| StageError {
                phase: PHASE_GENERATE,
                error: err,
            })?;

        let mut generated = String::new();
        while let Some(chunk) = rx.recv().await {
            self.pipeline_service
                .check_cancelled(self.request_id)
                .map_err(|err| StageError {
                    phase: PHASE_GENERATE,
                    error: err,
                })?;
            if let Some(err_msg) = chunk.error {
                if let Some((error_code, message)) = AiService::decode_pipeline_stream_error(&err_msg) {
                    return Err(StageError {
                        phase: PHASE_GENERATE,
                        error: AppErrorDto::new(&error_code, &message, true),
                    });
                }
                return Err(StageError {
                    phase: PHASE_GENERATE,
                    error: AppErrorDto::new("PIPELINE_GENERATE_FAILED", &err_msg, true),
                });
            }
            if !chunk.content.is_empty() {
                generated.push_str(&chunk.content);
                self.pipeline_service.emit_event(
                    self.app_handle,
                    AiPipelineEvent {
                        request_id: self.request_id.to_string(),
                        phase: PHASE_GENERATE.to_string(),
                        event_type: "delta".to_string(),
                        delta: Some(chunk.content),
                        error_code: None,
                        message: None,
                        recoverable: None,
                        meta: None,
                    },
                );
            }
        }

        self.audit_store
            .touch_pipeline_phase(&self.input.project_root, self.request_id, PHASE_POSTPROCESS);
        self.pipeline_service.emit_event(
            self.app_handle,
            AiPipelineEvent {
                request_id: self.request_id.to_string(),
                phase: PHASE_POSTPROCESS.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("normalizing response".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let normalized = self.normalize_output(&generated).map_err(|err| StageError {
            phase: PHASE_POSTPROCESS,
            error: err,
        })?;

        self.audit_store
            .touch_pipeline_phase(&self.input.project_root, self.request_id, PHASE_PERSIST);
        self.pipeline_service.emit_event(
            self.app_handle,
            AiPipelineEvent {
                request_id: self.request_id.to_string(),
                phase: PHASE_PERSIST.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("persisting run audit".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        self.pipeline_service
            .check_cancelled(self.request_id)
            .map_err(|err| StageError {
                phase: PHASE_PERSIST,
                error: err,
            })?;

        let persisted_records = if self.input.auto_persist {
            self.task_handlers
                .persist_task_output(
                    self.canonical_task,
                    &self.input.project_root,
                    self.input,
                    &normalized,
                    self.request_id,
                )
                .map_err(|err| StageError {
                    phase: PHASE_PERSIST,
                    error: err,
                })?
        } else {
            Vec::new()
        };

        if !persisted_records.is_empty() {
            self.pipeline_service.emit_event(
                self.app_handle,
                AiPipelineEvent {
                    request_id: self.request_id.to_string(),
                    phase: PHASE_PERSIST.to_string(),
                    event_type: "progress".to_string(),
                    delta: None,
                    error_code: None,
                    message: Some("business data persisted".to_string()),
                    recoverable: None,
                    meta: Some(json!({
                        "persistedRecords": persisted_records.clone(),
                    })),
                },
            );
        }

        Ok(PipelineSuccess {
            output_text: normalized,
            route,
            persisted_records,
        })
    }

    fn validate_input(
        &self,
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> Result<(), AppErrorDto> {
        if input.project_root.trim().is_empty() {
            return Err(AppErrorDto::new(
                "PIPELINE_PROJECT_ROOT_REQUIRED",
                "projectRoot 不能为空",
                true,
            ));
        }

        let chapter_id = input.chapter_id.as_deref().map(str::trim).unwrap_or("");
        let selected_text = input.selected_text.as_deref().map(str::trim).unwrap_or("");
        let chapter_content = input
            .chapter_content
            .as_deref()
            .map(str::trim)
            .unwrap_or("");
        let user_instruction = input.user_instruction.trim();

        match canonical_task {
            "chapter.draft" | "chapter.continue" => {
                if chapter_id.is_empty() {
                    return Err(AppErrorDto::new(
                        "PIPELINE_CHAPTER_ID_REQUIRED",
                        "该任务需要 chapterId",
                        true,
                    ));
                }
            }
            "chapter.plan" => {}
            "chapter.rewrite" | "prose.naturalize" => {
                if chapter_id.is_empty() {
                    return Err(AppErrorDto::new(
                        "PIPELINE_CHAPTER_ID_REQUIRED",
                        "该任务需要 chapterId",
                        true,
                    ));
                }
                if selected_text.is_empty() {
                    return Err(AppErrorDto::new(
                        "PIPELINE_SELECTED_TEXT_REQUIRED",
                        "该任务需要 selectedText",
                        true,
                    ));
                }
            }
            "character.create"
            | "world.create_rule"
            | "plot.create_node"
            | "glossary.create_term"
            | "narrative.create_obligation" => {
                if user_instruction.is_empty() {
                    return Err(AppErrorDto::new(
                        "PIPELINE_USER_INSTRUCTION_REQUIRED",
                        "该任务需要 userInstruction",
                        true,
                    ));
                }
            }
            "consistency.scan" => {
                if chapter_id.is_empty() {
                    return Err(AppErrorDto::new(
                        "PIPELINE_CHAPTER_ID_REQUIRED",
                        "该任务需要 chapterId",
                        true,
                    ));
                }
                if chapter_content.is_empty() {
                    return Err(AppErrorDto::new(
                        "PIPELINE_CHAPTER_CONTENT_REQUIRED",
                        "一致性扫描需要 chapterContent",
                        true,
                    ));
                }
            }
            "blueprint.generate_step" => {
                let step_key = input
                    .blueprint_step_key
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("");
                let step_title = input
                    .blueprint_step_title
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("");
                if step_key.is_empty() || step_title.is_empty() {
                    return Err(AppErrorDto::new(
                        "PIPELINE_BLUEPRINT_STEP_REQUIRED",
                        "蓝图任务需要 stepKey 与 stepTitle",
                        true,
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn normalize_output(&self, raw: &str) -> Result<String, AppErrorDto> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(AppErrorDto::new(
                "PIPELINE_EMPTY_OUTPUT",
                "模型未返回可用内容",
                true,
            ));
        }
        if !trimmed.starts_with("```") {
            return Ok(trimmed.to_string());
        }

        let mut lines = trimmed.lines().collect::<Vec<_>>();
        if !lines.is_empty() && lines[0].starts_with("```") {
            lines.remove(0);
        }
        if !lines.is_empty() && lines[lines.len() - 1].starts_with("```") {
            lines.pop();
        }
        let normalized = lines.join("\n").trim().to_string();
        if normalized.is_empty() {
            return Err(AppErrorDto::new(
                "PIPELINE_EMPTY_OUTPUT",
                "模型未返回可用内容",
                true,
            ));
        }
        Ok(normalized)
    }
}
