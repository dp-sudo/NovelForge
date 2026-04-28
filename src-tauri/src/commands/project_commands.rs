use serde::{Deserialize, Serialize};
use tauri::State;

use crate::errors::AppErrorDto;
use crate::infra::recent_projects::RecentProjectItem;
use crate::services::project_service::{CreateProjectInput, ProjectOpenResult, WritingStyle};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateProjectInput {
    pub name: String,
    pub force_error: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateProjectOutput {
    pub normalized_name: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectInput {
    pub project_root: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshotInput {
    pub project_root: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveWritingStyleInput {
    pub project_root: String,
    pub writing_style: WritingStyle,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWritingStyleInput {
    pub project_root: String,
}

#[tauri::command]
pub async fn validate_project(
    input: ValidateProjectInput,
    state: State<'_, AppState>,
) -> Result<ValidateProjectOutput, AppErrorDto> {
    if input.force_error.unwrap_or(false) {
        return Err(AppErrorDto::new(
            "PROJECT_VALIDATE_FORCED",
            "这是一个用于链路验证的强制错误",
            true,
        )
        .with_detail("forceError=true")
        .with_suggested_action("把 forceError 设为 false 以走成功路径"));
    }

    let normalized_name = state.project_service.validate_name(&input.name)?;
    Ok(ValidateProjectOutput {
        message: "项目名称验证通过".to_string(),
        normalized_name,
    })
}

#[tauri::command]
pub async fn create_project(
    input: CreateProjectInput,
    state: State<'_, AppState>,
) -> Result<ProjectOpenResult, AppErrorDto> {
    let result = state.project_service.create_project(input)?;
    crate::infra::logger::log_user_action("create_project", &result.project_root);
    Ok(result)
}

#[tauri::command]
pub async fn open_project(
    input: OpenProjectInput,
    state: State<'_, AppState>,
) -> Result<ProjectOpenResult, AppErrorDto> {
    let result = state.project_service.open_project(&input.project_root)?;
    crate::infra::logger::log_user_action("open_project", &input.project_root);
    // Auto-backup on first open of the day (best-effort, never blocks)
    state.backup_service.try_auto_backup(&input.project_root);
    Ok(result)
}

#[tauri::command]
pub async fn list_recent_projects(
    state: State<'_, AppState>,
) -> Result<Vec<RecentProjectItem>, AppErrorDto> {
    state.project_service.list_recent_projects()
}

#[tauri::command]
pub async fn clear_recent_projects(state: State<'_, AppState>) -> Result<(), AppErrorDto> {
    state.project_service.clear_recent_projects()
}

#[tauri::command]
pub async fn save_writing_style(
    input: SaveWritingStyleInput,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .project_service
        .save_writing_style(&input.project_root, &input.writing_style)
}

#[tauri::command]
pub async fn get_writing_style(
    input: GetWritingStyleInput,
    state: State<'_, AppState>,
) -> Result<WritingStyle, AppErrorDto> {
    state.project_service.get_writing_style(&input.project_root)
}

#[tauri::command]
pub async fn init_project_repository(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<crate::services::git_service::GitRepositoryStatus, AppErrorDto> {
    state.git_service.init_repository(&project_root)
}

#[tauri::command]
pub async fn get_project_repository_status(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<crate::services::git_service::GitRepositoryStatus, AppErrorDto> {
    state.git_service.read_status(&project_root)
}

#[tauri::command]
pub async fn commit_project_snapshot(
    input: GitSnapshotInput,
    state: State<'_, AppState>,
) -> Result<crate::services::git_service::GitSnapshotResult, AppErrorDto> {
    state
        .git_service
        .commit_snapshot(&input.project_root, input.message)
}

#[tauri::command]
pub async fn list_project_history(
    project_root: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::git_service::GitCommitRecord>, AppErrorDto> {
    state
        .git_service
        .list_history(&project_root, limit.unwrap_or(20))
}
