use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::Emitter;
use uuid::Uuid;

use crate::adapters::llm_types::{ContentBlock, Message, UnifiedGenerateRequest};
use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::ai_service::{AiService, TaskRouteResolution};
use crate::services::context_service::{CollectedContext, ContextService};
use crate::services::project_service::get_project_id;
use crate::services::prompt_builder::PromptBuilder;
use crate::services::skill_registry::SkillRegistry;
use crate::services::task_routing;

const PIPELINE_EVENT_NAME: &str = "ai:pipeline:event";
const PHASE_VALIDATE: &str = "validate";
const PHASE_CONTEXT: &str = "context";
const PHASE_ROUTE: &str = "route";
const PHASE_PROMPT: &str = "prompt";
const PHASE_GENERATE: &str = "generate";
const PHASE_POSTPROCESS: &str = "postprocess";
const PHASE_PERSIST: &str = "persist";
const PHASE_DONE: &str = "done";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiTaskPipelineInput {
    pub project_root: String,
    pub task_type: String,
    pub chapter_id: Option<String>,
    pub ui_action: Option<String>,
    #[serde(default)]
    pub user_instruction: String,
    pub selected_text: Option<String>,
    pub chapter_content: Option<String>,
    pub blueprint_step_key: Option<String>,
    pub blueprint_step_title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiTaskPipelineResult {
    pub request_id: String,
    pub task_type: String,
    pub status: String,
    pub output_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPipelineEvent {
    pub request_id: String,
    pub phase: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recoverable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Clone, Default)]
pub struct AiPipelineService {
    cancelled_requests: Arc<RwLock<HashSet<String>>>,
}

#[derive(Debug)]
struct StageError {
    phase: &'static str,
    error: AppErrorDto,
}

#[derive(Debug)]
struct PipelineSuccess {
    output_text: String,
    route: TaskRouteResolution,
}

impl AiPipelineService {
    pub async fn run_ai_task_pipeline(
        &self,
        app_handle: &tauri::AppHandle,
        ai_service: &AiService,
        context_service: &ContextService,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        input: RunAiTaskPipelineInput,
    ) -> Result<RunAiTaskPipelineResult, AppErrorDto> {
        let request_id = Uuid::new_v4().to_string();
        self.run_ai_task_pipeline_with_request_id(
            app_handle,
            ai_service,
            context_service,
            skill_registry,
            request_id,
            input,
        )
        .await
    }

    pub async fn run_ai_task_pipeline_with_request_id(
        &self,
        app_handle: &tauri::AppHandle,
        ai_service: &AiService,
        context_service: &ContextService,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        request_id: String,
        input: RunAiTaskPipelineInput,
    ) -> Result<RunAiTaskPipelineResult, AppErrorDto> {
        let canonical_task = task_routing::canonical_task_type(&input.task_type).into_owned();
        let started_at = Instant::now();

        let run_result = match self.insert_pipeline_run(
            &input.project_root,
            &request_id,
            input.chapter_id.as_deref(),
            &canonical_task,
            input.ui_action.as_deref(),
        ) {
            Ok(()) => {
                self.run_pipeline_inner(
                    app_handle,
                    ai_service,
                    context_service,
                    skill_registry,
                    &request_id,
                    &canonical_task,
                    &input,
                )
                .await
            }
            Err(error) => Err(StageError {
                phase: PHASE_VALIDATE,
                error,
            }),
        };

        match run_result {
            Ok(success) => {
                self.update_pipeline_run(
                    &input.project_root,
                    &request_id,
                    "succeeded",
                    PHASE_DONE,
                    None,
                    None,
                    started_at.elapsed().as_millis() as i64,
                );
                self.emit_event(
                    app_handle,
                    AiPipelineEvent {
                        request_id: request_id.clone(),
                        phase: PHASE_DONE.to_string(),
                        event_type: "done".to_string(),
                        delta: None,
                        error_code: None,
                        message: Some("pipeline completed".to_string()),
                        recoverable: None,
                        meta: Some(json!({
                            "taskType": canonical_task,
                            "outputLength": success.output_text.chars().count(),
                            "providerId": success.route.provider_id,
                            "modelId": success.route.model_id,
                        })),
                    },
                );
                self.clear_cancellation(&request_id);
                Ok(RunAiTaskPipelineResult {
                    request_id,
                    task_type: canonical_task,
                    status: "succeeded".to_string(),
                    output_text: Some(success.output_text),
                })
            }
            Err(stage_error) => {
                self.update_pipeline_run(
                    &input.project_root,
                    &request_id,
                    if stage_error.error.code == "PIPELINE_CANCELLED" {
                        "cancelled"
                    } else {
                        "failed"
                    },
                    stage_error.phase,
                    Some(stage_error.error.code.as_str()),
                    Some(stage_error.error.message.as_str()),
                    started_at.elapsed().as_millis() as i64,
                );
                self.emit_event(
                    app_handle,
                    AiPipelineEvent {
                        request_id: request_id.clone(),
                        phase: stage_error.phase.to_string(),
                        event_type: "error".to_string(),
                        delta: None,
                        error_code: Some(stage_error.error.code.clone()),
                        message: Some(stage_error.error.message.clone()),
                        recoverable: Some(stage_error.error.recoverable),
                        meta: None,
                    },
                );
                self.clear_cancellation(&request_id);
                Err(stage_error.error)
            }
        }
    }

    pub fn cancel_ai_task_pipeline(&self, request_id: &str) -> Result<(), AppErrorDto> {
        let trimmed = request_id.trim();
        if trimmed.is_empty() {
            return Err(AppErrorDto::new(
                "PIPELINE_REQUEST_ID_REQUIRED",
                "requestId 不能为空",
                true,
            ));
        }
        if let Ok(mut guard) = self.cancelled_requests.write() {
            guard.insert(trimmed.to_string());
        }
        Ok(())
    }

    async fn run_pipeline_inner(
        &self,
        app_handle: &tauri::AppHandle,
        ai_service: &AiService,
        context_service: &ContextService,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        request_id: &str,
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> Result<PipelineSuccess, StageError> {
        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_VALIDATE);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_VALIDATE.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("validating input".to_string()),
                recoverable: None,
                meta: Some(json!({ "taskType": canonical_task })),
            },
        );
        self.validate_input(canonical_task, input)
            .map_err(|err| StageError {
                phase: PHASE_VALIDATE,
                error: err,
            })?;
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_VALIDATE,
            error: err,
        })?;

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_CONTEXT);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_CONTEXT.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("collecting context".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let chapter_id = input.chapter_id.as_deref().map(str::trim).unwrap_or("");
        let context = if Self::requires_global_only_context(canonical_task) {
            context_service
                .collect_global_context_only(&input.project_root)
                .map_err(|err| StageError {
                    phase: PHASE_CONTEXT,
                    error: err,
                })?
        } else {
            context_service
                .collect_chapter_context(&input.project_root, chapter_id)
                .map_err(|err| StageError {
                    phase: PHASE_CONTEXT,
                    error: err,
                })?
        };
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_CONTEXT,
            error: err,
        })?;

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_ROUTE);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_ROUTE.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("resolving task route".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let route = AiService::inspect_task_route(canonical_task).map_err(|err| StageError {
            phase: PHASE_ROUTE,
            error: err,
        })?;
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
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
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_ROUTE,
            error: err,
        })?;

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_PROMPT);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
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
            .resolve_or_build_prompt(skill_registry, &context, canonical_task, input)
            .map_err(|err| StageError {
                phase: PHASE_PROMPT,
                error: err,
            })?;
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_PROMPT,
            error: err,
        })?;

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_GENERATE);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
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
                    text: Some(Self::generate_user_message(canonical_task).to_string()),
                }],
            }],
            system_prompt: Some(prompt),
            stream: true,
            task_type: Some(canonical_task.to_string()),
            ..Default::default()
        };
        let mut rx = ai_service
            .stream_generate_for_pipeline(req, None)
            .await
            .map_err(|err| StageError {
                phase: PHASE_GENERATE,
                error: err,
            })?;

        let mut generated = String::new();
        while let Some(chunk) = rx.recv().await {
            self.check_cancelled(request_id).map_err(|err| StageError {
                phase: PHASE_GENERATE,
                error: err,
            })?;
            if let Some(err_msg) = chunk.error {
                if let Some((error_code, message)) =
                    AiService::decode_pipeline_stream_error(&err_msg)
                {
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
                self.emit_event(
                    app_handle,
                    AiPipelineEvent {
                        request_id: request_id.to_string(),
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

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_POSTPROCESS);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_POSTPROCESS.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("normalizing response".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let normalized = self
            .normalize_output(&generated)
            .map_err(|err| StageError {
                phase: PHASE_POSTPROCESS,
                error: err,
            })?;

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_PERSIST);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_PERSIST.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("persisting run audit".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_PERSIST,
            error: err,
        })?;

        Ok(PipelineSuccess {
            output_text: normalized,
            route,
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
            "chapter.draft" | "chapter.continue" | "chapter.plan" => {
                if chapter_id.is_empty() {
                    return Err(AppErrorDto::new(
                        "PIPELINE_CHAPTER_ID_REQUIRED",
                        "该任务需要 chapterId",
                        true,
                    ));
                }
            }
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
            "character.create" | "world.create_rule" | "plot.create_node" => {
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

    fn resolve_or_build_prompt(
        &self,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        context: &CollectedContext,
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> Result<String, AppErrorDto> {
        let selected_text = input.selected_text.as_deref().unwrap_or("");
        let user_instruction = input.user_instruction.as_str();
        if let Ok(guard) = skill_registry.read() {
            if let Ok(Some(content)) = guard.read_skill_content(canonical_task) {
                let context_str = Self::context_to_string(context);
                let rendered = content
                    .replace("{projectContext}", &context_str)
                    .replace("{userInstruction}", user_instruction)
                    .replace("{selectedText}", selected_text);
                return Ok(rendered);
            }
        }

        let prompt = match canonical_task {
            "chapter.continue" => {
                PromptBuilder::build_continue(context, selected_text, user_instruction)
            }
            "chapter.rewrite" => {
                PromptBuilder::build_rewrite(context, selected_text, user_instruction)
            }
            "prose.naturalize" => PromptBuilder::build_naturalize(context, selected_text),
            "chapter.plan" => PromptBuilder::build_chapter_plan(context, user_instruction),
            "character.create" => PromptBuilder::build_character_create(context, user_instruction),
            "world.create_rule" => {
                PromptBuilder::build_world_create_rule(context, user_instruction)
            }
            "plot.create_node" => PromptBuilder::build_plot_create_node(context, user_instruction),
            "consistency.scan" => {
                let chapter_content = input.chapter_content.as_deref().unwrap_or("");
                PromptBuilder::build_consistency_scan(context, chapter_content)
            }
            "blueprint.generate_step" => {
                let step_key = input.blueprint_step_key.as_deref().unwrap_or_default();
                let step_title = input.blueprint_step_title.as_deref().unwrap_or_default();
                PromptBuilder::build_blueprint_step(context, step_key, step_title, user_instruction)
            }
            _ => PromptBuilder::build_chapter_draft(context, user_instruction),
        };
        Ok(prompt)
    }

    fn context_to_string(context: &CollectedContext) -> String {
        let global = &context.global_context;
        let related = &context.related_context;
        let mut parts = vec![];

        parts.push(format!("作品名称：{}", global.project_name));
        parts.push(format!("题材：{}", global.genre));
        if let Some(ref pov) = global.narrative_pov {
            parts.push(format!("叙事视角：{}", pov));
        }
        for step in &global.blueprint_summary {
            if step.status == "completed" {
                if let Some(ref content) = step.content {
                    let preview: String = content.chars().take(200).collect();
                    parts.push(format!("[蓝图] {}: {}", step.title, preview));
                }
            }
        }
        if let Some(ref ch) = related.chapter {
            parts.push(format!("章节：{}", ch.title));
            if !ch.summary.is_empty() {
                parts.push(format!("摘要：{}", ch.summary));
            }
        }
        for node in &related.plot_nodes {
            parts.push(format!("剧情节点：{}", node.title));
        }
        for ch in &related.characters {
            parts.push(format!("角色：{}", ch.name));
        }
        for rule in &related.world_rules {
            let preview: String = rule.description.chars().take(120).collect();
            parts.push(format!("世界规则：{}", preview));
        }
        parts.join("\n")
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

    fn insert_pipeline_run(
        &self,
        project_root: &str,
        request_id: &str,
        chapter_id: Option<&str>,
        task_type: &str,
        ui_action: Option<&str>,
    ) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("PIPELINE_DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();

        conn.execute(
            "INSERT INTO ai_pipeline_runs(id, project_id, chapter_id, task_type, ui_action, status, phase, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6, ?7)",
            params![
                request_id,
                project_id,
                chapter_id.map(str::trim).filter(|v| !v.is_empty()),
                task_type,
                ui_action.map(str::trim).filter(|v| !v.is_empty()),
                PHASE_VALIDATE,
                now
            ],
        )
        .map_err(|err| {
            AppErrorDto::new("PIPELINE_AUDIT_INSERT_FAILED", "记录 pipeline 运行失败", false)
                .with_detail(err.to_string())
        })?;
        Ok(())
    }

    fn update_pipeline_run(
        &self,
        project_root: &str,
        request_id: &str,
        status: &str,
        phase: &str,
        error_code: Option<&str>,
        error_message: Option<&str>,
        duration_ms: i64,
    ) {
        let conn = match open_database(Path::new(project_root)) {
            Ok(conn) => conn,
            Err(_) => return,
        };
        let _ = conn.execute(
            "UPDATE ai_pipeline_runs
             SET status = ?1,
                 phase = ?2,
                 error_code = ?3,
                 error_message = ?4,
                 duration_ms = ?5,
                 completed_at = ?6
             WHERE id = ?7",
            params![
                status,
                phase,
                error_code,
                error_message,
                duration_ms,
                now_iso(),
                request_id
            ],
        );
    }

    fn touch_pipeline_phase(&self, project_root: &str, request_id: &str, phase: &str) {
        let conn = match open_database(Path::new(project_root)) {
            Ok(conn) => conn,
            Err(_) => return,
        };
        let _ = conn.execute(
            "UPDATE ai_pipeline_runs
             SET phase = ?1
             WHERE id = ?2
               AND status = 'running'",
            params![phase, request_id],
        );
    }

    fn emit_event(&self, app_handle: &tauri::AppHandle, event: AiPipelineEvent) {
        let _ = app_handle.emit(PIPELINE_EVENT_NAME, event);
    }

    fn check_cancelled(&self, request_id: &str) -> Result<(), AppErrorDto> {
        if self.is_cancelled(request_id) {
            return Err(AppErrorDto::new("PIPELINE_CANCELLED", "任务已取消", true));
        }
        Ok(())
    }

    fn is_cancelled(&self, request_id: &str) -> bool {
        self.cancelled_requests
            .read()
            .map(|guard| guard.contains(request_id))
            .unwrap_or(false)
    }

    fn clear_cancellation(&self, request_id: &str) {
        if let Ok(mut guard) = self.cancelled_requests.write() {
            guard.remove(request_id);
        }
    }

    fn requires_global_only_context(task_type: &str) -> bool {
        matches!(
            task_type,
            "character.create"
                | "world.create_rule"
                | "plot.create_node"
                | "blueprint.generate_step"
        )
    }

    fn generate_user_message(task_type: &str) -> &'static str {
        match task_type {
            "character.create" => "请根据用户设想生成角色卡 JSON。",
            "world.create_rule" => "请根据用户设想生成世界设定 JSON。",
            "plot.create_node" => "请根据用户设想生成剧情节点 JSON。",
            "consistency.scan" => "请检查章节一致性并输出 JSON。",
            _ => "请根据上述要求生成内容。",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use rusqlite::params;
    use uuid::Uuid;

    use super::{AiPipelineService, RunAiTaskPipelineInput, PHASE_ROUTE};
    use crate::infra::database::open_database;
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn base_input(task_type: &str) -> RunAiTaskPipelineInput {
        RunAiTaskPipelineInput {
            project_root: "F:/tmp/project".to_string(),
            task_type: task_type.to_string(),
            chapter_id: Some("ch-1".to_string()),
            ui_action: Some("action".to_string()),
            user_instruction: "请继续".to_string(),
            selected_text: Some("示例".to_string()),
            chapter_content: Some("正文".to_string()),
            blueprint_step_key: Some("step-01-anchor".to_string()),
            blueprint_step_title: Some("锚点".to_string()),
        }
    }

    #[test]
    fn validate_rewrite_requires_selected_text() {
        let service = AiPipelineService::default();
        let mut input = base_input("chapter.rewrite");
        input.selected_text = Some("   ".to_string());
        let err = service
            .validate_input("chapter.rewrite", &input)
            .expect_err("expected error");
        assert_eq!(err.code, "PIPELINE_SELECTED_TEXT_REQUIRED");
    }

    #[test]
    fn validate_consistency_requires_chapter_content() {
        let service = AiPipelineService::default();
        let mut input = base_input("consistency.scan");
        input.chapter_content = Some(" ".to_string());
        let err = service
            .validate_input("consistency.scan", &input)
            .expect_err("expected error");
        assert_eq!(err.code, "PIPELINE_CHAPTER_CONTENT_REQUIRED");
    }

    #[test]
    fn normalize_output_removes_fence() {
        let service = AiPipelineService::default();
        let output = service
            .normalize_output("```json\n{\"a\":1}\n```")
            .expect("normalized");
        assert_eq!(output, "{\"a\":1}");
    }

    #[test]
    fn touch_pipeline_phase_updates_audit_phase() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "pipeline-audit-phase-test".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");

        let service = AiPipelineService::default();
        let request_id = Uuid::new_v4().to_string();
        service
            .insert_pipeline_run(
                &project.project_root,
                &request_id,
                None,
                "chapter.draft",
                Some("editor.ai.chapter.draft"),
            )
            .expect("insert run");
        service.touch_pipeline_phase(&project.project_root, &request_id, PHASE_ROUTE);

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let phase: String = conn
            .query_row(
                "SELECT phase FROM ai_pipeline_runs WHERE id = ?1",
                params![request_id],
                |row| row.get(0),
            )
            .expect("query phase");
        assert_eq!(phase, PHASE_ROUTE);

        remove_temp_workspace(&workspace);
    }
}
