use serde::Deserialize;
use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::chapter_service::{
    AutosaveDraftInput, ChapterInput, ChapterRecord, CreateVolumeInput, RecoverDraftResult,
    SaveChapterOutput, TimelineEntryRecord,
};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChapterRequest {
    pub project_root: String,
    pub input: ChapterInput,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveChapterRequest {
    pub project_root: String,
    pub chapter_id: String,
    pub content: String,
    pub request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverDraftRequest {
    pub project_root: String,
    pub chapter_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteChapterRequest {
    pub id: String,
}

#[tauri::command]
pub async fn list_chapters(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChapterRecord>, AppErrorDto> {
    state.chapter_service.list_chapters(&project_root)
}

#[tauri::command]
pub async fn list_timeline_entries(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<TimelineEntryRecord>, AppErrorDto> {
    state.chapter_service.list_timeline_entries(&project_root)
}

#[tauri::command]
pub async fn reorder_chapters(
    project_root: String,
    ordered_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .chapter_service
        .reorder_chapters(&project_root, ordered_ids)
}

#[tauri::command]
pub async fn create_chapter(
    input: CreateChapterRequest,
    state: State<'_, AppState>,
) -> Result<ChapterRecord, AppErrorDto> {
    let result = state
        .chapter_service
        .create_chapter(&input.project_root, input.input)?;
    crate::infra::logger::log_user_action("create_chapter", &format!("chapter={}", result.title));
    Ok(result)
}

#[tauri::command]
pub async fn save_chapter_content(
    input: SaveChapterRequest,
    state: State<'_, AppState>,
) -> Result<SaveChapterOutput, AppErrorDto> {
    let request_id = input.request_id.as_deref().unwrap_or("n/a");
    crate::infra::logger::log_user_action(
        "save_chapter.start",
        &format!("requestId={}, chapter={}", request_id, input.chapter_id),
    );
    match state.chapter_service.save_chapter_content(
        &input.project_root,
        &input.chapter_id,
        &input.content,
    ) {
        Ok(result) => {
            crate::infra::logger::log_user_action(
                "save_chapter.done",
                &format!(
                    "requestId={}, chapter={}, words={}, version={}",
                    request_id, input.chapter_id, result.current_words, result.version
                ),
            );
            Ok(result)
        }
        Err(err) => {
            crate::infra::logger::log_command_error(
                "save_chapter_content",
                &format!(
                    "requestId={}, chapter={}, code={}, message={}",
                    request_id,
                    input.chapter_id,
                    err.code.as_str(),
                    err.message.as_str()
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn autosave_draft(
    input: AutosaveDraftInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    let request_id = input.request_id.as_deref().unwrap_or("n/a");
    crate::infra::logger::log_user_action(
        "autosave.start",
        &format!("requestId={}, chapter={}", request_id, input.chapter_id),
    );
    match state.chapter_service.autosave_draft(
        &input.project_root,
        &input.chapter_id,
        &input.content,
    ) {
        Ok(draft_path) => {
            crate::infra::logger::log_user_action(
                "autosave.done",
                &format!("requestId={}, chapter={}", request_id, input.chapter_id),
            );
            Ok(draft_path)
        }
        Err(err) => {
            crate::infra::logger::log_command_error(
                "autosave_draft",
                &format!(
                    "requestId={}, chapter={}, code={}, message={}",
                    request_id,
                    input.chapter_id,
                    err.code.as_str(),
                    err.message.as_str()
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn recover_draft(
    input: RecoverDraftRequest,
    state: State<'_, AppState>,
) -> Result<RecoverDraftResult, AppErrorDto> {
    state
        .chapter_service
        .recover_draft(&input.project_root, &input.chapter_id)
}

#[tauri::command]
pub async fn read_chapter_content(
    project_root: String,
    chapter_id: String,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state
        .chapter_service
        .read_chapter_content(&project_root, &chapter_id)
}

#[tauri::command]
pub async fn delete_chapter(
    project_root: String,
    input: DeleteChapterRequest,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .chapter_service
        .delete_chapter(&project_root, &input.id)
}

// ── Snapshot Commands ──

#[tauri::command]
pub async fn create_snapshot(
    project_root: String,
    chapter_id: String,
    title: Option<String>,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::services::chapter_service::SnapshotRecord, AppErrorDto> {
    state.chapter_service.create_snapshot(
        &project_root,
        &chapter_id,
        title.as_deref(),
        note.as_deref(),
    )
}

#[tauri::command]
pub async fn list_snapshots(
    project_root: String,
    chapter_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::chapter_service::SnapshotRecord>, AppErrorDto> {
    state
        .chapter_service
        .list_snapshots(&project_root, chapter_id.as_deref())
}

#[tauri::command]
pub async fn read_snapshot_content(
    project_root: String,
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state
        .chapter_service
        .read_snapshot_content(&project_root, &snapshot_id)
}

// ── Volume Commands ──

#[tauri::command]
pub async fn list_volumes(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::chapter_service::VolumeRecord>, AppErrorDto> {
    state.volume_service.list(&project_root)
}

#[tauri::command]
pub async fn create_volume(
    project_root: String,
    input: CreateVolumeInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.volume_service.create(&project_root, input)
}

#[tauri::command]
pub async fn delete_volume(
    project_root: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.volume_service.delete(&project_root, &id)
}

#[tauri::command]
pub async fn assign_chapter_volume(
    project_root: String,
    chapter_id: String,
    volume_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .volume_service
        .assign_chapter(&project_root, &chapter_id, volume_id.as_deref())
}
