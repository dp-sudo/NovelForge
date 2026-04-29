use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::world_service::CreateWorldRuleInput;
use crate::state::AppState;

#[tauri::command]
pub async fn list_world_rules(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::world_service::WorldRuleRecord>, AppErrorDto> {
    state.world_service.list(&project_root)
}

#[tauri::command]
pub async fn create_world_rule(
    project_root: String,
    input: CreateWorldRuleInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.world_service.create(&project_root, input)
}

#[tauri::command]
pub async fn delete_world_rule(
    project_root: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.world_service.soft_delete(&project_root, &id)
}
