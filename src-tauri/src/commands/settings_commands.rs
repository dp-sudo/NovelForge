use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use tauri::State;
use tauri_plugin_updater::UpdaterExt;
use uuid::Uuid;

use crate::adapters::llm_types::{ModelPoolEntry, ModelPoolRecord, ProviderConfig, TaskRoute};
use crate::domain::routing_strategy::RoutingStrategyTemplate;
use crate::errors::AppErrorDto;
use crate::infra::app_database;
use crate::infra::app_database::PromotionPolicyRecord;
use crate::services::promotion_service::PromotionService;
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

fn ensure_updater_configured() -> Result<(), AppErrorDto> {
    let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json"))
        .map_err(|err| {
            AppErrorDto::new("UPDATER_CONFIG_INVALID", "更新器配置无效", false)
                .with_detail(err.to_string())
        })?;

    let updater = conf
        .get("plugins")
        .and_then(|plugins| plugins.get("updater"))
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            AppErrorDto::new("UPDATER_NOT_CONFIGURED", "更新器未配置", true)
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
        return Err(
            AppErrorDto::new("UPDATER_NOT_CONFIGURED", "更新器配置不完整", true)
                .with_suggested_action("请先配置真实 pubkey 与更新端点后再检查更新"),
        );
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

fn no_update_info(current_version: String) -> AppUpdateInfo {
    AppUpdateInfo {
        available: false,
        current_version,
        target_version: None,
        body: None,
        date: None,
    }
}

const DEPRECATED_LOAD_PROVIDER_LOG: &str =
    "[DEPRECATED_COMMAND] load_provider is compatibility-only";
const DEPRECATED_LOAD_PROVIDER_CONFIG_LOG: &str =
    "[DEPRECATED_COMMAND] load_provider_config is compatibility-only";
const DEPRECATED_SAVE_PROVIDER_CONFIG_LOG: &str =
    "[DEPRECATED_COMMAND] save_provider_config is compatibility-only";

fn deprecated_source(source: Option<&str>) -> &str {
    source.unwrap_or("unknown")
}

fn log_deprecated_command(message: &str, command: &str, source: Option<&str>) {
    let src = deprecated_source(source);
    log::warn!("{} source={}", message, src);
    crate::infra::logger::log_user_action(
        "compatibility_bridge.used",
        &format!("command={} source={}", command, src),
    );
    crate::infra::logger::record_deprecated_command_usage(command, src);
}

fn invalid_input_error(message: &'static str) -> AppErrorDto {
    AppErrorDto::new("INVALID_INPUT", message, true)
}

fn serialize_provider_config_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("SERIALIZE_ERROR", "无法序列化供应商配置", false).with_detail(err.to_string())
}

fn parse_provider_config_error(err: impl ToString) -> AppErrorDto {
    invalid_input_error("供应商配置格式无效").with_detail(err.to_string())
}

fn collect_task_route_delete_ids(routes: &[TaskRoute], route_id: &str) -> Vec<String> {
    let Some(canonical_target) = routes
        .iter()
        .find(|route| route.id == route_id)
        .map(|route| task_routing::canonical_task_type(&route.task_type).into_owned())
    else {
        return Vec::new();
    };

    let mut delete_ids = vec![route_id.to_string()];
    let mut seen_ids = HashSet::from([route_id.to_string()]);
    delete_ids.extend(
        routes
            .iter()
            .filter(|route| {
                route.id != route_id
                    && task_routing::canonical_task_type(&route.task_type).as_ref()
                        == canonical_target
                    && seen_ids.insert(route.id.clone())
            })
            .map(|route| route.id.clone()),
    );
    delete_ids
}

fn updater_init_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATER_INIT_FAILED", "无法初始化更新器", false).with_detail(err.to_string())
}

fn updater_check_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATER_CHECK_FAILED", "无法检查更新", true).with_detail(err.to_string())
}

fn updater_install_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATER_INSTALL_FAILED", "无法下载或安装更新", true)
        .with_detail(err.to_string())
}

async fn save_provider_and_reload(
    state: &State<'_, AppState>,
    config: ProviderConfig,
    api_key: Option<String>,
) -> Result<ProviderConfig, AppErrorDto> {
    let saved = state.settings_service.save_provider(config, api_key)?;
    state.ai_service.reload_provider(&saved.id).await?;
    Ok(saved)
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
    _state: State<'_, AppState>,
) -> Result<AppUpdateInfo, AppErrorDto> {
    ensure_updater_configured()?;
    let updater = app.updater().map_err(updater_init_error)?;
    let current_version = app.package_info().version.to_string();
    let maybe_update = updater.check().await.map_err(updater_check_error)?;

    if let Some(update) = maybe_update {
        return Ok(AppUpdateInfo {
            available: true,
            current_version,
            target_version: Some(update.version.to_string()),
            body: update.body.clone(),
            date: update.date.as_ref().map(|value| value.to_string()),
        });
    }

    Ok(no_update_info(current_version))
}

#[tauri::command]
pub async fn install_app_update(
    app: tauri::AppHandle,
    _state: State<'_, AppState>,
) -> Result<AppUpdateInfo, AppErrorDto> {
    ensure_updater_configured()?;
    let updater = app.updater().map_err(updater_init_error)?;
    let current_version = app.package_info().version.to_string();
    let maybe_update = updater.check().await.map_err(updater_check_error)?;
    let Some(update) = maybe_update else {
        return Ok(no_update_info(current_version));
    };

    update
        .download_and_install(|_chunk_length, _content_length| {}, || {})
        .await
        .map_err(updater_install_error)?;

    Ok(AppUpdateInfo {
        available: true,
        current_version,
        target_version: Some(update.version.to_string()),
        body: update.body.clone(),
        date: update.date.as_ref().map(|value| value.to_string()),
    })
}

#[tauri::command]
pub async fn get_deprecated_command_usage_report(
) -> Result<Vec<crate::infra::logger::DeprecatedCommandUsageEntry>, AppErrorDto> {
    Ok(crate::infra::logger::read_deprecated_command_usage())
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
    let saved = save_provider_and_reload(&state, config, api_key).await?;
    crate::infra::logger::log_security(
        "save_provider",
        &format!("provider={}", saved.display_name),
    );

    Ok(saved)
}

#[tauri::command]
pub async fn load_provider(
    provider_id: String,
    source: Option<String>,
    state: State<'_, AppState>,
) -> Result<ProviderConfig, AppErrorDto> {
    // 问题4修复(Deprecated 命令面): 兼容入口保留，但官方调用面改为 settingsApi.list_providers/save_provider。
    log_deprecated_command(
        DEPRECATED_LOAD_PROVIDER_LOG,
        "load_provider",
        source.as_deref(),
    );
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

// ── Model pool commands ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelPoolInput {
    pub name: String,
    pub pool_type: String,
    pub models: Vec<ModelPoolEntry>,
}

#[tauri::command]
pub async fn list_model_pools(
    _state: State<'_, AppState>,
) -> Result<Vec<ModelPoolRecord>, AppErrorDto> {
    crate::services::ai_service::AiService::list_model_pools()
}

#[tauri::command]
pub async fn create_model_pool(
    input: CreateModelPoolInput,
    _state: State<'_, AppState>,
) -> Result<ModelPoolRecord, AppErrorDto> {
    crate::services::ai_service::AiService::create_model_pool(
        &input.name,
        &input.pool_type,
        input.models,
    )
}

#[tauri::command]
pub async fn update_model_pool(
    pool_id: String,
    config: ModelPoolRecord,
    _state: State<'_, AppState>,
) -> Result<ModelPoolRecord, AppErrorDto> {
    crate::services::ai_service::AiService::update_model_pool(&pool_id, config)
}

#[tauri::command]
pub async fn delete_model_pool(
    pool_id: String,
    _state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    crate::services::ai_service::AiService::delete_model_pool(&pool_id)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendRoutingStrategyInput {
    pub project_root: String,
    pub project_stage: Option<String>,
    pub task_type: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRoutingStrategyTemplateInput {
    pub project_root: String,
    pub strategy_id: String,
}

#[tauri::command]
pub async fn recommend_routing_strategy(
    input: RecommendRoutingStrategyInput,
    _state: State<'_, AppState>,
) -> Result<Vec<RoutingStrategyTemplate>, AppErrorDto> {
    crate::services::ai_service::AiService::recommend_routing_strategy(
        &input.project_root,
        input.project_stage.as_deref(),
        input.task_type.as_deref(),
    )
}

#[tauri::command]
pub async fn apply_routing_strategy_template(
    input: ApplyRoutingStrategyTemplateInput,
    _state: State<'_, AppState>,
) -> Result<Vec<TaskRoute>, AppErrorDto> {
    crate::services::ai_service::AiService::apply_routing_strategy_template(
        &input.project_root,
        &input.strategy_id,
    )
}

#[tauri::command]
pub async fn get_project_routing_strategy(
    project_root: String,
    _state: State<'_, AppState>,
) -> Result<Option<String>, AppErrorDto> {
    crate::services::ai_service::AiService::get_project_routing_strategy_id(&project_root)
}

// ── Task route commands ──

#[tauri::command]
pub async fn list_task_routes(_state: State<'_, AppState>) -> Result<Vec<TaskRoute>, AppErrorDto> {
    let conn = app_database::open_or_create()?;
    let routes = app_database::load_task_routes(&conn)?;
    // 问题4修复(读写分离): list_task_routes 为纯读接口，不再触发默认路由写入。
    Ok(normalize_task_routes(routes))
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
    let fallback_provider = r
        .fallback_provider_id
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let fallback_model = r
        .fallback_model_id
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    r.fallback_provider_id = fallback_provider;
    r.fallback_model_id = if r.fallback_provider_id.is_some() {
        fallback_model
    } else {
        None
    };
    r.model_pool_id = r
        .model_pool_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    r.fallback_model_pool_id = r
        .fallback_model_pool_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    r.post_tasks = r
        .post_tasks
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .fold(Vec::<String>::new(), |mut acc, value| {
            if !acc
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(value.as_str()))
            {
                acc.push(value);
            }
            acc
        });

    let existing_routes = app_database::load_task_routes(&conn)?;
    let existing_same_task = existing_routes.iter().find(|existing| {
        task_routing::canonical_task_type(&existing.task_type).as_ref() == r.task_type
    });
    if r.model_pool_id.is_some() {
        if r.provider_id.is_empty() {
            r.provider_id = existing_same_task
                .map(|existing| existing.provider_id.clone())
                .unwrap_or_default();
        }
        if r.model_id.is_empty() {
            r.model_id = existing_same_task
                .map(|existing| existing.model_id.clone())
                .unwrap_or_default();
        }
    }
    if r.provider_id.is_empty() {
        return Err(invalid_input_error("供应商不能为空"));
    }
    if r.model_id.is_empty() {
        return Err(invalid_input_error("模型ID不能为空"));
    }
    if let Some(existing) = existing_same_task {
        if r.model_pool_id.is_none() {
            r.model_pool_id = existing.model_pool_id.clone();
        }
        if r.fallback_model_pool_id.is_none() {
            r.fallback_model_pool_id = existing.fallback_model_pool_id.clone();
        }
    }
    match (r.id.is_empty(), existing_same_task) {
        (true, Some(existing)) => {
            r.id = existing.id.clone();
            r.created_at = existing.created_at.clone();
        }
        (true, None) => {
            r.id = Uuid::new_v4().to_string();
            r.created_at = Some(now.clone());
        }
        (false, Some(existing)) => {
            if existing.id != r.id {
                return Err(AppErrorDto::new(
                    "TASK_ROUTE_DUPLICATE",
                    &format!("任务类型 '{}' 已存在路由配置", r.task_type),
                    true,
                ));
            }
            r.created_at = existing.created_at.clone();
        }
        (false, None) => {}
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
    for delete_route_id in collect_task_route_delete_ids(&routes, &route_id) {
        app_database::delete_task_route(&conn, &delete_route_id)?;
    }
    Ok(())
}

// ── Promotion policy commands ──

#[tauri::command]
pub async fn list_promotion_policies(
    _state: State<'_, AppState>,
) -> Result<Vec<PromotionPolicyRecord>, AppErrorDto> {
    PromotionService.list_policies()
}

#[tauri::command]
pub async fn save_promotion_policy(
    policy: PromotionPolicyRecord,
    _state: State<'_, AppState>,
) -> Result<PromotionPolicyRecord, AppErrorDto> {
    PromotionService.save_policy(policy)
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
    source: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppErrorDto> {
    // 问题4修复(Deprecated 命令面): 兼容旧协议，后续由 list_providers 收敛替代。
    log_deprecated_command(
        DEPRECATED_LOAD_PROVIDER_CONFIG_LOG,
        "load_provider_config",
        source.as_deref(),
    );
    let providers = state.settings_service.list_providers()?;
    serde_json::to_value(providers).map_err(serialize_provider_config_error)
}

#[tauri::command]
pub async fn save_provider_config(
    _project_root: String,
    input: serde_json::Value,
    source: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    // 问题4修复(Deprecated 命令面): 兼容旧协议，后续由 save_provider 收敛替代。
    log_deprecated_command(
        DEPRECATED_SAVE_PROVIDER_CONFIG_LOG,
        "save_provider_config",
        source.as_deref(),
    );
    let mut config: ProviderConfig =
        serde_json::from_value(input).map_err(parse_provider_config_error)?;
    let api_key = config.api_key.take();
    save_provider_and_reload(&state, config, api_key).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task_route(id: &str, task_type: &str) -> TaskRoute {
        TaskRoute {
            id: id.to_string(),
            task_type: task_type.to_string(),
            provider_id: "provider".to_string(),
            model_id: "model".to_string(),
            fallback_provider_id: None,
            fallback_model_id: None,
            model_pool_id: None,
            fallback_model_pool_id: None,
            post_tasks: Vec::new(),
            max_retries: 1,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn collect_task_route_delete_ids_returns_empty_when_route_missing() {
        let routes = vec![make_task_route("route-a", "chapter.draft")];
        assert!(collect_task_route_delete_ids(&routes, "missing-route").is_empty());
    }

    #[test]
    fn collect_task_route_delete_ids_keeps_target_first_and_collects_aliases() {
        let routes = vec![
            make_task_route("alias-1", "generate_chapter_draft"),
            make_task_route("target", "chapter.draft"),
            make_task_route("other", "chapter.continue"),
            make_task_route("alias-2", "draft"),
        ];

        let delete_ids = collect_task_route_delete_ids(&routes, "target");
        assert_eq!(
            delete_ids,
            vec![
                "target".to_string(),
                "alias-1".to_string(),
                "alias-2".to_string(),
            ]
        );
    }

    #[test]
    fn collect_task_route_delete_ids_deduplicates_repeated_alias_ids() {
        let routes = vec![
            make_task_route("target", "chapter.draft"),
            make_task_route("alias-dup", "draft"),
            make_task_route("alias-dup", "generate_chapter_draft"),
        ];

        let delete_ids = collect_task_route_delete_ids(&routes, "target");
        assert_eq!(
            delete_ids,
            vec!["target".to_string(), "alias-dup".to_string()]
        );
    }
}
