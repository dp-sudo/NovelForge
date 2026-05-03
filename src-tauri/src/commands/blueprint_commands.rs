use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::blueprint_service::SaveBlueprintStepInput;
use crate::state::AppState;

#[tauri::command]
pub async fn list_blueprint_steps(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::blueprint_service::BlueprintStep>, AppErrorDto> {
    state.blueprint_service.list_steps(&project_root)
}

#[tauri::command]
pub async fn save_blueprint_step(
    project_root: String,
    input: SaveBlueprintStepInput,
    state: State<'_, AppState>,
) -> Result<crate::services::blueprint_service::BlueprintStep, AppErrorDto> {
    state.blueprint_service.save_step(&project_root, input)
}

#[tauri::command]
pub async fn mark_blueprint_completed(
    project_root: String,
    step_key: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .blueprint_service
        .mark_completed(&project_root, &step_key)
}

#[tauri::command]
pub async fn reset_blueprint_step(
    project_root: String,
    step_key: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.blueprint_service.reset_step(&project_root, &step_key)
}
