use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::command_center_service::CommandCenterSnapshot;
use crate::state::AppState;

#[tauri::command]
pub async fn get_command_center_snapshot(
    project_root: String,
    chapter_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<CommandCenterSnapshot, AppErrorDto> {
    state
        .command_center_service
        .get_snapshot(&project_root, chapter_id.as_deref())
}
