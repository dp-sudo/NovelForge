use tauri::State;
use uuid::Uuid;

use crate::adapters::ProviderConfig;
use crate::errors::AppErrorDto;
use crate::services::ai_pipeline_service::RunAiTaskPipelineInput;
use crate::services::context_service::ContextService;
use crate::state::AppState;

#[tauri::command]
pub async fn run_ai_task_pipeline(
    app_handle: tauri::AppHandle,
    input: RunAiTaskPipelineInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    let request_id = Uuid::new_v4().to_string();
    crate::infra::logger::log_user_action(
        "pipeline.start",
        &format!(
            "requestId={}, taskType={}, chapterId={}",
            request_id,
            input.task_type,
            input
                .chapter_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .unwrap_or("n/a")
        ),
    );
    spawn_pipeline_run(&app_handle, &state, request_id.clone(), input);
    Ok(request_id)
}

#[tauri::command]
pub async fn cancel_ai_task_pipeline(
    request_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    crate::infra::logger::log_user_action("pipeline.cancel", &format!("requestId={}", request_id));
    state
        .ai_pipeline_service
        .cancel_ai_task_pipeline(&request_id)
}

fn spawn_pipeline_run(
    app_handle: &tauri::AppHandle,
    state: &State<'_, AppState>,
    request_id: String,
    input: RunAiTaskPipelineInput,
) {
    let app = app_handle.clone();
    let pipeline_service = state.ai_pipeline_service.clone();
    let ai_service = state.ai_service.clone();
    let skill_registry = state.skill_registry.clone();
    tokio::spawn(async move {
        let context_service = ContextService;
        let run_result = pipeline_service
            .run_ai_task_pipeline_with_request_id(
                &app,
                &ai_service,
                &context_service,
                &skill_registry,
                request_id.clone(),
                input,
            )
            .await;
        match run_result {
            Ok(result) => {
                crate::infra::logger::log_user_action(
                    "pipeline.done",
                    &format!(
                        "requestId={}, status={}, taskType={}",
                        request_id, result.status, result.task_type
                    ),
                );
            }
            Err(err) => {
                if err.code == "PIPELINE_CANCELLED" {
                    crate::infra::logger::log_user_action(
                        "pipeline.cancelled",
                        &format!("requestId={}, reason=cancelled_by_client", request_id),
                    );
                } else {
                    crate::infra::logger::log_command_error(
                        "run_ai_task_pipeline",
                        &format!(
                            "requestId={}, code={}, message={}",
                            request_id, err.code, err.message
                        ),
                    );
                }
            }
        }
    });
}

#[tauri::command]
pub async fn register_ai_provider(
    config: ProviderConfig,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    log::warn!("[DEPRECATED_COMMAND] register_ai_provider is compatibility-only");
    state.ai_service.register_provider(config).await;
    Ok(())
}

#[tauri::command]
pub async fn test_ai_connection(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    log::warn!("[DEPRECATED_COMMAND] test_ai_connection is compatibility-only");
    state.ai_service.test_connection(&provider_id).await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintSuggestionInput {
    pub project_root: String,
    pub step_key: String,
    pub step_title: String,
    pub user_instruction: String,
}

#[tauri::command]
pub async fn generate_blueprint_suggestion(
    app_handle: tauri::AppHandle,
    input: BlueprintSuggestionInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call(
        "blueprint",
        "default",
        &format!("blueprint.{}", input.step_key),
        None,
    );
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "blueprint.generate_step".to_string(),
            chapter_id: None,
            ui_action: Some("generate_blueprint_suggestion".to_string()),
            user_instruction: input.user_instruction,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: Some(input.step_key),
            blueprint_step_title: Some(input.step_title),
            auto_persist: true,
        },
    )
    .await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCharacterInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_character(
    app_handle: tauri::AppHandle,
    input: AiCharacterInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("character", "default", "character.create", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "character.create".to_string(),
            chapter_id: None,
            ui_action: Some("ai_generate_character".to_string()),
            user_instruction: input.user_description,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: true,
        },
    )
    .await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWorldRuleInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_world_rule(
    app_handle: tauri::AppHandle,
    input: AiWorldRuleInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("world", "default", "world.create_rule", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "world.create_rule".to_string(),
            chapter_id: None,
            ui_action: Some("ai_generate_world_rule".to_string()),
            user_instruction: input.user_description,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: true,
        },
    )
    .await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPlotNodeInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_plot_node(
    app_handle: tauri::AppHandle,
    input: AiPlotNodeInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("plot", "default", "plot.create_node", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "plot.create_node".to_string(),
            chapter_id: None,
            ui_action: Some("ai_generate_plot_node".to_string()),
            user_instruction: input.user_description,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: true,
        },
    )
    .await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConsistencyInput {
    pub project_root: String,
    pub chapter_id: String,
    pub chapter_content: String,
}

#[tauri::command]
pub async fn ai_scan_consistency(
    app_handle: tauri::AppHandle,
    input: AiConsistencyInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("consistency", "default", "consistency.scan", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "consistency.scan".to_string(),
            chapter_id: Some(input.chapter_id),
            ui_action: Some("ai_scan_consistency".to_string()),
            user_instruction: String::new(),
            selected_text: None,
            chapter_content: Some(input.chapter_content),
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: true,
        },
    )
    .await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGlossaryInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_glossary_term(
    app_handle: tauri::AppHandle,
    input: AiGlossaryInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("glossary", "default", "glossary.create_term", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "glossary.create_term".to_string(),
            chapter_id: None,
            ui_action: Some("ai_generate_glossary_term".to_string()),
            user_instruction: input.user_description,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: true,
        },
    )
    .await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiNarrativeInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_narrative_obligation(
    app_handle: tauri::AppHandle,
    input: AiNarrativeInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call(
        "narrative",
        "default",
        "narrative.create_obligation",
        None,
    );
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "narrative.create_obligation".to_string(),
            chapter_id: None,
            ui_action: Some("ai_generate_narrative_obligation".to_string()),
            user_instruction: input.user_description,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: true,
        },
    )
    .await
}

async fn run_pipeline_text_result(
    app_handle: &tauri::AppHandle,
    state: &State<'_, AppState>,
    input: RunAiTaskPipelineInput,
) -> Result<String, AppErrorDto> {
    let context_service = ContextService;
    let result = state
        .ai_pipeline_service
        .run_ai_task_pipeline(
            app_handle,
            &state.ai_service,
            &context_service,
            &state.skill_registry,
            input,
        )
        .await?;
    Ok(result.output_text.unwrap_or_default())
}
