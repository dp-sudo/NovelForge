use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::glossary_service::{CreateGlossaryTermInput, GlossaryService};
use crate::state::AppState;

#[tauri::command]
pub async fn list_glossary_terms(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::glossary_service::GlossaryTermRecord>, AppErrorDto> {
    state.glossary_service.list(&project_root)
}

#[tauri::command]
pub async fn create_glossary_term(
    project_root: String,
    input: CreateGlossaryTermInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.glossary_service.create(&project_root, input)
}
