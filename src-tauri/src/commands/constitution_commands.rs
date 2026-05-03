use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::constitution_service::{
    ConstitutionRule, ConstitutionValidationResult, ConstitutionViolation,
    CreateConstitutionRuleInput, UpdateConstitutionRuleInput,
};
use crate::state::AppState;

#[tauri::command]
pub async fn list_constitution_rules(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<ConstitutionRule>, AppErrorDto> {
    state.constitution_service.list(&project_root)
}

#[tauri::command]
pub async fn create_constitution_rule(
    project_root: String,
    input: CreateConstitutionRuleInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.constitution_service.create(&project_root, input)
}

#[tauri::command]
pub async fn update_constitution_rule(
    project_root: String,
    id: String,
    input: UpdateConstitutionRuleInput,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.constitution_service.update(&project_root, &id, input)
}

#[tauri::command]
pub async fn delete_constitution_rule(
    project_root: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.constitution_service.delete(&project_root, &id)
}

#[tauri::command]
pub async fn validate_constitution(
    project_root: String,
    text: String,
    run_id: Option<String>,
    chapter_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<ConstitutionValidationResult, AppErrorDto> {
    state.constitution_service.validate_text(
        &project_root,
        &text,
        run_id.as_deref(),
        chapter_id.as_deref(),
    )
}

#[tauri::command]
pub async fn list_constitution_violations(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<ConstitutionViolation>, AppErrorDto> {
    state.constitution_service.list_violations(&project_root)
}

#[tauri::command]
pub async fn update_violation_status(
    project_root: String,
    violation_id: String,
    status: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.constitution_service.update_violation_status(
        &project_root,
        &violation_id,
        &status,
        note.as_deref(),
    )
}

#[tauri::command]
pub async fn get_constitution_prompt_text(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state
        .constitution_service
        .collect_rules_for_prompt(&project_root)
}
