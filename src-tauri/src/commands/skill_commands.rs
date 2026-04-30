use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::skill_registry::{SkillManifest, SkillManifestPatch};
use crate::state::AppState;

// ── List all skills ──

#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillManifest>, AppErrorDto> {
    state
        .skill_registry
        .read()
        .map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                .with_detail(e.to_string())
        })?
        .list_skills()
}

// ── Get a single skill manifest ──

#[tauri::command]
pub async fn get_skill(
    id: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    state
        .skill_registry
        .read()
        .map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                .with_detail(e.to_string())
        })?
        .get_skill(&id)?
        .ok_or_else(|| {
            AppErrorDto::new(
                "SKILLS_NOT_FOUND",
                &format!("Skill '{}' not found", id),
                true,
            )
        })
}

// ── Get full .md content of a skill (for editing) ──

#[tauri::command]
pub async fn get_skill_content(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state
        .skill_registry
        .read()
        .map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                .with_detail(e.to_string())
        })?
        .read_skill_content(&id)?
        .ok_or_else(|| {
            AppErrorDto::new(
                "SKILLS_NOT_FOUND",
                &format!("Skill '{}' not found", id),
                true,
            )
        })
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

    let reg = state.skill_registry.read().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
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
    let reg = state.skill_registry.read().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.update_skill(&input.id, input.body.as_deref(), input.manifest)
}

// ── Delete a skill (user/imported only) ──

#[tauri::command]
pub async fn delete_skill(id: String, state: State<'_, AppState>) -> Result<(), AppErrorDto> {
    let reg = state.skill_registry.write().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.delete_skill(&id)
}

// ── Import a .md file from external path ──

#[tauri::command]
pub async fn import_skill_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    let reg = state.skill_registry.read().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.import_file(&file_path)
}

// ── Reset a built-in skill to original ──

#[tauri::command]
pub async fn reset_builtin_skill(
    id: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    let reg = state.skill_registry.read().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.reset_builtin(&id)
}

// ── Refresh/re-scan skills directory ──

#[tauri::command]
pub async fn refresh_skills(state: State<'_, AppState>) -> Result<Vec<SkillManifest>, AppErrorDto> {
    let reg = state.skill_registry.write().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.reload()?;
    reg.list_skills()
}
