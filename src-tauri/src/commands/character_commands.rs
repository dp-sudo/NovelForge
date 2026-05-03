use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::character_service::{
    CharacterService, CreateCharacterInput, CreateRelationshipInput, RelationshipService,
    UpdateCharacterInput,
};
use crate::state::AppState;

#[tauri::command]
pub async fn list_characters(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::character_service::CharacterRecord>, AppErrorDto> {
    state.character_service.list(&project_root)
}

#[tauri::command]
pub async fn create_character(
    project_root: String,
    input: CreateCharacterInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.character_service.create(&project_root, input)
}

#[tauri::command]
pub async fn update_character(
    project_root: String,
    input: UpdateCharacterInput,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.character_service.update(&project_root, input)
}

#[tauri::command]
pub async fn delete_character(
    project_root: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.character_service.soft_delete(&project_root, &id)
}

#[tauri::command]
pub async fn list_character_relationships(
    project_root: String,
    character_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::character_service::CharacterRelationship>, AppErrorDto> {
    state
        .relationship_service
        .list(&project_root, character_id.as_deref())
}

#[tauri::command]
pub async fn create_character_relationship(
    project_root: String,
    input: CreateRelationshipInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.relationship_service.create(&project_root, input)
}

#[tauri::command]
pub async fn delete_character_relationship(
    project_root: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.relationship_service.delete(&project_root, &id)
}
