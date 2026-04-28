use tauri::State;

use crate::errors::AppErrorDto;
use crate::state::AppState;

#[tauri::command]
pub async fn get_dashboard_stats(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<crate::services::dashboard_service::DashboardStats, AppErrorDto> {
    state.dashboard_service.get_stats(&project_root)
}
