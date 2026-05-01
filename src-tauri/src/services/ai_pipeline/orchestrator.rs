use std::sync::{Arc, RwLock};

use serde_json::json;

use crate::adapters::llm_types::{ContentBlock, Message, UnifiedGenerateRequest};
use crate::errors::AppErrorDto;
use crate::services::ai_pipeline::audit_store::PipelineAuditStore;
use crate::services::ai_pipeline::continuity_pack::ContinuityPackCompiler;
use crate::services::ai_pipeline::prompt_resolver::PromptResolver;
use crate::services::ai_pipeline::task_handlers::{RuntimeStateWriteOptions, TaskHandlers};
use crate::services::ai_pipeline_service::{
    AiPipelineEvent, AiPipelineService, PersistedRecord, RunAiTaskPipelineInput,
};
use crate::services::ai_service::{AiService, TaskRouteResolution};
use crate::services::blueprint_service::{
    extract_certainty_zones_from_content, BlueprintCertaintyZones,
};
use crate::services::context_service::ContextService;
use crate::services::project_service::{AiStrategyProfile, ProjectService};
use crate::services::skill_registry::{SkillRegistry, SkillSelectionContext};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PersistMode {
    None,
    Formal,
    DerivedReview,
}

impl PersistMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Formal => "formal",
            Self::DerivedReview => "derived_review",
        }
    }
}

type CertaintyZones = BlueprintCertaintyZones;

#[derive(Debug, Clone)]
struct FreezeConflict {
    matched_zone: String,
}

impl<'a> PipelineOrchestrator<'a> {
    pub async fn run(&self) -> Result<PipelineSuccess, StageError> {
        self.audit_store.touch_pipeline_phase(
            &self.input.project_root,
            self.request_id,
            PHASE_VALIDATE,
        );
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

        self.audit_store.touch_pipeline_phase(
            &self.input.project_root,
            self.request_id,
            PHASE_CONTEXT,
        );
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
        let chapter_id = self
            .input
            .chapter_id
            .as_deref()
            .map(str::trim)
            .unwrap_or("");
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
        let strategy_profile = self.load_ai_strategy_profile();
        let certainty_zones = extract_certainty_zones(&context);
        let freeze_conflict =
            detect_freeze_conflict(self.input.user_instruction.as_str(), &certainty_zones);
        if let Some(conflict) = freeze_conflict.as_ref() {
            self.pipeline_service.emit_event(
                self.app_handle,
                AiPipelineEvent {
                    request_id: self.request_id.to_string(),
                    phase: PHASE_CONTEXT.to_string(),
                    event_type: "progress".to_string(),
                    delta: None,
                    error_code: None,
                    message: Some("freeze-zone conflict detected".to_string()),
                    recoverable: Some(true),
                    meta: Some(json!({
                        "freezeConflict": conflict.matched_zone,
                        "certaintyZones": {
                            "frozen": certainty_zones.frozen,
                            "promised": certainty_zones.promised,
                            "exploratory": certainty_zones.exploratory,
                        },
                    })),
                },
            );
            return Err(StageError {
                phase: PHASE_CONTEXT,
                error: freeze_conflict_error(conflict),
            });
        }
        let selection_context = self.build_skill_selection_context(
            &strategy_profile,
            &context,
            &certainty_zones,
        );

        self.audit_store.touch_pipeline_phase(
            &self.input.project_root,
            self.request_id,
            PHASE_ROUTE,
        );
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
        let (route, selected_skills) = {
            let guard = self.skill_registry.read().map_err(|err| StageError {
                phase: PHASE_ROUTE,
                error: AppErrorDto::new("SKILLS_LOCK_FAILED", "skill registry lock failed", false)
                    .with_detail(err.to_string()),
            })?;
            let selected = guard
                .select_skills_for_task_with_context(self.canonical_task, &selection_context)
                .map_err(|err| StageError {
                    phase: PHASE_ROUTE,
                    error: err,
                })?;
            let resolved = AiService::inspect_task_route_with_override(
                self.canonical_task,
                selected.route_override.as_ref(),
            )
            .map_err(|err| StageError {
                phase: PHASE_ROUTE,
                error: err,
            })?;
            (resolved, selected)
        };
        let selected_skill_ids = selected_skills.all_skill_ids();
        let runtime_state_writes = selected_skills.all_state_writes();
        let runtime_affects_layers = selected_skills.all_affects_layers();
        let route_override_meta = selected_skills.route_override.as_ref().map(|route| {
            json!({
                "provider": route.provider,
                "model": route.model,
                "reason": route.reason,
            })
        });
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
                    "selectedSkills": {
                        "workflow": selected_skills.workflow_skills.len(),
                        "capability": selected_skills.capability_skills.len(),
                        "extractor": selected_skills.extractor_skills.len(),
                        "policy": selected_skills.policy_skills.len(),
                        "review": selected_skills.review_skills.len(),
                    },
                    "activeBundles": selection_context.active_bundle_ids,
                    "sceneTags": selection_context.scene_tags,
                    "availableContexts": selection_context.available_contexts,
                    "explicitSkillIds": selection_context.explicit_skill_ids,
                    "selectedSkillIds": selected_skill_ids.clone(),
                    "stateWrites": runtime_state_writes.clone(),
                    "affectsLayers": runtime_affects_layers.clone(),
                    "routeOverride": route_override_meta,
                })),
            },
        );
        self.pipeline_service
            .check_cancelled(self.request_id)
            .map_err(|err| StageError {
                phase: PHASE_ROUTE,
                error: err,
            })?;

        let continuity_depth = strategy_profile.continuity_pack_depth.clone();
        let continuity_pack = ContinuityPackCompiler.compile(
            &self.input.project_root,
            self.canonical_task,
            &continuity_depth,
            &context,
            self.context_service,
            self.input.chapter_id.as_deref(),
            &runtime_affects_layers,
        );

        self.audit_store.touch_pipeline_phase(
            &self.input.project_root,
            self.request_id,
            PHASE_PROMPT,
        );
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
                meta: Some(json!({
                    "continuityDepth": continuity_depth,
                })),
            },
        );
        let prompt = self
            .prompt_resolver
            .resolve_or_build_prompt(
                self.skill_registry,
                &context,
                &continuity_pack,
                self.canonical_task,
                self.input,
                &selected_skills,
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

        self.audit_store.touch_pipeline_phase(
            &self.input.project_root,
            self.request_id,
            PHASE_GENERATE,
        );
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
            model: route.model_id.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![ContentBlock {
                    block_type: "text".to_string(),
                    text: Some(
                        PromptResolver::generate_user_message(self.canonical_task).to_string(),
                    ),
                }],
            }],
            system_prompt: Some(prompt),
            stream: true,
            provider_id: Some(route.provider_id.clone()),
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

        self.audit_store.touch_pipeline_phase(
            &self.input.project_root,
            self.request_id,
            PHASE_POSTPROCESS,
        );
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
        let normalized = self
            .normalize_output(&generated)
            .map_err(|err| StageError {
                phase: PHASE_POSTPROCESS,
                error: err,
            })?;

        self.audit_store.touch_pipeline_phase(
            &self.input.project_root,
            self.request_id,
            PHASE_PERSIST,
        );
        let persist_mode = self.resolve_persist_mode();
        let should_persist_output = should_persist_task_output(self.canonical_task, persist_mode);
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
                meta: Some(json!({
                    "persistMode": persist_mode.as_str(),
                    "stateWritePolicy": strategy_profile.state_write_policy.as_str(),
                    "automationTier": selection_context.automation_tier.as_deref(),
                    "legacyAutoPersist": self.input.auto_persist,
                    "certaintyZones": {
                        "frozen": certainty_zones.frozen,
                        "promised": certainty_zones.promised,
                        "exploratory": certainty_zones.exploratory,
                    },
                })),
            },
        );
        self.pipeline_service
            .check_cancelled(self.request_id)
            .map_err(|err| StageError {
                phase: PHASE_PERSIST,
                error: err,
            })?;

        let persisted_records = if should_persist_output {
            if let Err(err) = self.pipeline_service.check_cancelled(self.request_id) {
                log::info!(
                    "pipeline request {} cancelled before persistence, skip writing task output",
                    self.request_id
                );
                return Err(StageError {
                    phase: PHASE_PERSIST,
                    error: err,
                });
            }
            self.task_handlers
                .persist_task_output_with_runtime_state(
                    self.canonical_task,
                    &self.input.project_root,
                    self.input,
                    &normalized,
                    self.request_id,
                    RuntimeStateWriteOptions {
                        state_writes: &runtime_state_writes,
                        state_write_policy: &strategy_profile.state_write_policy,
                        persist_mode: persist_mode.as_str(),
                        active_skill_ids: &selected_skill_ids,
                        affects_layers: &runtime_affects_layers,
                    },
                )
                .map_err(|err| StageError {
                    phase: PHASE_PERSIST,
                    error: err,
                })?
        } else {
            if self.input.auto_persist || self.input.persist_mode.is_some() {
                log::info!(
                    "[PIPELINE_PERSIST] skip task output persistence: request={} task={} mode={}",
                    self.request_id,
                    self.canonical_task,
                    persist_mode.as_str()
                );
            }
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
            | "narrative.create_obligation"
                if user_instruction.is_empty() =>
            {
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

    fn load_ai_strategy_profile(&self) -> AiStrategyProfile {
        match ProjectService.get_ai_strategy_profile(&self.input.project_root) {
            Ok(profile) => profile,
            Err(err) => {
                log::warn!(
                    "[AI_STRATEGY] failed to load ai strategy profile for {}: {} {}",
                    self.input.project_root,
                    err.code,
                    err.message
                );
                AiStrategyProfile::default()
            }
        }
    }

    fn build_skill_selection_context(
        &self,
        profile: &AiStrategyProfile,
        context: &crate::services::context_service::CollectedContext,
        certainty_zones: &CertaintyZones,
    ) -> SkillSelectionContext {
        let runtime = self.input.skill_selection.as_ref();
        let runtime_explicit_skill_ids = runtime
            .map(|selection| selection.explicit_skill_ids.as_slice())
            .unwrap_or(&[]);
        let runtime_bundle_ids = runtime
            .map(|selection| selection.active_bundle_ids.as_slice())
            .unwrap_or(&[]);
        let runtime_scene_tags = runtime
            .map(|selection| selection.scene_tags.as_slice())
            .unwrap_or(&[]);
        let runtime_contexts = runtime
            .map(|selection| selection.available_contexts.as_slice())
            .unwrap_or(&[]);
        let inferred_scene_tags = if runtime
            .map(|selection| selection.disable_inferred_scene_tags)
            .unwrap_or(false)
        {
            Vec::new()
        } else {
            self.infer_scene_tags(context)
        };
        let scene_bundle_ids = infer_scene_bundle_ids(&inferred_scene_tags);
        let generation_workflow_stack =
            resolve_generation_workflow_stack(profile, self.canonical_task);
        let baseline_explicit_skill_ids =
            merge_unique_values(&generation_workflow_stack, &profile.always_on_policy_skills);
        let explicit_skill_ids = baseline_explicit_skill_ids;
        let mut available_contexts = self.collect_available_contexts(context);
        if certainty_zones.has_any() {
            available_contexts.push("certainty_zones".to_string());
        }

        SkillSelectionContext {
            explicit_skill_ids: merge_unique_values(
                &explicit_skill_ids,
                runtime_explicit_skill_ids,
            ),
            active_bundle_ids: merge_unique_values(
                &merge_unique_values(&profile.default_capability_bundles, &scene_bundle_ids),
                runtime_bundle_ids,
            ),
            scene_tags: merge_unique_values(&inferred_scene_tags, runtime_scene_tags),
            available_contexts: merge_unique_values(&available_contexts, runtime_contexts),
            automation_tier: Some(self.resolve_automation_tier(profile)),
        }
    }

    fn resolve_automation_tier(&self, profile: &AiStrategyProfile) -> String {
        let explicit_tier = self
            .input
            .automation_tier
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(value) = explicit_tier {
            return value.to_string();
        }
        if profile.review_strictness >= 5 && is_derived_review_task(self.canonical_task) {
            return "confirm".to_string();
        }
        profile.automation_default.to_string()
    }

    fn resolve_persist_mode(&self) -> PersistMode {
        let explicit = self
            .input
            .persist_mode
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let mode = match explicit {
            Some(mode) => parse_persist_mode(mode).unwrap_or_else(|| {
                log::warn!(
                    "[PIPELINE_PERSIST] unknown persist_mode '{}', fallback to none (request={})",
                    mode,
                    self.request_id
                );
                PersistMode::None
            }),
            None if self.input.auto_persist => infer_legacy_persist_mode(self.canonical_task),
            None => PersistMode::None,
        };
        mode
    }

    fn collect_available_contexts(
        &self,
        context: &crate::services::context_service::CollectedContext,
    ) -> Vec<String> {
        let mut keys = Vec::new();
        keys.push("constitution".to_string());
        if !context.global_context.blueprint_summary.is_empty()
            || !context.related_context.characters.is_empty()
            || !context.related_context.world_rules.is_empty()
            || !context.related_context.plot_nodes.is_empty()
            || !context.related_context.relationship_edges.is_empty()
        {
            keys.push("canon".to_string());
        }
        if context.related_context.chapter.is_some() {
            keys.push("chapter".to_string());
        }
        keys.push("state".to_string());
        if context.related_context.previous_chapter_summary.is_some() {
            keys.push("recent_continuity".to_string());
        }
        keys
    }

    fn infer_scene_tags(
        &self,
        _context: &crate::services::context_service::CollectedContext,
    ) -> Vec<String> {
        let mut tags = Vec::new();
        let combined = format!(
            "{}\n{}\n{}",
            self.input.user_instruction,
            self.input.selected_text.as_deref().unwrap_or(""),
            self.input.chapter_content.as_deref().unwrap_or("")
        );
        let lowered = combined.to_ascii_lowercase();
        if lowered.contains("战斗")
            || lowered.contains("厮杀")
            || lowered.contains("决战")
            || lowered.contains("打斗")
            || lowered.contains("交手")
        {
            tags.push("battle".to_string());
        }
        if lowered.contains("对话")
            || lowered.contains("争吵")
            || lowered.contains("告白")
            || lowered.contains("情绪")
            || lowered.contains("心理")
        {
            tags.push("dialogue".to_string());
            tags.push("emotion".to_string());
        }
        if lowered.contains("场景")
            || lowered.contains("环境")
            || lowered.contains("风景")
            || lowered.contains("背景")
            || lowered.contains("世界观")
        {
            tags.push("environment".to_string());
        }
        tags.sort();
        tags.dedup();
        tags
    }
}

fn merge_unique_values(primary: &[String], secondary: &[String]) -> Vec<String> {
    let mut values = Vec::new();
    for item in primary.iter().chain(secondary.iter()) {
        let normalized = item.trim();
        if normalized.is_empty() {
            continue;
        }
        if !values
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(normalized))
        {
            values.push(normalized.to_string());
        }
    }
    values
}

fn infer_scene_bundle_ids(scene_tags: &[String]) -> Vec<String> {
    let mut bundles = Vec::new();
    for tag in scene_tags {
        match tag.trim().to_ascii_lowercase().as_str() {
            "dialogue" => bundles.push("bundle.character-expression".to_string()),
            "emotion" => bundles.push("bundle.emotion-progression".to_string()),
            "environment" | "battle" => bundles.push("bundle.scene-environment".to_string()),
            _ => {}
        }
    }
    bundles.sort();
    bundles.dedup();
    bundles
}

fn resolve_generation_workflow_stack(
    profile: &AiStrategyProfile,
    canonical_task: &str,
) -> Vec<String> {
    if !canonical_task.starts_with("chapter.") {
        return profile.default_workflow_stack.clone();
    }
    match profile
        .chapter_generation_mode
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "draft_only" => vec!["chapter.draft".to_string()],
        "plan_draft" => vec!["chapter.plan".to_string(), "chapter.draft".to_string()],
        "plan_scene_draft" => vec![
            "chapter.plan".to_string(),
            "context.collect".to_string(),
            "chapter.draft".to_string(),
        ],
        _ => profile.default_workflow_stack.clone(),
    }
}

fn extract_certainty_zones(
    context: &crate::services::context_service::CollectedContext,
) -> CertaintyZones {
    for step in &context.global_context.blueprint_summary {
        if step.step_key != "step-08-chapters" {
            continue;
        }
        if let Some(zones) = step.certainty_zones.as_ref() {
            if zones.has_any() {
                return zones.clone();
            }
        }
        if let Some(content) = step.content.as_deref() {
            if let Some(zones) = extract_certainty_zones_from_content(content) {
                if zones.has_any() {
                    return zones;
                }
            }
        }
    }

    CertaintyZones::default()
}

fn detect_freeze_conflict(
    user_instruction: &str,
    zones: &CertaintyZones,
) -> Option<FreezeConflict> {
    let normalized_instruction = user_instruction.trim();
    if normalized_instruction.is_empty() || zones.frozen.is_empty() {
        return None;
    }
    let lowered = normalized_instruction.to_ascii_lowercase();
    let mut has_mutation_intent = false;
    for keyword in ["修改", "更改", "重写", "推翻", "删除", "改动", "替换"] {
        if lowered.contains(keyword) {
            has_mutation_intent = true;
            break;
        }
    }
    if !has_mutation_intent {
        return None;
    }
    for frozen_item in &zones.frozen {
        let candidate = frozen_item.trim();
        if candidate.chars().count() < 2 {
            continue;
        }
        if lowered.contains(&candidate.to_ascii_lowercase()) {
            return Some(FreezeConflict {
                matched_zone: candidate.to_string(),
            });
        }
    }
    None
}

fn freeze_conflict_error(conflict: &FreezeConflict) -> AppErrorDto {
    AppErrorDto::new(
        "PIPELINE_FREEZE_CONFLICT",
        &format!(
            "检测到冻结区冲突：请求涉及改写冻结项「{}」，已阻断执行",
            conflict.matched_zone
        ),
        true,
    )
    .with_suggested_action("请在蓝图 > 章节路线 > 确定性分区调整冻结区，或修改指令避免改写冻结事实")
}

fn parse_persist_mode(raw: &str) -> Option<PersistMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" => Some(PersistMode::None),
        "formal" => Some(PersistMode::Formal),
        "derived_review" => Some(PersistMode::DerivedReview),
        _ => None,
    }
}

fn infer_legacy_persist_mode(canonical_task: &str) -> PersistMode {
    if canonical_task.eq_ignore_ascii_case("consistency.scan")
        || canonical_task.to_ascii_lowercase().contains("review")
    {
        PersistMode::DerivedReview
    } else {
        PersistMode::Formal
    }
}

fn should_persist_task_output(canonical_task: &str, mode: PersistMode) -> bool {
    match mode {
        PersistMode::None => false,
        PersistMode::Formal => true,
        PersistMode::DerivedReview => is_derived_review_task(canonical_task),
    }
}

fn is_derived_review_task(canonical_task: &str) -> bool {
    matches!(
        canonical_task,
        "consistency.scan"
            | "timeline.review"
            | "relationship.review"
            | "dashboard.review"
            | "export.review"
    ) || canonical_task.ends_with(".review")
}

#[cfg(test)]
mod tests {
    use super::{
        detect_freeze_conflict, extract_certainty_zones, freeze_conflict_error,
        infer_legacy_persist_mode,
        infer_scene_bundle_ids, is_derived_review_task, merge_unique_values, parse_persist_mode,
        resolve_generation_workflow_stack, should_persist_task_output, CertaintyZones,
        FreezeConflict, PersistMode,
    };
    use crate::services::context_service::{
        BlueprintStepSummary, CollectedContext, GlobalContext, RelatedContext,
    };
    use crate::services::project_service::AiStrategyProfile;

    #[test]
    fn merge_unique_values_trims_and_deduplicates_case_insensitively() {
        let merged = merge_unique_values(
            &[
                " battle ".to_string(),
                "dialogue".to_string(),
                "STATE".to_string(),
            ],
            &[
                "battle".to_string(),
                " emotion ".to_string(),
                "state".to_string(),
            ],
        );
        assert_eq!(
            merged,
            vec![
                "battle".to_string(),
                "dialogue".to_string(),
                "STATE".to_string(),
                "emotion".to_string()
            ]
        );
    }

    #[test]
    fn parse_persist_mode_accepts_contract_values() {
        assert_eq!(parse_persist_mode("none"), Some(PersistMode::None));
        assert_eq!(parse_persist_mode("formal"), Some(PersistMode::Formal));
        assert_eq!(
            parse_persist_mode("derived_review"),
            Some(PersistMode::DerivedReview)
        );
        assert_eq!(
            parse_persist_mode(" derived_review "),
            Some(PersistMode::DerivedReview)
        );
    }

    #[test]
    fn parse_persist_mode_rejects_unknown_values() {
        assert_eq!(parse_persist_mode(""), None);
        assert_eq!(parse_persist_mode("legacy"), None);
        assert_eq!(parse_persist_mode("formal_review"), None);
    }

    #[test]
    fn legacy_persist_mode_infers_review_tasks() {
        assert_eq!(
            infer_legacy_persist_mode("relationship.review"),
            PersistMode::DerivedReview
        );
        assert_eq!(
            infer_legacy_persist_mode("consistency.scan"),
            PersistMode::DerivedReview
        );
        assert_eq!(
            infer_legacy_persist_mode("chapter.plan"),
            PersistMode::Formal
        );
    }

    #[test]
    fn derived_review_mode_only_persists_review_like_tasks() {
        assert!(is_derived_review_task("timeline.review"));
        assert!(is_derived_review_task("dashboard.review"));
        assert!(!is_derived_review_task("chapter.draft"));

        assert!(should_persist_task_output(
            "timeline.review",
            PersistMode::DerivedReview
        ));
        assert!(!should_persist_task_output(
            "chapter.plan",
            PersistMode::DerivedReview
        ));
        assert!(!should_persist_task_output(
            "chapter.plan",
            PersistMode::None
        ));
        assert!(should_persist_task_output(
            "chapter.plan",
            PersistMode::Formal
        ));
    }

    #[test]
    fn infer_scene_bundle_ids_maps_scene_tags_to_default_bundles() {
        let bundles = infer_scene_bundle_ids(&[
            "dialogue".to_string(),
            "emotion".to_string(),
            "battle".to_string(),
            "environment".to_string(),
        ]);
        assert!(bundles.contains(&"bundle.character-expression".to_string()));
        assert!(bundles.contains(&"bundle.emotion-progression".to_string()));
        assert!(bundles.contains(&"bundle.scene-environment".to_string()));
    }

    #[test]
    fn extract_certainty_zones_reads_partitioned_blueprint_sections() {
        let context = CollectedContext {
            global_context: GlobalContext {
                project_name: "test".to_string(),
                genre: "玄幻".to_string(),
                narrative_pov: None,
                writing_style: None,
                locked_terms: Vec::new(),
                banned_terms: Vec::new(),
                blueprint_summary: vec![BlueprintStepSummary {
                    step_key: "step-08-chapters".to_string(),
                    title: "章节规划".to_string(),
                    status: "completed".to_string(),
                    certainty_zones: None,
                    content: Some(
                        "冻结区\n- 终局真相\n承诺区\n- 主角将直面宗门审判\n探索区\n- 支线人物立场可变化"
                            .to_string(),
                    ),
                }],
            },
            related_context: RelatedContext {
                chapter: None,
                characters: Vec::new(),
                world_rules: Vec::new(),
                plot_nodes: Vec::new(),
                relationship_edges: Vec::new(),
                previous_chapter_summary: None,
            },
        };

        let zones = extract_certainty_zones(&context);
        assert_eq!(zones.frozen, vec!["终局真相".to_string()]);
        assert_eq!(zones.promised, vec!["主角将直面宗门审判".to_string()]);
        assert_eq!(zones.exploratory, vec!["支线人物立场可变化".to_string()]);
    }

    #[test]
    fn detect_freeze_conflict_flags_mutation_on_frozen_items() {
        let zones = CertaintyZones {
            frozen: vec!["终局真相".to_string()],
            promised: vec![],
            exploratory: vec![],
        };
        assert!(detect_freeze_conflict("请重写终局真相的揭示方式", &zones).is_some());
        assert!(detect_freeze_conflict("补充一个新支线", &zones).is_none());
    }

    #[test]
    fn freeze_conflict_error_uses_blocking_code() {
        let err = freeze_conflict_error(&FreezeConflict {
            matched_zone: "终局真相".to_string(),
        });
        assert_eq!(err.code, "PIPELINE_FREEZE_CONFLICT");
        assert!(err.message.contains("终局真相"));
    }

    #[test]
    fn extract_certainty_zones_prefers_explicit_dto_over_legacy_content() {
        let context = CollectedContext {
            global_context: GlobalContext {
                project_name: "test".to_string(),
                genre: "玄幻".to_string(),
                narrative_pov: None,
                writing_style: None,
                locked_terms: Vec::new(),
                banned_terms: Vec::new(),
                blueprint_summary: vec![BlueprintStepSummary {
                    step_key: "step-08-chapters".to_string(),
                    title: "章节规划".to_string(),
                    status: "completed".to_string(),
                    certainty_zones: Some(CertaintyZones {
                        frozen: vec!["DTO-终局".to_string()],
                        promised: vec!["DTO-承诺".to_string()],
                        exploratory: vec!["DTO-探索".to_string()],
                    }),
                    content: Some(
                        "冻结区\n- 文本终局\n承诺区\n- 文本承诺\n探索区\n- 文本探索".to_string(),
                    ),
                }],
            },
            related_context: RelatedContext {
                chapter: None,
                characters: Vec::new(),
                world_rules: Vec::new(),
                plot_nodes: Vec::new(),
                relationship_edges: Vec::new(),
                previous_chapter_summary: None,
            },
        };

        let zones = extract_certainty_zones(&context);
        assert_eq!(zones.frozen, vec!["DTO-终局".to_string()]);
        assert_eq!(zones.promised, vec!["DTO-承诺".to_string()]);
        assert_eq!(zones.exploratory, vec!["DTO-探索".to_string()]);
    }

    #[test]
    fn resolve_generation_workflow_stack_respects_chapter_generation_mode() {
        let profile = AiStrategyProfile {
            chapter_generation_mode: "draft_only".to_string(),
            ..AiStrategyProfile::default()
        };
        assert_eq!(
            resolve_generation_workflow_stack(&profile, "chapter.draft"),
            vec!["chapter.draft".to_string()]
        );

        let profile = AiStrategyProfile {
            chapter_generation_mode: "plan_scene_draft".to_string(),
            ..AiStrategyProfile::default()
        };
        assert_eq!(
            resolve_generation_workflow_stack(&profile, "chapter.draft"),
            vec![
                "chapter.plan".to_string(),
                "context.collect".to_string(),
                "chapter.draft".to_string()
            ]
        );
    }
}
