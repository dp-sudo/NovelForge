use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::context_service::{
    ApplyAssetCandidateInput, ApplyAssetCandidateResult, ApplyStructuredDraftInput,
    ApplyStructuredDraftResult, EditorContextPanel,
};
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

#[tauri::command]
pub async fn apply_asset_candidate(
    project_root: String,
    chapter_id: String,
    input: ApplyAssetCandidateInput,
    state: State<'_, AppState>,
) -> Result<ApplyAssetCandidateResult, AppErrorDto> {
    state
        .context_service
        .apply_asset_candidate(&project_root, &chapter_id, input)
}

#[tauri::command]
pub async fn apply_structured_draft(
    project_root: String,
    chapter_id: String,
    input: ApplyStructuredDraftInput,
    state: State<'_, AppState>,
) -> Result<ApplyStructuredDraftResult, AppErrorDto> {
    state
        .context_service
        .apply_structured_draft(&project_root, &chapter_id, input)
}
