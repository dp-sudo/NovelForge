use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Emitter;
use uuid::Uuid;

use crate::adapters::llm_types::{ContentBlock, Message, UnifiedGenerateRequest};
use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::ai_service::{AiService, TaskRouteResolution};
use crate::services::capability_pack_service::CapabilityPackService;
use crate::services::constitution_service::ConstitutionService;
use crate::services::context_service::{CollectedContext, ContextService};
use crate::services::glossary_service::CreateGlossaryTermInput;
use crate::services::narrative_service::CreateObligationInput;
use crate::services::project_service::get_project_id;
use crate::services::prompt_builder::PromptBuilder;
use crate::services::skill_registry::SkillRegistry;
use crate::services::state_tracker_service::StateTrackerService;
use crate::services::task_routing;
use crate::services::{
    blueprint_service::{BlueprintService, SaveBlueprintStepInput},
    character_service::{CharacterService, CreateCharacterInput},
    glossary_service::GlossaryService,
    narrative_service::NarrativeService,
    plot_service::{CreatePlotNodeInput, PlotService},
    world_service::{CreateWorldRuleInput, WorldService},
};

const PIPELINE_EVENT_NAME: &str = "ai:pipeline:event";
const PHASE_VALIDATE: &str = "validate";
const PHASE_COMPILE_CONTEXT: &str = "compile_context";
const PHASE_ROUTE: &str = "route";
const PHASE_COMPOSE_PROMPT: &str = "compose_prompt";
const PHASE_GENERATE: &str = "generate";
const PHASE_POSTPROCESS: &str = "postprocess";
const PHASE_REVIEW: &str = "review";
const PHASE_PERSIST: &str = "persist";
const PHASE_CHECKPOINT: &str = "checkpoint";

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
    #[serde(default)]
    pub auto_persist: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiTaskPipelineResult {
    pub request_id: String,
    pub task_type: String,
    pub status: String,
    pub output_text: Option<String>,
    pub persisted_records: Vec<PersistedRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedRecord {
    pub entity_type: String,
    pub entity_id: String,
    pub action: String,
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
    persisted_records: Vec<PersistedRecord>,
    story_checkpoint_id: Option<String>,
    context_snapshot_id: Option<String>,
    review_queue_count: usize,
    context_compilation_snapshot: Value,
    review_checklist: Vec<ReviewChecklistItem>,
    review_work_items: Vec<ReviewWorkItemBrief>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewChecklistItem {
    key: String,
    title: String,
    severity: String,
    status: String,
    message: String,
}

#[derive(Debug, Clone)]
struct ReviewChecklist {
    items: Vec<ReviewChecklistItem>,
    requires_human_review: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewWorkItemBrief {
    id: String,
    key: String,
    title: String,
    severity: String,
    message: String,
    status: String,
}

#[derive(Debug, Clone)]
struct StoryCheckpointRecord {
    checkpoint_id: String,
    review_queue_count: usize,
    status: String,
    review_work_items: Vec<ReviewWorkItemBrief>,
}

impl AiPipelineService {
    pub async fn run_ai_task_pipeline(
        &self,
        app_handle: &tauri::AppHandle,
        ai_service: &AiService,
        context_service: &ContextService,
        constitution_service: &ConstitutionService,
        state_tracker_service: &StateTrackerService,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        input: RunAiTaskPipelineInput,
    ) -> Result<RunAiTaskPipelineResult, AppErrorDto> {
        let request_id = Uuid::new_v4().to_string();
        self.run_ai_task_pipeline_with_request_id(
            app_handle,
            ai_service,
            context_service,
            constitution_service,
            state_tracker_service,
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
        constitution_service: &ConstitutionService,
        state_tracker_service: &StateTrackerService,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        request_id: String,
        input: RunAiTaskPipelineInput,
    ) -> Result<RunAiTaskPipelineResult, AppErrorDto> {
        let canonical_task = task_routing::canonical_task_type(&input.task_type).into_owned();
        let task_contract = task_routing::task_execution_contract(&canonical_task);
        let started_at = Instant::now();

        let run_result = match self.insert_pipeline_run(
            &input.project_root,
            &request_id,
            input.chapter_id.as_deref(),
            &canonical_task,
            input.ui_action.as_deref(),
            &task_contract,
        ) {
            Ok(()) => {
                self.run_pipeline_inner(
                    app_handle,
                    ai_service,
                    context_service,
                    constitution_service,
                    state_tracker_service,
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
                    PHASE_CHECKPOINT,
                    None,
                    None,
                    started_at.elapsed().as_millis() as i64,
                );
                self.emit_event(
                    app_handle,
                    AiPipelineEvent {
                        request_id: request_id.clone(),
                        phase: PHASE_CHECKPOINT.to_string(),
                        event_type: "done".to_string(),
                        delta: None,
                        error_code: None,
                        message: Some("pipeline completed".to_string()),
                        recoverable: None,
                        meta: Some(json!({
                            "taskType": canonical_task,
                            "taskContract": task_contract,
                            "contextCompilationSnapshot": success.context_compilation_snapshot,
                            "contextSnapshotId": success.context_snapshot_id,
                            "reviewChecklist": success.review_checklist,
                            "reviewWorkItems": success.review_work_items,
                            "checkpointId": success.story_checkpoint_id.clone(),
                            "outputLength": success.output_text.chars().count(),
                            "providerId": success.route.provider_id,
                            "modelId": success.route.model_id,
                            "persistedRecords": success.persisted_records.clone(),
                            "storyCheckpointId": success.story_checkpoint_id,
                            "reviewQueueCount": success.review_queue_count,
                        })),
                    },
                );
                self.clear_cancellation(&request_id);
                Ok(RunAiTaskPipelineResult {
                    request_id,
                    task_type: canonical_task,
                    status: "succeeded".to_string(),
                    output_text: Some(success.output_text),
                    persisted_records: success.persisted_records,
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
        constitution_service: &ConstitutionService,
        state_tracker_service: &StateTrackerService,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        request_id: &str,
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> Result<PipelineSuccess, StageError> {
        let task_contract = task_routing::task_execution_contract(canonical_task);
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
                meta: Some(json!({
                    "taskType": canonical_task,
                    "taskContract": task_contract,
                })),
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

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_COMPILE_CONTEXT);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_COMPILE_CONTEXT.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("compiling context".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let chapter_id = input.chapter_id.as_deref().map(str::trim).unwrap_or("");
        let context = if Self::requires_global_only_context(canonical_task) {
            context_service
                .collect_global_context_only(&input.project_root)
                .map_err(|err| StageError {
                    phase: PHASE_COMPILE_CONTEXT,
                    error: err,
                })?
        } else {
            context_service
                .collect_chapter_context(&input.project_root, chapter_id)
                .map_err(|err| StageError {
                    phase: PHASE_COMPILE_CONTEXT,
                    error: err,
                })?
        };
        let context_compilation_snapshot =
            Self::build_context_manifest(&context, canonical_task, &task_contract);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_COMPILE_CONTEXT.to_string(),
                event_type: "progress".to_string(),
                delta: None,
                error_code: None,
                message: Some("context compilation ready".to_string()),
                recoverable: None,
                meta: Some(json!({
                    "contextCompilationSnapshot": context_compilation_snapshot.clone(),
                    "taskContract": task_contract,
                })),
            },
        );
        let context_snapshot_id = self
            .record_context_snapshot(
                &input.project_root,
                request_id,
                canonical_task,
                input.chapter_id.as_deref(),
                &context_compilation_snapshot,
            )
            .ok();
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_COMPILE_CONTEXT,
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
                    "taskContract": task_contract,
                })),
            },
        );
        self.update_run_ledger_route(
            &input.project_root,
            request_id,
            &route.provider_id,
            &route.model_id,
        );
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_ROUTE,
            error: err,
        })?;

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_COMPOSE_PROMPT);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_COMPOSE_PROMPT.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("composing prompt".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let prompt = self
            .resolve_or_build_prompt(skill_registry, &context, canonical_task, input)
            .map_err(|err| StageError {
                phase: PHASE_COMPOSE_PROMPT,
                error: err,
            })?;
        let constitution_rules_text = constitution_service
            .collect_rules_for_prompt(&input.project_root)
            .unwrap_or_default();
        let state_snapshot_text = input
            .chapter_id
            .as_deref()
            .map(|ch_id| {
                state_tracker_service
                    .collect_state_for_prompt(&input.project_root, ch_id)
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let capability_pack_service = CapabilityPackService;
        let resolved_pack = capability_pack_service.resolve_pack(&task_contract, &context);
        let capability_pack_text = capability_pack_service.format_for_prompt(&resolved_pack);
        let compiled_prompt = Self::compose_compiled_prompt(
            canonical_task,
            &task_contract,
            &context_compilation_snapshot,
            &prompt,
            &constitution_rules_text,
            &state_snapshot_text,
            &capability_pack_text,
        );
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_COMPOSE_PROMPT,
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
                    "capabilityPack": task_contract.capability_pack,
                    "authorityLayer": task_contract.authority_layer,
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
            system_prompt: Some(compiled_prompt),
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

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_REVIEW);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_REVIEW.to_string(),
                event_type: "start".to_string(),
                delta: None,
                error_code: None,
                message: Some("review checklist evaluating".to_string()),
                recoverable: None,
                meta: None,
            },
        );
        let review_checklist =
            Self::build_review_checklist(&input.project_root, constitution_service, &context, canonical_task, &task_contract, &normalized);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_REVIEW.to_string(),
                event_type: "progress".to_string(),
                delta: None,
                error_code: None,
                message: Some("review checklist ready".to_string()),
                recoverable: None,
                meta: Some(json!({
                    "taskContract": task_contract,
                    "contextCompilationSnapshot": context_compilation_snapshot.clone(),
                    "reviewChecklist": review_checklist.items.clone(),
                    "reviewSummary": {
                        "requiresHumanReview": review_checklist.requires_human_review,
                        "attentionCount": review_checklist.items.iter().filter(|item| item.status == "attention").count(),
                    }
                })),
            },
        );
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_REVIEW,
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
                meta: Some(json!({
                    "taskContract": task_contract,
                    "contextCompilationSnapshot": context_compilation_snapshot.clone(),
                    "reviewChecklist": review_checklist.items.clone(),
                    "reviewGate": task_contract.review_gate,
                })),
            },
        );
        self.check_cancelled(request_id).map_err(|err| StageError {
            phase: PHASE_PERSIST,
            error: err,
        })?;

        let persisted_records = if input.auto_persist {
            self.persist_task_output(
                canonical_task,
                &input.project_root,
                input,
                &normalized,
                request_id,
            )
            .map_err(|err| StageError {
                phase: PHASE_PERSIST,
                error: err,
            })?
        } else {
            Vec::new()
        };

        if !persisted_records.is_empty() {
            self.emit_event(
                app_handle,
                AiPipelineEvent {
                    request_id: request_id.to_string(),
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

        let checkpoint_record = match self.record_story_checkpoint(
            &input.project_root,
            request_id,
            canonical_task,
            input.chapter_id.as_deref(),
            &task_contract,
            &context_compilation_snapshot,
            &review_checklist,
            input.auto_persist,
            &persisted_records,
        ) {
            Ok(record) => {
                self.emit_event(
                    app_handle,
                    AiPipelineEvent {
                        request_id: request_id.to_string(),
                        phase: PHASE_PERSIST.to_string(),
                        event_type: "progress".to_string(),
                        delta: None,
                        error_code: None,
                        message: Some("story checkpoint recorded".to_string()),
                        recoverable: None,
                        meta: Some(json!({
                            "taskContract": task_contract,
                            "contextCompilationSnapshot": context_compilation_snapshot.clone(),
                            "reviewChecklist": review_checklist.items.clone(),
                            "reviewWorkItems": record.review_work_items.clone(),
                            "checkpointId": record.checkpoint_id.clone(),
                            "storyCheckpointId": record.checkpoint_id,
                            "reviewQueueCount": record.review_queue_count,
                            "checkpointStatus": record.status,
                        })),
                    },
                );
                Some(record)
            }
            Err(err) => {
                self.emit_event(
                    app_handle,
                    AiPipelineEvent {
                        request_id: request_id.to_string(),
                        phase: PHASE_PERSIST.to_string(),
                        event_type: "progress".to_string(),
                        delta: None,
                        error_code: Some(err.code.clone()),
                        message: Some(format!("story checkpoint skipped: {}", err.message)),
                        recoverable: Some(true),
                        meta: None,
                    },
                );
                None
            }
        };
        self.update_run_ledger_result(
            &input.project_root,
            request_id,
            normalized.chars().count() as i64,
            &persisted_records,
            review_checklist
                .items
                .iter()
                .filter(|item| item.status == "attention")
                .count() as i64,
            review_checklist.requires_human_review,
            checkpoint_record
                .as_ref()
                .map(|record| record.checkpoint_id.as_str()),
        );

        self.touch_pipeline_phase(&input.project_root, request_id, PHASE_CHECKPOINT);
        self.emit_event(
            app_handle,
            AiPipelineEvent {
                request_id: request_id.to_string(),
                phase: PHASE_CHECKPOINT.to_string(),
                event_type: "progress".to_string(),
                delta: None,
                error_code: None,
                message: Some("checkpoint finalized".to_string()),
                recoverable: None,
                meta: Some(json!({
                    "taskContract": task_contract,
                    "contextCompilationSnapshot": context_compilation_snapshot.clone(),
                    "reviewChecklist": review_checklist.items.clone(),
                    "reviewWorkItems": checkpoint_record.as_ref().map(|record| record.review_work_items.clone()).unwrap_or_default(),
                    "checkpointId": checkpoint_record.as_ref().map(|record| record.checkpoint_id.clone()),
                })),
            },
        );

        // Auto-create state snapshot for chapter tasks after successful persist
        if matches!(
            canonical_task,
            "chapter.draft" | "chapter.continue" | "chapter.rewrite"
        ) {
            if let Some(ch_id) = input.chapter_id.as_deref() {
                let snapshot_input = crate::services::state_tracker_service::CreateSnapshotInput {
                    chapter_id: ch_id.to_string(),
                    snapshot_type: Some("post_chapter".to_string()),
                    notes: Some(format!("Pipeline 自动创建 ({})", canonical_task)),
                    character_states: context
                        .related_context
                        .characters
                        .iter()
                        .map(|c| {
                            crate::services::state_tracker_service::CreateCharacterStateInput {
                                character_id: c.id.clone(),
                                location: None,
                                emotional_state: None,
                                arc_progress: None,
                                knowledge_gained: None,
                                relationships_changed: None,
                                status_notes: Some(format!("参与章节 {} 任务 {}", ch_id, canonical_task)),
                            }
                        })
                        .collect(),
                    plot_states: context
                        .related_context
                        .plot_nodes
                        .iter()
                        .map(|p| {
                            crate::services::state_tracker_service::CreatePlotStateInput {
                                plot_node_id: Some(p.id.clone()),
                                progress_status: "in_progress".to_string(),
                                tension_level: None,
                                open_threads: p.conflict.clone(),
                            }
                        })
                        .collect(),
                    world_states: vec![],
                };
                let _ = state_tracker_service.create_snapshot(&input.project_root, snapshot_input);
            }
        }

        Ok(PipelineSuccess {
            output_text: normalized,
            route,
            persisted_records,
            story_checkpoint_id: checkpoint_record
                .as_ref()
                .map(|record| record.checkpoint_id.clone()),
            context_snapshot_id,
            review_queue_count: checkpoint_record
                .as_ref()
                .map(|record| record.review_queue_count)
                .unwrap_or(0),
            context_compilation_snapshot,
            review_checklist: review_checklist.items.clone(),
            review_work_items: checkpoint_record
                .as_ref()
                .map(|record| record.review_work_items.clone())
                .unwrap_or_default(),
        })
    }

    fn build_context_manifest(
        context: &CollectedContext,
        task_type: &str,
        contract: &task_routing::TaskExecutionContract,
    ) -> Value {
        let global = &context.global_context;
        let related = &context.related_context;
        let strategy = match contract.authority_layer {
            task_routing::StoryAuthorityLayer::StoryConstitution => "constitution_first",
            task_routing::StoryAuthorityLayer::FormalAssets => "asset_grounded",
            task_routing::StoryAuthorityLayer::SceneExecution => "scene_focused",
            task_routing::StoryAuthorityLayer::ReviewAudit => "audit_focused",
            task_routing::StoryAuthorityLayer::Custom => "default",
        };
        let blueprint_completed = global
            .blueprint_summary
            .iter()
            .filter(|step| step.status == "completed")
            .count();
        let source_counts = json!({
            "blueprintCompleted": blueprint_completed,
            "lockedTerms": global.locked_terms.len(),
            "bannedTerms": global.banned_terms.len(),
            "chapterBound": related.chapter.is_some(),
            "characters": related.characters.len(),
            "worldRules": related.world_rules.len(),
            "plotNodes": related.plot_nodes.len(),
            "relationshipEdges": related.relationship_edges.len(),
            "hasPreviousSummary": related.previous_chapter_summary.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false),
        });
        let priorities = vec![
            "story_constitution",
            "formal_assets",
            "dynamic_scene_state",
            "task_instruction",
        ];
        let estimated_tokens = 320
            + (blueprint_completed as i64 * 90)
            + (related.characters.len() as i64 * 55)
            + (related.world_rules.len() as i64 * 65)
            + (related.plot_nodes.len() as i64 * 45)
            + (related.relationship_edges.len() as i64 * 40);
        let token_budget = json!({
            "hardLimit": 12000,
            "reservedForGeneration": 2600,
            "estimatedContextTokens": estimated_tokens,
            "strategy": "budgeted_trim",
        });
        json!({
            "taskType": task_type,
            "compileStrategy": strategy,
            "project": {
                "name": global.project_name,
                "genre": global.genre,
                "narrativePov": global.narrative_pov,
            },
            "sources": source_counts,
            "trimming": {
                "policy": "priority_trim",
                "blueprintStepContentMaxChars": 800,
                "ruleMaxChars": 300,
                "chapterSummaryMaxChars": 1200,
            },
            "priority": priorities,
            "conflictResolution": {
                "mode": "constitution_then_assets_then_scene",
                "ifConflict": "raise_review_item",
            },
            "tokenBudget": token_budget,
            "sourceDigests": {
                "lockedTermsPreview": global.locked_terms.iter().take(6).collect::<Vec<_>>(),
                "bannedTermsPreview": global.banned_terms.iter().take(6).collect::<Vec<_>>(),
            }
        })
    }

    fn record_context_snapshot(
        &self,
        project_root: &str,
        run_id: &str,
        task_type: &str,
        chapter_id: Option<&str>,
        snapshot: &Value,
    ) -> Result<String, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        let snapshot_id = Uuid::new_v4().to_string();
        let compile_strategy = snapshot
            .get("compileStrategy")
            .and_then(|value| value.as_str())
            .unwrap_or("default");
        let sources_json = snapshot
            .get("sources")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .to_string();
        let trimming_json = snapshot
            .get("trimming")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .to_string();
        let priority_json = snapshot
            .get("priority")
            .cloned()
            .unwrap_or_else(|| json!([]))
            .to_string();
        let conflict_json = snapshot
            .get("conflictResolution")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .to_string();
        let token_budget_json = snapshot
            .get("tokenBudget")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .to_string();

        conn.execute(
            "INSERT INTO story_os_v2_context_snapshots(
                id, run_id, project_id, chapter_id, task_type, compile_strategy, sources_json,
                trimming_json, priority_json, conflict_resolution_json, token_budget_json,
                compiled_manifest_json, created_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                &snapshot_id,
                run_id,
                &project_id,
                chapter_id.map(str::trim).filter(|v| !v.is_empty()),
                task_type,
                compile_strategy,
                &sources_json,
                &trimming_json,
                &priority_json,
                &conflict_json,
                &token_budget_json,
                snapshot.to_string(),
                &now
            ],
        )
        .map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "写入上下文编译快照失败", true)
                .with_detail(err.to_string())
        })?;

        Ok(snapshot_id)
    }

    fn compose_compiled_prompt(
        task_type: &str,
        contract: &task_routing::TaskExecutionContract,
        context_manifest: &Value,
        raw_prompt: &str,
        constitution_rules_text: &str,
        state_snapshot_text: &str,
        capability_pack_text: &str,
    ) -> String {
        let mut lines = Vec::new();
        lines.push("# NovelForge 生产系统协议".to_string());
        lines.push(
            "你在长篇小说生产操作系统内执行任务，不追求字数最大化，追求正确层级推进。".to_string(),
        );
        lines.push(format!("- taskType: {}", task_type));
        lines.push(format!(
            "- authorityLayer: {}",
            Self::authority_layer_key(contract.authority_layer)
        ));
        lines.push(format!(
            "- stateLayer: {}",
            Self::state_layer_key(contract.state_layer)
        ));
        lines.push(format!("- capabilityPack: {}", contract.capability_pack));
        lines.push(format!(
            "- reviewGate: {}",
            Self::review_gate_key(contract.review_gate)
        ));
        lines.push("- 规则：优先遵守故事宪法、再遵守正式资产、最后才扩展动态状态。".to_string());
        lines.push(String::new());
        if !constitution_rules_text.is_empty() {
            lines.push(constitution_rules_text.to_string());
            lines.push(String::new());
        }
        if !state_snapshot_text.is_empty() {
            lines.push(state_snapshot_text.to_string());
            lines.push(String::new());
        }
        if !capability_pack_text.is_empty() {
            lines.push(capability_pack_text.to_string());
            lines.push(String::new());
        }
        lines.push("# 上下文编译清单(JSON)".to_string());
        lines.push(context_manifest.to_string());
        lines.push(String::new());
        lines.push("# 任务提示".to_string());
        lines.push(raw_prompt.to_string());
        lines.join("\n")
    }

    fn build_review_checklist(
        project_root: &str,
        constitution_service: &ConstitutionService,
        context: &CollectedContext,
        task_type: &str,
        contract: &task_routing::TaskExecutionContract,
        output: &str,
    ) -> ReviewChecklist {
        let mut items = Vec::new();
        let normalized = output.trim();
        let output_len = normalized.chars().count();

        let banned_hits: Vec<String> = context
            .global_context
            .banned_terms
            .iter()
            .filter(|term| !term.trim().is_empty() && normalized.contains(term.as_str()))
            .cloned()
            .collect();
        if banned_hits.is_empty() {
            items.push(ReviewChecklistItem {
                key: "banned_terms".to_string(),
                title: "全局禁用词检查".to_string(),
                severity: "high".to_string(),
                status: "pass".to_string(),
                message: "未检测到全局禁用词".to_string(),
            });
        } else {
            items.push(ReviewChecklistItem {
                key: "banned_terms".to_string(),
                title: "全局禁用词检查".to_string(),
                severity: "high".to_string(),
                status: "attention".to_string(),
                message: format!("检测到全局禁用词：{}", banned_hits.join("、")),
            });
        }

        if let Ok(validation) = constitution_service.validate_text(project_root, normalized, None, None) {
            if validation.violations_found > 0 {
                for (idx, violation) in validation.violations.into_iter().enumerate() {
                    items.push(ReviewChecklistItem {
                        key: format!("constitution_violation_{}", idx),
                        title: "故事宪法违规".to_string(),
                        severity: if violation.severity == "blocker" { "critical".to_string() } else { "high".to_string() },
                        status: "attention".to_string(),
                        message: violation.violation_text,
                    });
                }
            } else {
                items.push(ReviewChecklistItem {
                    key: "constitution_check".to_string(),
                    title: "故事宪法检查".to_string(),
                    severity: "high".to_string(),
                    status: "pass".to_string(),
                    message: format!("已通过 {} 项活动宪法规则检查", validation.total_rules_checked),
                });
            }
        }

        let min_len = if matches!(
            contract.authority_layer,
            task_routing::StoryAuthorityLayer::SceneExecution
        ) {
            180
        } else {
            80
        };
        if output_len < min_len {
            items.push(ReviewChecklistItem {
                key: "output_length".to_string(),
                title: "产出完整度".to_string(),
                severity: "medium".to_string(),
                status: "attention".to_string(),
                message: format!("产出长度偏短（{} 字，建议至少 {} 字）", output_len, min_len),
            });
        } else {
            items.push(ReviewChecklistItem {
                key: "output_length".to_string(),
                title: "产出完整度".to_string(),
                severity: "low".to_string(),
                status: "pass".to_string(),
                message: format!("产出长度 {} 字", output_len),
            });
        }

        if matches!(
            task_type,
            "chapter.draft" | "chapter.continue" | "chapter.rewrite"
        ) {
            let has_dialogue = normalized.contains('“') || normalized.contains('"');
            let has_action = normalized.contains('。') || normalized.contains('！');
            let status = if has_dialogue && has_action {
                "pass"
            } else {
                "attention"
            };
            items.push(ReviewChecklistItem {
                key: "scene_texture".to_string(),
                title: "场景推进纹理".to_string(),
                severity: "medium".to_string(),
                status: status.to_string(),
                message: if status == "pass" {
                    "含对白/动作纹理，具备可精修基础".to_string()
                } else {
                    "对白或动作纹理不足，建议人工精修场景推进".to_string()
                },
            });
        }

        let requires_human_review = matches!(
            contract.review_gate,
            task_routing::ReviewGateMode::ManualRequired
        );
        ReviewChecklist {
            items,
            requires_human_review,
        }
    }

    fn record_story_checkpoint(
        &self,
        project_root: &str,
        run_id: &str,
        task_type: &str,
        chapter_id: Option<&str>,
        contract: &task_routing::TaskExecutionContract,
        context_manifest: &Value,
        review_checklist: &ReviewChecklist,
        auto_persist: bool,
        persisted_records: &[PersistedRecord],
    ) -> Result<StoryCheckpointRecord, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let mut conn = open_database(project_root_path).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        let checkpoint_id = Uuid::new_v4().to_string();
        let checklist_json =
            serde_json::to_string(&review_checklist.items).unwrap_or("[]".to_string());
        let persisted_records_json =
            serde_json::to_string(persisted_records).unwrap_or("[]".to_string());
        let context_manifest_json = context_manifest.to_string();
        let status = if review_checklist.requires_human_review {
            if auto_persist {
                "persisted_pending_review"
            } else {
                "awaiting_manual_review"
            }
        } else {
            "persisted"
        };

        let tx = conn.transaction().map_err(|e| {
            AppErrorDto::new("DB_WRITE_FAILED", "写入审查检查点失败", true)
                .with_detail(e.to_string())
        })?;
        tx.execute(
            "INSERT INTO ai_story_checkpoints(
                id, run_id, project_id, chapter_id, task_type, authority_layer, state_layer,
                capability_pack, review_gate, context_manifest_json, review_checklist_json,
                persisted_records_json, persistence_mode, status, created_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                &checkpoint_id,
                run_id,
                &project_id,
                chapter_id.map(str::trim).filter(|v| !v.is_empty()),
                task_type,
                Self::authority_layer_key(contract.authority_layer),
                Self::state_layer_key(contract.state_layer),
                contract.capability_pack,
                Self::review_gate_key(contract.review_gate),
                &context_manifest_json,
                &checklist_json,
                &persisted_records_json,
                if auto_persist {
                    "auto_persist"
                } else {
                    "manual_persist"
                },
                status,
                &now
            ],
        )
        .map_err(|e| {
            AppErrorDto::new("DB_WRITE_FAILED", "写入审查检查点失败", true)
                .with_detail(e.to_string())
        })?;

        let mut review_queue_count = 0usize;
        let mut review_work_items = Vec::new();
        for item in &review_checklist.items {
            if item.status != "attention" {
                continue;
            }
            let review_item_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO story_os_v2_review_work_items(
                    id, run_id, checkpoint_id, project_id, chapter_id, task_type, checklist_key,
                    title, severity, message, status, created_at, updated_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', ?11, ?11)",
                params![
                    &review_item_id,
                    run_id,
                    &checkpoint_id,
                    &project_id,
                    chapter_id.map(str::trim).filter(|v| !v.is_empty()),
                    task_type,
                    &item.key,
                    &item.title,
                    &item.severity,
                    &item.message,
                    &now
                ],
            )
            .map_err(|e| {
                AppErrorDto::new("DB_WRITE_FAILED", "写入 v2 审查工单失败", true)
                    .with_detail(e.to_string())
            })?;
            review_work_items.push(ReviewWorkItemBrief {
                id: review_item_id,
                key: item.key.clone(),
                title: item.title.clone(),
                severity: item.severity.clone(),
                message: item.message.clone(),
                status: "pending".to_string(),
            });
            review_queue_count += 1;
        }
        tx.execute(
            "UPDATE story_os_v2_run_ledger
             SET checkpoint_id = ?1, updated_at = ?2
             WHERE id = ?3",
            params![&checkpoint_id, &now, run_id],
        )
        .map_err(|e| {
            AppErrorDto::new("DB_WRITE_FAILED", "回写 v2 运行台账 checkpoint 失败", true)
                .with_detail(e.to_string())
        })?;
        tx.commit().map_err(|e| {
            AppErrorDto::new("DB_WRITE_FAILED", "提交审查检查点失败", true)
                .with_detail(e.to_string())
        })?;

        Ok(StoryCheckpointRecord {
            checkpoint_id,
            review_queue_count,
            status: status.to_string(),
            review_work_items,
        })
    }

    fn authority_layer_key(layer: task_routing::StoryAuthorityLayer) -> &'static str {
        match layer {
            task_routing::StoryAuthorityLayer::StoryConstitution => "story_constitution",
            task_routing::StoryAuthorityLayer::FormalAssets => "formal_assets",
            task_routing::StoryAuthorityLayer::SceneExecution => "scene_execution",
            task_routing::StoryAuthorityLayer::ReviewAudit => "review_audit",
            task_routing::StoryAuthorityLayer::Custom => "custom",
        }
    }

    fn state_layer_key(layer: task_routing::StoryStateLayer) -> &'static str {
        match layer {
            task_routing::StoryStateLayer::Constitution => "constitution_state",
            task_routing::StoryStateLayer::Asset => "asset_state",
            task_routing::StoryStateLayer::DynamicScene => "dynamic_scene_state",
            task_routing::StoryStateLayer::Review => "review_state",
            task_routing::StoryStateLayer::Custom => "custom_state",
        }
    }

    fn review_gate_key(mode: task_routing::ReviewGateMode) -> &'static str {
        match mode {
            task_routing::ReviewGateMode::ManualRequired => "manual_required",
            task_routing::ReviewGateMode::ManualRecommended => "manual_recommended",
        }
    }

    fn persist_task_output(
        &self,
        canonical_task: &str,
        project_root: &str,
        input: &RunAiTaskPipelineInput,
        normalized_output: &str,
        request_id: &str,
    ) -> Result<Vec<PersistedRecord>, AppErrorDto> {
        let mut records = Vec::new();
        match canonical_task {
            "character.create" => {
                let create_input = Self::build_character_create_input(
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = CharacterService.create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "character".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            "world.create_rule" => {
                let create_input = Self::build_world_rule_create_input(
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = WorldService.create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "world_rule".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            "plot.create_node" => {
                let create_input = Self::build_plot_node_create_input(
                    project_root,
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = PlotService.create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "plot_node".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            "blueprint.generate_step" => {
                let step_key = input
                    .blueprint_step_key
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        AppErrorDto::new(
                            "PIPELINE_BLUEPRINT_STEP_REQUIRED",
                            "蓝图持久化缺少 stepKey",
                            true,
                        )
                    })?;
                let saved = BlueprintService.save_step(
                    project_root,
                    SaveBlueprintStepInput {
                        step_key: step_key.to_string(),
                        content: Self::normalize_blueprint_content(normalized_output),
                        ai_generated: Some(true),
                    },
                )?;
                records.push(PersistedRecord {
                    entity_type: "blueprint_step".to_string(),
                    entity_id: saved.id,
                    action: "updated".to_string(),
                });
            }
            "consistency.scan" => {
                let chapter_id = input
                    .chapter_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        AppErrorDto::new(
                            "PIPELINE_CHAPTER_ID_REQUIRED",
                            "一致性持久化缺少 chapterId",
                            true,
                        )
                    })?;
                let batch_size = self.persist_ai_consistency_issues(
                    project_root,
                    chapter_id,
                    normalized_output,
                )?;
                records.push(PersistedRecord {
                    entity_type: "consistency_issue_batch".to_string(),
                    entity_id: format!("{}:{}", chapter_id, request_id),
                    action: format!("inserted:{}", batch_size),
                });
            }
            "glossary.create_term" => {
                let create_input = Self::build_glossary_term_create_input(
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = GlossaryService.create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "glossary_term".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            "narrative.create_obligation" => {
                let create_input = Self::build_narrative_obligation_create_input(
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = NarrativeService.create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "narrative_obligation".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            _ => {}
        }
        Ok(records)
    }

    fn build_character_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateCharacterInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("character"))?;
        let name = Self::pick_string(
            &root,
            &["name", "characterName", "角色名", "title"],
            Some("未命名角色"),
        );
        let role_type = Self::pick_string(
            &root,
            &["roleType", "role_type", "type", "角色类型"],
            Some("配角"),
        );
        let aliases = Self::pick_string_array(&root, &["aliases", "alias", "别名"]);
        Ok(CreateCharacterInput {
            name,
            aliases: if aliases.is_empty() {
                None
            } else {
                Some(aliases)
            },
            role_type,
            age: None,
            gender: None,
            identity_text: Self::pick_optional_string(
                &root,
                &["identityText", "identity_text", "identity", "身份"],
            ),
            appearance: Self::pick_optional_string(&root, &["appearance", "looks", "外貌"]),
            motivation: Self::pick_optional_string(&root, &["motivation", "核心动机", "drive"]),
            desire: Self::pick_optional_string(&root, &["desire", "欲望"]),
            fear: Self::pick_optional_string(&root, &["fear", "恐惧"]),
            flaw: Self::pick_optional_string(&root, &["flaw", "缺陷"]),
            arc_stage: Self::pick_optional_string(&root, &["arcStage", "arc_stage", "成长弧线"]),
            locked_fields: None,
            notes: Self::pick_optional_string(&root, &["notes", "remark", "备注"]).or_else(|| {
                (!fallback_instruction.trim().is_empty()).then(|| fallback_instruction.to_string())
            }),
        })
    }

    fn build_world_rule_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateWorldRuleInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("worldRule"))?;
        let title = Self::pick_string(&root, &["title", "name", "设定名"], Some("未命名设定"));
        let category = Self::pick_string(&root, &["category", "type", "类别"], Some("世界规则"));
        let description = Self::pick_string(
            &root,
            &["description", "summary", "desc", "描述"],
            Some(fallback_instruction),
        );
        let constraint_level = Self::normalize_constraint_level(
            Self::pick_optional_string(
                &root,
                &[
                    "constraintLevel",
                    "constraint_level",
                    "strictness",
                    "约束等级",
                ],
            )
            .as_deref(),
        );
        let related_entities =
            Self::pick_string_array(&root, &["relatedEntities", "related_entities", "entities"]);
        Ok(CreateWorldRuleInput {
            title,
            category,
            description,
            constraint_level,
            related_entities: if related_entities.is_empty() {
                None
            } else {
                Some(related_entities)
            },
            examples: Self::pick_optional_string(&root, &["examples", "示例"]),
            contradiction_policy: Self::pick_optional_string(
                &root,
                &["contradictionPolicy", "contradiction_policy", "冲突策略"],
            ),
        })
    }

    fn build_plot_node_create_input(
        project_root: &str,
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreatePlotNodeInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("plotNode"))?;
        let sort_order = PlotService.next_sort_order(project_root)?;
        Ok(CreatePlotNodeInput {
            title: Self::pick_string(&root, &["title", "name", "节点标题"], Some("未命名节点")),
            node_type: Self::pick_string(
                &root,
                &["nodeType", "node_type", "type", "节点类型"],
                Some("开端"),
            ),
            sort_order,
            goal: Self::pick_optional_string(&root, &["goal", "objective", "目标"]).or_else(|| {
                (!fallback_instruction.trim().is_empty()).then(|| fallback_instruction.to_string())
            }),
            conflict: Self::pick_optional_string(&root, &["conflict", "冲突"]),
            emotional_curve: Self::pick_optional_string(
                &root,
                &["emotionalCurve", "emotional_curve", "情绪曲线"],
            ),
            status: Self::pick_optional_string(&root, &["status", "状态"]),
            related_characters: {
                let related =
                    Self::pick_string_array(&root, &["relatedCharacters", "related_characters"]);
                if related.is_empty() {
                    None
                } else {
                    Some(related)
                }
            },
        })
    }

    fn build_glossary_term_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateGlossaryTermInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("glossaryTerm"))?;
        let term = Self::pick_string(&root, &["term", "name", "词条"], Some("未命名名词"));
        let term_type = Self::pick_string(
            &root,
            &["termType", "term_type", "type", "类型"],
            Some("术语"),
        );
        let aliases = Self::pick_string_array(&root, &["aliases", "alias", "别名"]);
        Ok(CreateGlossaryTermInput {
            term,
            term_type,
            aliases: if aliases.is_empty() {
                None
            } else {
                Some(aliases)
            },
            description: Self::pick_optional_string(
                &root,
                &["description", "summary", "desc", "描述"],
            )
            .or_else(|| {
                (!fallback_instruction.trim().is_empty()).then(|| fallback_instruction.to_string())
            }),
            locked: Some(Self::pick_bool(&root, &["locked"], false)),
            banned: Some(Self::pick_bool(&root, &["banned"], false)),
        })
    }

    fn build_narrative_obligation_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateObligationInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("obligation"))?;
        let related_entities =
            Self::pick_string_array(&root, &["relatedEntities", "related_entities", "entities"]);
        Ok(CreateObligationInput {
            obligation_type: Self::pick_string(
                &root,
                &["obligationType", "obligation_type", "type"],
                Some("foreshadowing"),
            ),
            description: Self::pick_string(
                &root,
                &["description", "summary", "desc"],
                Some(fallback_instruction),
            ),
            planted_chapter_id: Self::pick_optional_string(
                &root,
                &["plantedChapterId", "planted_chapter_id"],
            ),
            expected_payoff_chapter_id: Self::pick_optional_string(
                &root,
                &["expectedPayoffChapterId", "expected_payoff_chapter_id"],
            ),
            actual_payoff_chapter_id: Self::pick_optional_string(
                &root,
                &["actualPayoffChapterId", "actual_payoff_chapter_id"],
            ),
            payoff_status: Self::pick_optional_string(
                &root,
                &["payoffStatus", "payoff_status", "status"],
            ),
            severity: Self::pick_optional_string(&root, &["severity", "priority"]),
            related_entities: if related_entities.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&related_entities).unwrap_or_default())
            },
        })
    }

    fn persist_ai_consistency_issues(
        &self,
        project_root: &str,
        chapter_id: &str,
        normalized_output: &str,
    ) -> Result<usize, AppErrorDto> {
        use crate::services::consistency_service::{AiConsistencyIssueInput, ConsistencyService};

        let value = Self::extract_output_value(normalized_output)?;
        let issues_raw = value
            .get("issues")
            .and_then(|item| item.as_array())
            .cloned()
            .or_else(|| value.as_array().cloned())
            .unwrap_or_default();

        let mut parsed_issues = Vec::new();
        for issue in issues_raw {
            let issue_obj = match issue.as_object() {
                Some(obj) => obj,
                None => continue,
            };
            let explanation = Self::pick_string(issue_obj, &["explanation", "message"], Some(""));
            if explanation.trim().is_empty() {
                continue;
            }
            parsed_issues.push(AiConsistencyIssueInput {
                issue_type: Self::pick_string(
                    issue_obj,
                    &["issueType", "issue_type", "type"],
                    Some("prose_style"),
                ),
                severity: Self::normalize_consistency_severity(Self::pick_optional_string(
                    issue_obj,
                    &["severity", "level"],
                )),
                source_text: Self::pick_string(
                    issue_obj,
                    &["sourceText", "source_text", "snippet"],
                    Some(""),
                ),
                explanation,
                suggested_fix: Self::pick_optional_string(
                    issue_obj,
                    &["suggestedFix", "suggested_fix", "fix"],
                ),
            });
        }

        ConsistencyService.persist_ai_issues(project_root, chapter_id, parsed_issues)
    }

    fn extract_output_value(normalized_output: &str) -> Result<Value, AppErrorDto> {
        if let Ok(value) = serde_json::from_str::<Value>(normalized_output) {
            return Ok(value);
        }

        let brace_start = normalized_output.find('{');
        let brace_end = normalized_output.rfind('}');
        if let (Some(start), Some(end)) = (brace_start, brace_end) {
            if end > start {
                let json_text = &normalized_output[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(json_text) {
                    return Ok(value);
                }
            }
        }

        let bracket_start = normalized_output.find('[');
        let bracket_end = normalized_output.rfind(']');
        if let (Some(start), Some(end)) = (bracket_start, bracket_end) {
            if end > start {
                let json_text = &normalized_output[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(json_text) {
                    return Ok(value);
                }
            }
        }

        Err(AppErrorDto::new(
            "PIPELINE_PERSIST_PARSE_FAILED",
            "AI 返回结果无法解析为 JSON",
            true,
        ))
    }

    fn extract_output_object(
        normalized_output: &str,
        nested_key: Option<&str>,
    ) -> Result<serde_json::Map<String, Value>, AppErrorDto> {
        let value = Self::extract_output_value(normalized_output)?;
        let root_value = if let Some(key) = nested_key {
            value.get(key).cloned().unwrap_or(value)
        } else {
            value
        };
        root_value.as_object().cloned().ok_or_else(|| {
            AppErrorDto::new(
                "PIPELINE_PERSIST_PARSE_FAILED",
                "AI 返回 JSON 结构不是对象",
                true,
            )
        })
    }

    fn pick_optional_string(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(value) = obj.get(*key) {
                match value {
                    Value::String(v) => {
                        let trimmed = v.trim();
                        if !trimmed.is_empty() {
                            return Some(trimmed.to_string());
                        }
                    }
                    Value::Number(v) => return Some(v.to_string()),
                    Value::Bool(v) => return Some(v.to_string()),
                    _ => {}
                }
            }
        }
        None
    }

    fn pick_string(
        obj: &serde_json::Map<String, Value>,
        keys: &[&str],
        fallback: Option<&str>,
    ) -> String {
        Self::pick_optional_string(obj, keys)
            .or_else(|| fallback.map(str::to_string))
            .unwrap_or_default()
    }

    fn pick_bool(obj: &serde_json::Map<String, Value>, keys: &[&str], fallback: bool) -> bool {
        for key in keys {
            if let Some(value) = obj.get(*key) {
                match value {
                    Value::Bool(v) => return *v,
                    Value::Number(v) => return v.as_i64().unwrap_or(0) != 0,
                    Value::String(v) => {
                        let normalized = v.trim().to_ascii_lowercase();
                        if matches!(normalized.as_str(), "true" | "1" | "yes" | "是") {
                            return true;
                        }
                        if matches!(normalized.as_str(), "false" | "0" | "no" | "否") {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
        }
        fallback
    }

    fn pick_string_array(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Vec<String> {
        for key in keys {
            if let Some(value) = obj.get(*key) {
                match value {
                    Value::Array(values) => {
                        let list = values
                            .iter()
                            .filter_map(|item| item.as_str())
                            .map(str::trim)
                            .filter(|item| !item.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>();
                        if !list.is_empty() {
                            return list;
                        }
                    }
                    Value::String(v) => {
                        let list = v
                            .split(&[',', '，', '、'][..])
                            .map(str::trim)
                            .filter(|item| !item.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>();
                        if !list.is_empty() {
                            return list;
                        }
                    }
                    _ => {}
                }
            }
        }
        Vec::new()
    }

    fn normalize_constraint_level(raw: Option<&str>) -> String {
        let value = raw.unwrap_or("").trim().to_ascii_lowercase();
        if value.contains("weak") || value.contains("low") || value.contains("弱") {
            return "weak".to_string();
        }
        if value.contains("absolute")
            || value.contains("must")
            || value.contains("不可")
            || value.contains("绝对")
        {
            return "absolute".to_string();
        }
        if value.contains("strong") || value.contains("high") || value.contains("强") {
            return "strong".to_string();
        }
        "normal".to_string()
    }

    fn normalize_consistency_severity(raw: Option<String>) -> String {
        let value = raw
            .unwrap_or_else(|| "medium".to_string())
            .trim()
            .to_ascii_lowercase();
        if matches!(
            value.as_str(),
            "blocker" | "high" | "medium" | "low" | "info"
        ) {
            value
        } else {
            "medium".to_string()
        }
    }

    fn normalize_blueprint_content(normalized_output: &str) -> String {
        if let Ok(value) = Self::extract_output_value(normalized_output) {
            if value.is_object() {
                if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                    return pretty;
                }
            }
        }
        normalized_output.to_string()
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
            "chapter.draft" | "chapter.continue" if chapter_id.is_empty() => {
                return Err(AppErrorDto::new(
                    "PIPELINE_CHAPTER_ID_REQUIRED",
                    "该任务需要 chapterId",
                    true,
                ));
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
            | "narrative.create_obligation" if user_instruction.is_empty() => {
                return Err(AppErrorDto::new(
                    "PIPELINE_USER_INSTRUCTION_REQUIRED",
                    "该任务需要 userInstruction",
                    true,
                ));
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
            "prose.naturalize" => PromptBuilder::build_naturalize(selected_text),
            "chapter.plan" => PromptBuilder::build_chapter_plan(context, user_instruction),
            "character.create" => PromptBuilder::build_character_create(context, user_instruction),
            "world.create_rule" => {
                PromptBuilder::build_world_create_rule(context, user_instruction)
            }
            "plot.create_node" => PromptBuilder::build_plot_create_node(context, user_instruction),
            "glossary.create_term" => {
                PromptBuilder::build_glossary_create_term(context, user_instruction)
            }
            "narrative.create_obligation" => {
                PromptBuilder::build_narrative_create_obligation(context, user_instruction)
            }
            "consistency.scan" => {
                let chapter_content = input.chapter_content.as_deref().unwrap_or("");
                PromptBuilder::build_consistency_scan(context, chapter_content)
            }
            "blueprint.generate_step" => {
                let step_key = input.blueprint_step_key.as_deref().unwrap_or_default();
                let step_title = input.blueprint_step_title.as_deref().unwrap_or_default();
                PromptBuilder::build_blueprint_step(context, step_key, step_title, user_instruction)
            }
            "timeline.review" => PromptBuilder::build_timeline_review(context, user_instruction),
            "relationship.review" => {
                PromptBuilder::build_relationship_review(context, user_instruction)
            }
            "dashboard.review" => PromptBuilder::build_dashboard_review(context, user_instruction),
            "export.review" => PromptBuilder::build_export_review(context, user_instruction),
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
        for edge in &related.relationship_edges {
            let mut line = format!(
                "关系：{} -> {} [{}]",
                edge.source_name, edge.target_name, edge.relationship_type
            );
            if let Some(ref description) = edge.description {
                if !description.trim().is_empty() {
                    line.push_str(&format!("：{}", description.trim()));
                }
            }
            parts.push(line);
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
        task_contract: &task_routing::TaskExecutionContract,
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
        conn.execute(
            "INSERT INTO story_os_v2_run_ledger(
                id, project_id, chapter_id, task_type, ui_action, authority_layer, state_layer,
                capability_pack, review_gate, status, phase, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'running', ?10, ?11, ?11)",
            params![
                request_id,
                &project_id,
                chapter_id.map(str::trim).filter(|v| !v.is_empty()),
                task_type,
                ui_action.map(str::trim).filter(|v| !v.is_empty()),
                Self::authority_layer_key(task_contract.authority_layer),
                Self::state_layer_key(task_contract.state_layer),
                task_contract.capability_pack,
                Self::review_gate_key(task_contract.review_gate),
                PHASE_VALIDATE,
                &now
            ],
        )
        .map_err(|err| {
            AppErrorDto::new(
                "PIPELINE_AUDIT_INSERT_FAILED",
                "记录 v2 run ledger 失败",
                false,
            )
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
        let now = now_iso();
        let _ = conn.execute(
            "UPDATE story_os_v2_run_ledger
             SET status = ?1,
                 phase = ?2,
                 error_code = ?3,
                 error_message = ?4,
                 updated_at = ?5,
                 completed_at = ?6
             WHERE id = ?7",
            params![
                status,
                phase,
                error_code,
                error_message,
                &now,
                &now,
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
        let _ = conn.execute(
            "UPDATE story_os_v2_run_ledger
             SET phase = ?1, updated_at = ?2
             WHERE id = ?3
               AND status = 'running'",
            params![phase, now_iso(), request_id],
        );
    }

    fn update_run_ledger_route(
        &self,
        project_root: &str,
        request_id: &str,
        provider_id: &str,
        model_id: &str,
    ) {
        let conn = match open_database(Path::new(project_root)) {
            Ok(conn) => conn,
            Err(_) => return,
        };
        let _ = conn.execute(
            "UPDATE story_os_v2_run_ledger
             SET provider_id = ?1, model_id = ?2, updated_at = ?3
             WHERE id = ?4",
            params![provider_id, model_id, now_iso(), request_id],
        );
    }

    fn update_run_ledger_result(
        &self,
        project_root: &str,
        request_id: &str,
        output_text_length: i64,
        persisted_records: &[PersistedRecord],
        review_attention_count: i64,
        requires_human_review: bool,
        checkpoint_id: Option<&str>,
    ) {
        let conn = match open_database(Path::new(project_root)) {
            Ok(conn) => conn,
            Err(_) => return,
        };
        let persisted_records_json =
            serde_json::to_string(persisted_records).unwrap_or("[]".to_string());
        let _ = conn.execute(
            "UPDATE story_os_v2_run_ledger
             SET output_text_length = ?1,
                 persisted_records_json = ?2,
                 review_attention_count = ?3,
                 requires_human_review = ?4,
                 checkpoint_id = COALESCE(?5, checkpoint_id),
                 updated_at = ?6
             WHERE id = ?7",
            params![
                output_text_length,
                persisted_records_json,
                review_attention_count,
                if requires_human_review { 1_i64 } else { 0_i64 },
                checkpoint_id,
                now_iso(),
                request_id
            ],
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
                | "glossary.create_term"
                | "narrative.create_obligation"
                | "timeline.review"
                | "relationship.review"
                | "dashboard.review"
                | "export.review"
        )
    }

    fn generate_user_message(task_type: &str) -> &'static str {
        match task_type {
            "character.create" => "请根据用户设想生成角色卡 JSON。",
            "world.create_rule" => "请根据用户设想生成世界设定 JSON。",
            "plot.create_node" => "请根据用户设想生成剧情节点 JSON。",
            "glossary.create_term" => "请根据用户设想生成名词条目 JSON。",
            "narrative.create_obligation" => "请根据用户设想生成叙事义务 JSON。",
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
    use crate::services::task_routing;

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
            auto_persist: false,
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
    fn validate_chapter_plan_allows_project_level_planning() {
        let service = AiPipelineService::default();
        let mut input = base_input("chapter.plan");
        input.chapter_id = None;
        service
            .validate_input("chapter.plan", &input)
            .expect("chapter plan should allow missing chapter id");
    }

    #[test]
    fn validate_glossary_requires_user_instruction() {
        let service = AiPipelineService::default();
        let mut input = base_input("glossary.create_term");
        input.user_instruction = "   ".to_string();
        let err = service
            .validate_input("glossary.create_term", &input)
            .expect_err("expected error");
        assert_eq!(err.code, "PIPELINE_USER_INSTRUCTION_REQUIRED");
    }

    #[test]
    fn validate_narrative_requires_user_instruction() {
        let service = AiPipelineService::default();
        let mut input = base_input("narrative.create_obligation");
        input.user_instruction = "   ".to_string();
        let err = service
            .validate_input("narrative.create_obligation", &input)
            .expect_err("expected error");
        assert_eq!(err.code, "PIPELINE_USER_INSTRUCTION_REQUIRED");
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
        let task_contract = task_routing::task_execution_contract("chapter.draft");
        service
            .insert_pipeline_run(
                &project.project_root,
                &request_id,
                None,
                "chapter.draft",
                Some("editor.ai.chapter.draft"),
                &task_contract,
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
