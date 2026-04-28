use serde::Deserialize;
use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::export_service::{ExportOptions, ExportOutput};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportChapterRequest {
    pub project_root: String,
    pub chapter_id: String,
    pub format: String,
    pub output_path: String,
    pub options: Option<ExportOptions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBookRequest {
    pub project_root: String,
    pub format: String,
    pub output_path: String,
    pub options: Option<ExportOptions>,
}

#[tauri::command]
pub async fn export_chapter(
    input: ExportChapterRequest,
    state: State<'_, AppState>,
) -> Result<ExportOutput, AppErrorDto> {
    state.export_service.export_chapter(
        &input.project_root,
        &input.chapter_id,
        &input.format,
        &input.output_path,
        input.options,
    )
}

#[tauri::command]
pub async fn export_book(
    input: ExportBookRequest,
    state: State<'_, AppState>,
) -> Result<ExportOutput, AppErrorDto> {
    state.export_service.export_book(
        &input.project_root,
        &input.format,
        &input.output_path,
        input.options,
    )
}
