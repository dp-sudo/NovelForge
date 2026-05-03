use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::plot_service::CreatePlotNodeInput;
use crate::state::AppState;

#[tauri::command]
pub async fn list_plot_nodes(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::plot_service::PlotNodeRecord>, AppErrorDto> {
    state.plot_service.list(&project_root)
}

#[tauri::command]
pub async fn create_plot_node(
    project_root: String,
    input: CreatePlotNodeInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.plot_service.create(&project_root, input)
}

#[tauri::command]
pub async fn reorder_plot_nodes(
    project_root: String,
    ordered_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.plot_service.reorder(&project_root, ordered_ids)
}
