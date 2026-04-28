use tauri::State;
use uuid::Uuid;
use serde::Serialize;
use tauri_plugin_updater::UpdaterExt;

use crate::adapters::llm_types::{ProviderConfig, TaskRoute};
use crate::errors::AppErrorDto;
use crate::infra::app_database;
use crate::services::settings_service::EditorSettings;
use crate::state::AppState;

fn canonical_task_type(task_type: &str) -> &str {
    match task_type {
        "chapter_draft" | "generate_chapter_draft" | "draft" => "chapter.draft",
        "chapter_continue" | "continue_chapter" | "continue_draft" => "chapter.continue",
        "chapter_rewrite" | "rewrite_selection" => "chapter.rewrite",
        "chapter_plan" | "plan_chapter" => "chapter.plan",
        "prose_naturalize" | "deai_text" => "prose.naturalize",
        "character_create" => "character.create",
        "world.generate" | "world_create_rule" => "world.create_rule",
        "plot.generate" | "plot_create_node" => "plot.create_node",
        "consistency_scan" => "consistency.scan",
        "blueprint_generate" => "blueprint.generate_step",
        _ => task_type,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub target_version: Option<String>,
    pub body: Option<String>,
    pub date: Option<String>,
}

#[tauri::command]
pub async fn get_license_status(
    state: State<'_, AppState>,
) -> Result<crate::services::license_service::LicenseStatus, AppErrorDto> {
    state.license_service.get_status()
}

#[tauri::command]
pub async fn activate_license(
    license_key: String,
    state: State<'_, AppState>,
) -> Result<crate::services::license_service::LicenseStatus, AppErrorDto> {
    state.license_service.activate(&license_key)
}

#[tauri::command]
pub async fn check_app_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<AppUpdateInfo, AppErrorDto> {
    let _ = state;
    let updater = app.updater().map_err(|err| {
        AppErrorDto::new("UPDATER_INIT_FAILED", "Cannot initialize updater", false)
            .with_detail(err.to_string())
    })?;
    let current_version = app.package_info().version.to_string();
    let maybe_update = updater.check().await.map_err(|err| {
        AppErrorDto::new("UPDATER_CHECK_FAILED", "Cannot check updates", true)
            .with_detail(err.to_string())
    })?;

    if let Some(update) = maybe_update {
        return Ok(AppUpdateInfo {
            available: true,
            current_version,
            target_version: Some(update.version.to_string()),
            body: update.body.clone(),
            date: update.date.as_ref().map(|value| value.to_string()),
        });
    }

    Ok(AppUpdateInfo {
        available: false,
        current_version,
        target_version: None,
        body: None,
        date: None,
    })
}

#[tauri::command]
pub async fn install_app_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<AppUpdateInfo, AppErrorDto> {
    let _ = state;
    let updater = app.updater().map_err(|err| {
        AppErrorDto::new("UPDATER_INIT_FAILED", "Cannot initialize updater", false)
            .with_detail(err.to_string())
    })?;
    let current_version = app.package_info().version.to_string();
    let maybe_update = updater.check().await.map_err(|err| {
        AppErrorDto::new("UPDATER_CHECK_FAILED", "Cannot check updates", true)
            .with_detail(err.to_string())
    })?;
    let Some(update) = maybe_update else {
        return Ok(AppUpdateInfo {
            available: false,
            current_version,
            target_version: None,
            body: None,
            date: None,
        });
    };

    update
        .download_and_install(
            |_chunk_length, _content_length| {},
            || {},
        )
        .await
        .map_err(|err| {
            AppErrorDto::new("UPDATER_INSTALL_FAILED", "Cannot download or install update", true)
                .with_detail(err.to_string())
        })?;

    Ok(AppUpdateInfo {
        available: true,
        current_version,
        target_version: Some(update.version.to_string()),
        body: update.body.clone(),
        date: update.date.as_ref().map(|value| value.to_string()),
    })
}

#[tauri::command]
pub async fn list_providers(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderConfig>, AppErrorDto> {
    state.settings_service.list_providers()
}

#[tauri::command]
pub async fn save_provider(
    config: ProviderConfig,
    api_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<ProviderConfig, AppErrorDto> {
    let saved = state.settings_service.save_provider(config, api_key)?;
    crate::infra::logger::log_security("save_provider", &format!("provider={}", saved.display_name));
    state.ai_service.reload_provider(&saved.id).await?;

    // Auto-create default task routes for every task type if not already configured
    if let Some(ref default_model) = saved.default_model {
        if !default_model.is_empty() {
            let conn = app_database::open_or_create()?;
            let existing_routes = app_database::load_task_routes(&conn)?;
            let now = crate::infra::time::now_iso();
            let task_types = [
                "chapter.draft",
                "chapter.continue",
                "chapter.rewrite",
                "chapter.plan",
                "prose.naturalize",
                "character.create",
                "world.create_rule",
                "consistency.scan",
                "blueprint.generate_step",
                "plot.create_node",
            ];
            for tt in &task_types {
                if !existing_routes
                    .iter()
                    .any(|r| canonical_task_type(&r.task_type) == *tt)
                {
                    let route = TaskRoute {
                        id: Uuid::new_v4().to_string(),
                        task_type: tt.to_string(),
                        provider_id: saved.id.clone(),
                        model_id: default_model.clone(),
                        fallback_provider_id: None,
                        fallback_model_id: None,
                        max_retries: 1,
                        created_at: Some(now.clone()),
                        updated_at: Some(now.clone()),
                    };
                    app_database::upsert_task_route(&conn, &route, &now)?;
                }
            }
        }
    }

    Ok(saved)
}

#[tauri::command]
pub async fn load_provider(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<ProviderConfig, AppErrorDto> {
    state.settings_service.load_provider(&provider_id)
}

#[tauri::command]
pub async fn delete_provider(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    crate::infra::logger::log_security("delete_provider", &format!("provider_id={}", provider_id));
    state.settings_service.delete_provider(&provider_id)?;
    state.ai_service.unregister_provider(&provider_id).await;
    Ok(())
}

#[tauri::command]
pub async fn test_provider_connection(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    state.settings_service.test_connection(&provider_id).await
}

// ── Model registry commands ──

#[tauri::command]
pub async fn refresh_provider_models(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<crate::services::model_registry_service::RefreshResult, AppErrorDto> {
    state
        .model_registry_service
        .refresh_provider_models(&provider_id)
        .await
}

#[tauri::command]
pub async fn get_provider_models(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::adapters::llm_types::ModelRecord>, AppErrorDto> {
    state.model_registry_service.get_models(&provider_id)
}

#[tauri::command]
pub async fn get_refresh_logs(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::adapters::llm_types::RefreshLog>, AppErrorDto> {
    state.model_registry_service.get_refresh_logs(&provider_id)
}

// ── Task route commands ──

#[tauri::command]
pub async fn list_task_routes(state: State<'_, AppState>) -> Result<Vec<TaskRoute>, AppErrorDto> {
    let conn = app_database::open_or_create()?;
    app_database::load_task_routes(&conn)
}

#[tauri::command]
pub async fn save_task_route(
    route: TaskRoute,
    state: State<'_, AppState>,
) -> Result<TaskRoute, AppErrorDto> {
    let now = crate::infra::time::now_iso();
    let conn = app_database::open_or_create()?;
    let mut r = route;
    if r.id.is_empty() {
        r.id = Uuid::new_v4().to_string();
    }
    app_database::upsert_task_route(&conn, &r, &now)?;
    r.created_at = Some(now.clone());
    r.updated_at = Some(now);
    Ok(r)
}

#[tauri::command]
pub async fn delete_task_route(
    route_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    let conn = app_database::open_or_create()?;
    app_database::delete_task_route(&conn, &route_id)
}

// ── Remote registry commands ──

#[tauri::command]
pub async fn check_remote_registry(
    url: String,
    state: State<'_, AppState>,
) -> Result<crate::services::model_registry_service::RegistryCheckResult, AppErrorDto> {
    state
        .model_registry_service
        .check_remote_registry(&url)
        .await
}

#[tauri::command]
pub async fn apply_registry_update(
    url: String,
    state: State<'_, AppState>,
) -> Result<crate::services::model_registry_service::RegistryApplyResult, AppErrorDto> {
    state
        .model_registry_service
        .apply_registry_update(&url)
        .await
}

// ── Editor settings commands ──

#[tauri::command]
pub async fn load_editor_settings(
    state: State<'_, AppState>,
) -> Result<EditorSettings, AppErrorDto> {
    state.settings_service.load_editor_settings()
}

#[tauri::command]
pub async fn save_editor_settings(
    settings: EditorSettings,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.settings_service.save_editor_settings(&settings)
}

// ── Legacy backward-compatible wrappers ──

#[tauri::command]
pub async fn load_provider_config(
    _project_root: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppErrorDto> {
    let providers = state.settings_service.list_providers()?;
    serde_json::to_value(providers).map_err(|e| {
        AppErrorDto::new(
            "SERIALIZE_ERROR",
            "Cannot serialize provider configs",
            false,
        )
        .with_detail(e.to_string())
    })
}

#[tauri::command]
pub async fn save_provider_config(
    _project_root: String,
    input: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    let config: ProviderConfig = serde_json::from_value(input).map_err(|e| {
        AppErrorDto::new("INVALID_INPUT", "Invalid provider config format", true)
            .with_detail(e.to_string())
    })?;
    let saved = state.settings_service.save_provider(config, None)?;
    state.ai_service.reload_provider(&saved.id).await?;
    Ok(())
}
