use rusqlite::Connection;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use tauri::State;
use tauri_plugin_updater::UpdaterExt;
use uuid::Uuid;

use crate::adapters::llm_types::{ProviderConfig, TaskRoute};
use crate::errors::AppErrorDto;
use crate::infra::app_database;
use crate::services::settings_service::EditorSettings;
use crate::services::task_routing;
use crate::state::AppState;

fn normalize_task_routes(routes: Vec<TaskRoute>) -> Vec<TaskRoute> {
    let mut dedup: BTreeMap<String, (bool, TaskRoute)> = BTreeMap::new();
    for mut route in routes {
        let canonical = task_routing::canonical_task_type(&route.task_type).into_owned();
        let is_exact_canonical = route.task_type == canonical;
        route.task_type = canonical.clone();

        let should_replace = match dedup.get(&canonical) {
            None => true,
            Some((existing_exact, _)) => is_exact_canonical && !*existing_exact,
        };
        if should_replace {
            dedup.insert(canonical, (is_exact_canonical, route));
        }
    }
    dedup.into_values().map(|(_, route)| route).collect()
}

fn pick_primary_route_seed(
    routes: &[TaskRoute],
    providers: &[ProviderConfig],
) -> Option<(String, String)> {
    const PROVIDER_SEED_PRIORITY: &[&str] = &[
        "deepseek",
        "kimi",
        "zhipu",
        "minimax",
        "openai",
        "anthropic",
        "gemini",
        "custom",
    ];

    if let Some(existing) = routes
        .iter()
        .find(|route| !route.provider_id.trim().is_empty() && !route.model_id.trim().is_empty())
    {
        return Some((
            existing.provider_id.trim().to_string(),
            existing.model_id.trim().to_string(),
        ));
    }

    for provider_id in PROVIDER_SEED_PRIORITY {
        if let Some(provider) = providers
            .iter()
            .find(|provider| provider.id == *provider_id)
        {
            let model_id = provider.default_model.as_deref().unwrap_or("").trim();
            if !model_id.is_empty() {
                return Some((provider.id.clone(), model_id.to_string()));
            }
        }
    }

    providers.iter().find_map(|provider| {
        let model_id = provider.default_model.as_deref().unwrap_or("").trim();
        if provider.id.trim().is_empty() || model_id.is_empty() {
            None
        } else {
            Some((provider.id.clone(), model_id.to_string()))
        }
    })
}

fn ensure_default_task_routes(
    conn: &Connection,
    providers: &[ProviderConfig],
    routes: Vec<TaskRoute>,
) -> Result<Vec<TaskRoute>, AppErrorDto> {
    let normalized_routes = normalize_task_routes(routes);
    if !normalized_routes.is_empty() {
        return Ok(normalized_routes);
    }
    let existing_task_types: HashSet<String> = normalized_routes
        .iter()
        .map(|route| route.task_type.clone())
        .collect();

    let Some((provider_id, model_id)) = pick_primary_route_seed(&normalized_routes, providers)
    else {
        return Ok(normalized_routes);
    };

    let now = crate::infra::time::now_iso();
    let mut changed = false;
    for task_type in task_routing::TASK_ROUTE_TYPES_WITH_CUSTOM {
        if existing_task_types.contains(*task_type) {
            continue;
        }

        let route = TaskRoute {
            id: Uuid::new_v4().to_string(),
            task_type: (*task_type).to_string(),
            provider_id: provider_id.clone(),
            model_id: model_id.clone(),
            fallback_provider_id: None,
            fallback_model_id: None,
            max_retries: 1,
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };
        app_database::upsert_task_route(conn, &route, &now)?;
        changed = true;
    }

    if !changed {
        return Ok(normalized_routes);
    }

    let refreshed = app_database::load_task_routes(conn)?;
    Ok(normalize_task_routes(refreshed))
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
        .download_and_install(|_chunk_length, _content_length| {}, || {})
        .await
        .map_err(|err| {
            AppErrorDto::new(
                "UPDATER_INSTALL_FAILED",
                "Cannot download or install update",
                true,
            )
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
    crate::infra::logger::log_security(
        "save_provider",
        &format!("provider={}", saved.display_name),
    );
    state.ai_service.reload_provider(&saved.id).await?;

    // Auto-create default task routes for every task type if not already configured
    if let Some(ref default_model) = saved.default_model {
        if !default_model.is_empty() {
            let conn = app_database::open_or_create()?;
            let existing_routes = app_database::load_task_routes(&conn)?;
            let now = crate::infra::time::now_iso();
            let task_types = task_routing::TASK_ROUTE_TYPES_WITH_CUSTOM;
            for tt in task_types {
                if !existing_routes
                    .iter()
                    .any(|r| task_routing::canonical_task_type(&r.task_type).as_ref() == *tt)
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
pub async fn list_task_routes(_state: State<'_, AppState>) -> Result<Vec<TaskRoute>, AppErrorDto> {
    let conn = app_database::open_or_create()?;
    let providers = app_database::load_all_providers(&conn)?;
    let routes = app_database::load_task_routes(&conn)?;
    ensure_default_task_routes(&conn, &providers, routes)
}

#[tauri::command]
pub async fn save_task_route(
    route: TaskRoute,
    state: State<'_, AppState>,
) -> Result<TaskRoute, AppErrorDto> {
    let now = crate::infra::time::now_iso();
    let conn = app_database::open_or_create()?;
    let mut r = route;
    r.task_type = task_routing::canonical_task_type(&r.task_type).into_owned();
    r.provider_id = r.provider_id.trim().to_string();
    r.model_id = r.model_id.trim().to_string();
    r.max_retries = r.max_retries.max(1).min(8);
    r.fallback_provider_id = r
        .fallback_provider_id
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    r.fallback_model_id = r
        .fallback_model_id
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if r.fallback_provider_id.is_none() {
        r.fallback_model_id = None;
    }

    if r.provider_id.is_empty() {
        return Err(AppErrorDto::new("INVALID_INPUT", "Provider 不能为空", true));
    }
    if r.model_id.is_empty() {
        return Err(AppErrorDto::new("INVALID_INPUT", "模型 ID 不能为空", true));
    }

    let existing_routes = app_database::load_task_routes(&conn)?;
    let existing_same_task = existing_routes.iter().find(|existing| {
        task_routing::canonical_task_type(&existing.task_type).as_ref() == r.task_type
    });
    if r.id.is_empty() {
        if let Some(existing) = existing_same_task {
            r.id = existing.id.clone();
            r.created_at = existing.created_at.clone();
        } else {
            r.id = Uuid::new_v4().to_string();
            r.created_at = Some(now.clone());
        }
    } else if let Some(existing) = existing_same_task {
        if existing.id != r.id {
            return Err(AppErrorDto::new(
                "TASK_ROUTE_DUPLICATE",
                &format!("任务类型 '{}' 已存在路由配置", r.task_type),
                true,
            ));
        }
        r.created_at = existing.created_at.clone();
    }

    app_database::upsert_task_route(&conn, &r, &now)?;
    if r.created_at.is_none() {
        r.created_at = Some(now.clone());
    }
    r.updated_at = Some(now);
    Ok(r)
}

#[tauri::command]
pub async fn delete_task_route(
    route_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    let conn = app_database::open_or_create()?;
    let routes = app_database::load_task_routes(&conn)?;
    let target = routes.iter().find(|r| r.id == route_id);
    let Some(target_route) = target else {
        return Ok(());
    };
    let canonical_target = task_routing::canonical_task_type(&target_route.task_type).into_owned();

    app_database::delete_task_route(&conn, &route_id)?;
    for alias_route in routes.iter().filter(|r| {
        r.id != route_id
            && task_routing::canonical_task_type(&r.task_type).as_ref() == canonical_target
    }) {
        app_database::delete_task_route(&conn, &alias_route.id)?;
    }
    Ok(())
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
