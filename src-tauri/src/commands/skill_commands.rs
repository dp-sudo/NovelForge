use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::skill_registry::{validate_skill_id, SkillManifest, SkillManifestPatch};
use crate::state::AppState;

fn skills_lock_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("SKILLS_LOCK_FAILED", "技能注册表锁定失败", false).with_detail(err.to_string())
}

fn skill_not_found_error(id: &str) -> AppErrorDto {
    AppErrorDto::new("SKILLS_NOT_FOUND", &format!("未找到技能 '{}'", id), true)
}

// ── List all skills ──

#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillManifest>, AppErrorDto> {
    state
        .skill_registry
        .read()
        .map_err(skills_lock_error)?
        .list_skills()
}

// ── Get a single skill manifest ──

#[tauri::command]
pub async fn get_skill(
    id: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    validate_skill_id(&id)?;
    state
        .skill_registry
        .read()
        .map_err(skills_lock_error)?
        .get_skill(&id)?
        .ok_or_else(|| skill_not_found_error(&id))
}

// ── Get full .md content of a skill (for editing) ──

#[tauri::command]
pub async fn get_skill_content(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    validate_skill_id(&id)?;
    state
        .skill_registry
        .read()
        .map_err(skills_lock_error)?
        .read_skill_content(&id)?
        .ok_or_else(|| skill_not_found_error(&id))
}

// ── Create a new skill ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSkillInput {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub icon: Option<String>,
    pub body: String,
}

#[tauri::command]
pub async fn create_skill(
    input: CreateSkillInput,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    let now = crate::infra::time::now_iso();
    let manifest = SkillManifest {
        id: input.id,
        name: input.name,
        description: input.description,
        version: 1,
        source: "user".to_string(),
        category: input.category.unwrap_or_else(|| "utility".to_string()),
        tags: input.tags.unwrap_or_default(),
        input_schema: serde_json::json!({"type": "object"}),
        output_schema: serde_json::json!({"type": "object"}),
        requires_user_confirmation: true,
        writes_to_project: false,
        author: Some("User".to_string()),
        icon: input.icon,
        created_at: now.clone(),
        updated_at: now,
        skill_class: None,
        bundle_ids: Vec::new(),
        always_on: false,
        trigger_conditions: Vec::new(),
        required_contexts: Vec::new(),
        state_writes: Vec::new(),
        automation_tier: None,
        scene_tags: Vec::new(),
        affects_layers: Vec::new(),
        task_route: None,
    };

    let reg = state.skill_registry.write().map_err(skills_lock_error)?;
    reg.create_skill(&manifest, &input.body)?;
    Ok(manifest)
}

// ── Update an existing skill's content and/or manifest metadata ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSkillInput {
    pub id: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub manifest: Option<SkillManifestPatch>,
}

#[tauri::command]
pub async fn update_skill(
    input: UpdateSkillInput,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    validate_skill_id(&input.id)?;
    let reg = state.skill_registry.write().map_err(skills_lock_error)?;
    reg.update_skill(&input.id, input.body.as_deref(), input.manifest)
}

// ── Delete a skill (user/imported only) ──

#[tauri::command]
pub async fn delete_skill(id: String, state: State<'_, AppState>) -> Result<(), AppErrorDto> {
    validate_skill_id(&id)?;
    let reg = state.skill_registry.write().map_err(skills_lock_error)?;
    reg.delete_skill(&id)
}

// ── Import a .md file from external path ──

#[tauri::command]
pub async fn import_skill_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    let reg = state.skill_registry.write().map_err(skills_lock_error)?;
    reg.import_file(&file_path)
}

// ── Reset a built-in skill to original ──

#[tauri::command]
pub async fn reset_builtin_skill(
    id: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    validate_skill_id(&id)?;
    let reg = state.skill_registry.write().map_err(skills_lock_error)?;
    reg.reset_builtin(&id)
}

// ── Refresh/re-scan skills directory ──

#[tauri::command]
pub async fn refresh_skills(state: State<'_, AppState>) -> Result<Vec<SkillManifest>, AppErrorDto> {
    let reg = state.skill_registry.write().map_err(skills_lock_error)?;
    reg.reload()?;
    reg.list_skills()
}
