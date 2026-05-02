use tauri::State;
use uuid::Uuid;

use crate::adapters::ProviderConfig;
use crate::errors::AppErrorDto;
use crate::services::ai_pipeline_service::RunAiTaskPipelineInput;
use crate::services::context_service::ContextService;
use crate::state::AppState;

fn deprecated_source(source: Option<&str>) -> &str {
    source.unwrap_or("unknown")
}

const DEPRECATED_REGISTER_AI_PROVIDER_LOG: &str =
    "[DEPRECATED_COMMAND] register_ai_provider is compatibility-only";
const DEPRECATED_TEST_AI_CONNECTION_LOG: &str =
    "[DEPRECATED_COMMAND] test_ai_connection is compatibility-only";

fn log_deprecated_command(message: &str, command: &str, source: Option<&str>) {
    let src = deprecated_source(source);
    log::warn!("{} source={}", message, src);
    crate::infra::logger::log_user_action(
        "compatibility_bridge.used",
        &format!("command={} source={}", command, src),
    );
    crate::infra::logger::record_deprecated_command_usage(command, src);
}

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
    source: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    // 问题2修复(命令面收敛): compatibility-only，计划在 2026-07-31 后移除。
    log_deprecated_command(
        DEPRECATED_REGISTER_AI_PROVIDER_LOG,
        "register_ai_provider",
        source.as_deref(),
    );
    crate::services::settings_service::validate_provider_base_url_security(&config.base_url)?;
    state.ai_service.register_provider(config).await;
    Ok(())
}

#[tauri::command]
pub async fn test_ai_connection(
    provider_id: String,
    source: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    // 问题2修复(命令面收敛): compatibility-only，计划在 2026-07-31 后移除。
    log_deprecated_command(
        DEPRECATED_TEST_AI_CONNECTION_LOG,
        "test_ai_connection",
        source.as_deref(),
    );
    state.ai_service.test_connection(&provider_id).await
}
