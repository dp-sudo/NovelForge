use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::Emitter;
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::services::ai_pipeline::audit_store::PipelineAuditStore;
use crate::services::ai_pipeline::orchestrator::{
    PipelineOrchestrator, PHASE_DONE, PHASE_VALIDATE,
};
use crate::services::ai_pipeline::prompt_resolver::PromptResolver;
use crate::services::ai_pipeline::task_handlers::TaskHandlers;
use crate::services::ai_service::AiService;
use crate::services::context_service::ContextService;
use crate::services::skill_registry::SkillRegistry;
use crate::services::task_routing;

const PIPELINE_EVENT_NAME: &str = "ai:pipeline:event";

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
    // 问题3修复(职责拆分): 业务编排仅负责协调，审计/Prompt/任务处理下沉到独立模块。
    cancelled_requests: Arc<RwLock<HashSet<String>>>,
    audit_store: PipelineAuditStore,
    prompt_resolver: PromptResolver,
    task_handlers: TaskHandlers,
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

        let run_result = match self.audit_store.insert_pipeline_run(
            &input.project_root,
            &request_id,
            input.chapter_id.as_deref(),
            &canonical_task,
            input.ui_action.as_deref(),
            PHASE_VALIDATE,
        ) {
            Ok(()) => {
                let orchestrator = PipelineOrchestrator {
                    pipeline_service: self,
                    audit_store: &self.audit_store,
                    prompt_resolver: &self.prompt_resolver,
                    task_handlers: &self.task_handlers,
                    app_handle,
                    ai_service,
                    context_service,
                    skill_registry,
                    request_id: &request_id,
                    canonical_task: &canonical_task,
                    input: &input,
                };
                orchestrator.run().await
            }
            Err(error) => Err(crate::services::ai_pipeline::orchestrator::StageError {
                phase: PHASE_VALIDATE,
                error,
            }),
        };

        match run_result {
            Ok(success) => {
                self.audit_store.update_pipeline_run(
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
                            "persistedRecords": success.persisted_records.clone(),
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
                self.audit_store.update_pipeline_run(
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

    pub(crate) fn emit_event(&self, app_handle: &tauri::AppHandle, event: AiPipelineEvent) {
        let _ = app_handle.emit(PIPELINE_EVENT_NAME, event);
    }

    pub(crate) fn check_cancelled(&self, request_id: &str) -> Result<(), AppErrorDto> {
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
}

#[cfg(test)]
mod tests {
    use super::AiPipelineService;

    #[test]
    fn cancel_rejects_empty_request_id() {
        let service = AiPipelineService::default();
        let err = service
            .cancel_ai_task_pipeline("   ")
            .expect_err("should reject empty request id");
        assert_eq!(err.code, "PIPELINE_REQUEST_ID_REQUIRED");
    }

    #[test]
    fn cancel_marks_request_as_cancelled() {
        let service = AiPipelineService::default();
        service
            .cancel_ai_task_pipeline("req-1")
            .expect("cancel should succeed");
        let err = service
            .check_cancelled("req-1")
            .expect_err("should be cancelled");
        assert_eq!(err.code, "PIPELINE_CANCELLED");
    }
}
