use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::narrative_service::CreateObligationInput;
use crate::state::AppState;

#[tauri::command]
pub async fn list_narrative_obligations(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::narrative_service::NarrativeObligation>, AppErrorDto> {
    state.narrative_service.list(&project_root)
}

#[tauri::command]
pub async fn create_narrative_obligation(
    project_root: String,
    input: CreateObligationInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.narrative_service.create(&project_root, input)
}

#[tauri::command]
pub async fn update_obligation_status(
    project_root: String,
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .narrative_service
        .update_status(&project_root, &id, &status)
}

#[tauri::command]
pub async fn delete_narrative_obligation(
    project_root: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.narrative_service.delete(&project_root, &id)
}
