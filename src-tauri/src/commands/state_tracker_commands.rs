use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::state_tracker_service::{
    CreateSnapshotInput, StateSnapshotSummary, StoryStateSnapshot,
};
use crate::state::AppState;

#[tauri::command]
pub async fn create_state_snapshot(
    project_root: String,
    input: CreateSnapshotInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state
        .state_tracker_service
        .create_snapshot(&project_root, input)
}

#[tauri::command]
pub async fn get_latest_state_snapshot(
    project_root: String,
    chapter_id: String,
    state: State<'_, AppState>,
) -> Result<Option<StoryStateSnapshot>, AppErrorDto> {
    state
        .state_tracker_service
        .get_latest_snapshot(&project_root, &chapter_id)
}

#[tauri::command]
pub async fn list_state_snapshots(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<StateSnapshotSummary>, AppErrorDto> {
    state.state_tracker_service.list_snapshots(&project_root)
}

#[tauri::command]
pub async fn delete_state_snapshot(
    project_root: String,
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .state_tracker_service
        .delete_snapshot(&project_root, &snapshot_id)
}

#[tauri::command]
pub async fn get_state_prompt_text(
    project_root: String,
    chapter_id: String,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state
        .state_tracker_service
        .collect_state_for_prompt(&project_root, &chapter_id)
}
