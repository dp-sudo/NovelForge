use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::consistency_service::{ConsistencyService, ScanChapterInput};
use crate::state::AppState;

#[tauri::command]
pub async fn scan_chapter_consistency(
    project_root: String,
    input: ScanChapterInput,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::consistency_service::ConsistencyIssue>, AppErrorDto> {
    crate::infra::logger::log_user_action(
        "consistency_scan",
        &format!("chapter={}", input.chapter_id),
    );
    state.consistency_service.scan_chapter(&project_root, input)
}

#[tauri::command]
pub async fn list_consistency_issues(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::consistency_service::ConsistencyIssue>, AppErrorDto> {
    state.consistency_service.list_issues(&project_root)
}

#[tauri::command]
pub async fn update_issue_status(
    project_root: String,
    issue_id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .consistency_service
        .update_issue_status(&project_root, &issue_id, &status)
}
