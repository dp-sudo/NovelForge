use tauri::State;

use crate::errors::AppErrorDto;
use crate::services::skill_registry::SkillManifest;
use crate::state::AppState;

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
        prompt_strategy: "replace".to_string(),
        author: Some("User".to_string()),
        icon: input.icon,
        created_at: now.clone(),
        updated_at: now,
        task_route: None,
    };

    let reg = state.skill_registry.read().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.create_skill(&manifest, &input.body)?;
    Ok(manifest)
}

#[tauri::command]
pub async fn update_skill(
    id: String,
    body: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto> {
    let reg = state.skill_registry.read().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.update_skill(&id, &body)
}

#[tauri::command]
pub async fn delete_skill(id: String, state: State<'_, AppState>) -> Result<(), AppErrorDto> {
    let reg = state.skill_registry.write().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.delete_skill(&id)
}

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

#[tauri::command]
pub async fn refresh_skills(state: State<'_, AppState>) -> Result<Vec<SkillManifest>, AppErrorDto> {
    let reg = state.skill_registry.write().map_err(|e| {
        AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
            .with_detail(e.to_string())
    })?;
    reg.reload()?;
    reg.list_skills()
}
