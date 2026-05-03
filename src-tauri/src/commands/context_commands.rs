use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::context_service::{
    ApplyAssetCandidateInput, ApplyAssetCandidateResult, ApplyStructuredDraftInput,
    ApplyStructuredDraftResult, EditorContextPanel, ReviewQueueItem,
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

#[tauri::command]
pub async fn update_review_queue_item_status(
    project_root: String,
    item_id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .context_service
        .update_review_queue_item_status(&project_root, &item_id, &status)
}

#[tauri::command]
pub async fn list_review_work_items(
    project_root: String,
    chapter_id: Option<String>,
    task_type: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewQueueItem>, AppErrorDto> {
    state.context_service.list_review_work_items(
        &project_root,
        chapter_id.as_deref(),
        task_type.as_deref(),
        status.as_deref(),
        limit.unwrap_or(100),
    )
}
