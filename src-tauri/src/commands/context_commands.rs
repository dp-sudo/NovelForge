use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::context_service::{
    ApplyAssetCandidateInput, ApplyAssetCandidateResult, ApplyStructuredDraftInput,
    ApplyStructuredDraftResult, EditorContextPanel, RejectStructuredDraftResult,
};
use crate::services::review_trail_service::{ReviewTrailRecord, ReviewTrailService};
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
pub async fn materialize_chapter_structured_drafts(
    project_root: String,
    chapter_id: String,
    state: State<'_, AppState>,
) -> Result<EditorContextPanel, AppErrorDto> {
    state
        .context_service
        .collect_editor_context_with_persisted_drafts(&project_root, &chapter_id)
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
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<ApplyStructuredDraftResult, AppErrorDto> {
    state.context_service.apply_structured_draft_with_reason(
        &project_root,
        &chapter_id,
        input,
        reason.as_deref(),
    )
}

#[tauri::command]
pub async fn reject_structured_draft(
    project_root: String,
    chapter_id: String,
    draft_item_id: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<RejectStructuredDraftResult, AppErrorDto> {
    state.context_service.reject_structured_draft_with_reason(
        &project_root,
        &chapter_id,
        &draft_item_id,
        reason.as_deref(),
    )
}

#[tauri::command]
pub async fn get_review_trail(
    project_root: String,
    entity_type: String,
    entity_id: String,
) -> Result<Vec<ReviewTrailRecord>, AppErrorDto> {
    ReviewTrailService.get_review_trail(&project_root, &entity_type, &entity_id)
}
