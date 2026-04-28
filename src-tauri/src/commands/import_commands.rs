use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::import_service::ImportInput;
use crate::state::AppState;

#[tauri::command]
pub async fn import_chapter_files(
    input: ImportInput,
    state: State<'_, AppState>,
) -> Result<crate::services::import_service::ImportResult, AppErrorDto> {
    state.import_service.import_files(input)
}

#[tauri::command]
pub async fn create_backup(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<crate::services::backup_service::BackupResult, AppErrorDto> {
    state.backup_service.create_backup(&project_root)
}

#[tauri::command]
pub async fn list_backups(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::backup_service::BackupResult>, AppErrorDto> {
    state.backup_service.list_backups(&project_root)
}

#[tauri::command]
pub async fn restore_backup(
    project_root: String,
    backup_path: String,
    state: State<'_, AppState>,
) -> Result<crate::services::backup_service::RestoreResult, AppErrorDto> {
    state
        .backup_service
        .restore_backup(&project_root, &backup_path)
}
