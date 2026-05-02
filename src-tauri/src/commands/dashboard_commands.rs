use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::feedback_service::{FeedbackEventRecord, FeedbackService};
use crate::state::AppState;

#[tauri::command]
pub async fn get_dashboard_stats(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<crate::services::dashboard_service::DashboardStats, AppErrorDto> {
    state.dashboard_service.get_stats(&project_root)
}

#[tauri::command]
pub async fn get_feedback_events(
    project_root: String,
) -> Result<Vec<FeedbackEventRecord>, AppErrorDto> {
    FeedbackService.get_feedback_events(&project_root)
}

#[tauri::command]
pub async fn acknowledge_feedback_event(
    project_root: String,
    event_id: String,
) -> Result<FeedbackEventRecord, AppErrorDto> {
    FeedbackService.acknowledge_feedback_event(&project_root, &event_id)
}

#[tauri::command]
pub async fn resolve_feedback_event(
    project_root: String,
    event_id: String,
    note: String,
) -> Result<FeedbackEventRecord, AppErrorDto> {
    FeedbackService.resolve_feedback_event(&project_root, &event_id, &note)
}

#[tauri::command]
pub async fn ignore_feedback_event(
    project_root: String,
    event_id: String,
    reason: String,
) -> Result<FeedbackEventRecord, AppErrorDto> {
    FeedbackService.ignore_feedback_event(&project_root, &event_id, &reason)
}
