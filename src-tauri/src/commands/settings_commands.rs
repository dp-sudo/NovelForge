use serde::Serialize;
use tauri::State;
use tauri_plugin_updater::UpdaterExt;
use uuid::Uuid;

use crate::adapters::llm_types::{ProviderConfig, TaskRoute};
use crate::errors::AppErrorDto;
use crate::infra::app_database;
use crate::services::settings_service::{self, EditorSettings};
use crate::services::task_routing;
use crate::state::AppState;


fn ensure_updater_configured() -> Result<(), AppErrorDto> {
    let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json"))
        .map_err(|err| {
            AppErrorDto::new("UPDATER_CONFIG_INVALID", "Updater config is invalid", false)
                .with_detail(err.to_string())
        })?;

    let updater = conf
        .get("plugins")
        .and_then(|plugins| plugins.get("updater"))
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            AppErrorDto::new("UPDATER_NOT_CONFIGURED", "Updater is not configured", true)
                .with_suggested_action("请在 tauri.conf.json 中配置 updater 插件")
        })?;

    let pubkey = updater
        .get("pubkey")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim();
    let endpoints = updater
        .get("endpoints")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let placeholder_pubkey = pubkey.is_empty()
        || pubkey
            .chars()
            .all(|ch| ch == 'A' || ch == '=' || ch.is_whitespace());
    let has_valid_endpoint = endpoints
        .iter()
        .filter_map(|value| value.as_str())
        .any(|value| !value.trim().is_empty());

    if placeholder_pubkey || !has_valid_endpoint {
        return Err(AppErrorDto::new(
            "UPDATER_NOT_CONFIGURED",
            "Updater config is incomplete",
            true,
        )
        .with_suggested_action("请先配置真实 pubkey 与更新端点后再检查更新"));
    }

    Ok(())
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
) -> Result<AppUpdateInfo, AppErrorDto> {
    ensure_updater_configured()?;
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
) -> Result<AppUpdateInfo, AppErrorDto> {
    ensure_updater_configured()?;
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

    if let Some(ref default_model) = saved.default_model {
        if !default_model.is_empty() {
            let conn = app_database::open_or_create()?;
            let existing_routes = app_database::load_task_routes(&conn)?;
            let now = crate::infra::time::now_iso();
            let task_types = task_routing::task_route_types_with_custom();
            for tt in task_types {
                if !existing_routes
                    .iter()
                    .any(|r| task_routing::canonical_task_type(&r.task_type).as_ref() == tt)
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

#[tauri::command]
pub async fn list_task_routes() -> Result<Vec<TaskRoute>, AppErrorDto> {
    let conn = app_database::open_or_create()?;
    let providers = app_database::load_all_providers(&conn)?;
    let routes = app_database::load_task_routes(&conn)?;
    settings_service::ensure_default_task_routes(&conn, &providers, routes)
}

#[tauri::command]
pub async fn save_task_route(
    route: TaskRoute,
    _state: State<'_, AppState>,
) -> Result<TaskRoute, AppErrorDto> {
    let now = crate::infra::time::now_iso();
    let conn = app_database::open_or_create()?;
    let mut r = route;
    r.task_type = task_routing::canonical_task_type(&r.task_type).into_owned();
    r.provider_id = r.provider_id.trim().to_string();
    r.model_id = r.model_id.trim().to_string();
    r.max_retries = r.max_retries.clamp(1, 8);
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
    _state: State<'_, AppState>,
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

