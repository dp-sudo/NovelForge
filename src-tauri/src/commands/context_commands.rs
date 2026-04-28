use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::context_service::EditorContextPanel;
use crate::state::AppState;

#[tauri::command]
pub async fn get_chapter_context(
    project_root: String,
    chapter_id: String,
    state: State<'_, AppState>,
) -> Result<EditorContextPanel, AppErrorDto> {
    state
        .context_service
        .collect_editor_context(&project_root, &chapter_id)
}
