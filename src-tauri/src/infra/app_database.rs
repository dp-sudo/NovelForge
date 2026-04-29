//! App-level SQLite database (separate from project databases).
//!
//! Stores provider configurations, model registry, task routes, and app settings.
//! Located at the per-user app data directory (`%LOCALAPPDATA%\\NovelForge` on Windows).

use std::fs;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::path::PathBuf;

use rusqlite::{params, Connection};

use crate::adapters::llm_types::{ModelRecord, ProviderConfig, TaskRoute};
use crate::errors::AppErrorDto;
use crate::services::task_routing;
use uuid::Uuid;

use log::info;

/// Get the path to the app-level database.
pub fn app_database_path() -> Result<PathBuf, AppErrorDto> {
    let app_dir = app_dir()?;
    let db_path = app_dir.join("novelforge.db");
    migrate_legacy_database_if_needed(&db_path)?;
    Ok(db_path)
}

/// Get the app data directory, creating it if needed.
pub fn app_dir() -> Result<PathBuf, AppErrorDto> {
    crate::infra::app_paths::app_data_dir()
}

/// Initialize the app-level database (creates tables if not exists).
pub fn open_or_create() -> Result<Connection, AppErrorDto> {
    let db_path = app_database_path()?;
    let conn = Connection::open(&db_path).map_err(|e| {
        AppErrorDto::new("APP_DB_OPEN_FAILED", "Cannot open app database", true)
            .with_detail(e.to_string())
    })?;
    // Run migrations (will create tables and track versions if not already done)
    let result = crate::infra::migrator::run_app_pending(&conn)?;
    for v in &result.applied {
        log::info!("[DB] Applied app migration: {}", v);
    }
    ensure_schema_compatibility(&conn)?;
    // 问题4修复(单一初始化入口): 默认任务路由仅在 app-db 初始化阶段补齐。
    ensure_default_task_routes_initialized(&conn)?;
    Ok(conn)
}

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
        if let Some(provider) = providers.iter().find(|provider| provider.id == *provider_id) {
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

fn ensure_default_task_routes_initialized(conn: &Connection) -> Result<(), AppErrorDto> {
    let routes = load_task_routes(conn)?;
    let normalized_routes = normalize_task_routes(routes);
    if !normalized_routes.is_empty() {
        return Ok(());
    }

    let providers = load_all_providers(conn)?;
    let Some((provider_id, model_id)) = pick_primary_route_seed(&normalized_routes, &providers) else {
        return Ok(());
    };

    let existing_task_types: HashSet<String> = normalized_routes
        .iter()
        .map(|route| route.task_type.clone())
        .collect();
    let now = crate::infra::time::now_iso();
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
        upsert_task_route(conn, &route, &now)?;
    }

    Ok(())
}

/// Read a single provider config from the app database.
pub fn load_provider(
    conn: &Connection,
    provider_id: &str,
) -> Result<Option<ProviderConfig>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, display_name, vendor, protocol, base_url, endpoint_path,
                api_key_secret_ref, auth_mode, auth_header_name,
                anthropic_version, beta_headers, custom_headers,
                default_model, enabled, timeout_ms, connect_timeout_ms, max_retries,
                model_refresh_mode, models_path, last_model_refresh_at
         FROM llm_providers WHERE id = ?1",
        )
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot read provider", true)
                .with_detail(e.to_string())
        })?;

    let result = stmt.query_row(params![provider_id], |row| {
        let beta_raw: Option<String> = row.get(10).ok();
        let custom_raw: Option<String> = row.get(11).ok();
        Ok(ProviderConfig {
            id: row.get(0)?,
            display_name: row.get(1)?,
            vendor: row.get(2)?,
            protocol: row.get(3)?,
            base_url: row.get(4)?,
            endpoint_path: row.get(5)?,
            api_key: None,
            auth_mode: row.get(7)?,
            auth_header_name: row.get(8)?,
            anthropic_version: row.get(9)?,
            beta_headers: beta_raw.and_then(|s| serde_json::from_str(&s).ok()),
            custom_headers: custom_raw.and_then(|s| serde_json::from_str(&s).ok()),
            default_model: row.get(12)?,
            timeout_ms: row.get(14)?,
            connect_timeout_ms: row.get(15)?,
            max_retries: row.get(16)?,
            model_refresh_mode: row.get(17)?,
            models_path: row.get(18)?,
            last_model_refresh_at: row.get(19)?,
        })
    });

    match result {
        Ok(config) => Ok(Some(config)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(
            AppErrorDto::new("DB_READ_FAILED", "Cannot read provider", true)
                .with_detail(e.to_string()),
        ),
    }
}

/// Read all providers from the app database.
pub fn load_all_providers(conn: &Connection) -> Result<Vec<ProviderConfig>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, display_name, vendor, protocol, base_url, endpoint_path,
                api_key_secret_ref, auth_mode, auth_header_name,
                anthropic_version, beta_headers, custom_headers,
                default_model, enabled, timeout_ms, connect_timeout_ms, max_retries,
                model_refresh_mode, models_path, last_model_refresh_at
         FROM llm_providers ORDER BY display_name",
        )
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot list providers", true)
                .with_detail(e.to_string())
        })?;

    let providers = stmt
        .query_map([], |row| {
            let beta_raw: Option<String> = row.get(10).ok();
            let custom_raw: Option<String> = row.get(11).ok();
            Ok(ProviderConfig {
                id: row.get(0)?,
                display_name: row.get(1)?,
                vendor: row.get(2)?,
                protocol: row.get(3)?,
                base_url: row.get(4)?,
                endpoint_path: row.get(5)?,
                api_key: None,
                auth_mode: row.get(7)?,
                auth_header_name: row.get(8)?,
                anthropic_version: row.get(9)?,
                beta_headers: beta_raw.and_then(|s| serde_json::from_str(&s).ok()),
                custom_headers: custom_raw.and_then(|s| serde_json::from_str(&s).ok()),
                default_model: row.get(12)?,
                timeout_ms: row.get(14)?,
                connect_timeout_ms: row.get(15)?,
                max_retries: row.get(16)?,
                model_refresh_mode: row.get(17)?,
                models_path: row.get(18)?,
                last_model_refresh_at: row.get(19)?,
            })
        })
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot list providers", true)
                .with_detail(e.to_string())
        })?;

    providers.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "Error reading providers", true)
            .with_detail(e.to_string())
    })
}

/// Upsert a provider config into the app database.
pub fn upsert_provider(
    conn: &Connection,
    config: &ProviderConfig,
    now: &str,
) -> Result<(), AppErrorDto> {
    let beta_json = config
        .beta_headers
        .as_ref()
        .and_then(|h| serde_json::to_string(h).ok());
    let custom_json = config
        .custom_headers
        .as_ref()
        .and_then(|h| serde_json::to_string(h).ok());

    conn.execute(
        "INSERT INTO llm_providers (id, display_name, vendor, protocol, base_url, endpoint_path,
         api_key_secret_ref, auth_mode, auth_header_name,
         anthropic_version, beta_headers, custom_headers,
         default_model, enabled, timeout_ms, connect_timeout_ms, max_retries,
         model_refresh_mode, models_path, last_model_refresh_at, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)
         ON CONFLICT(id) DO UPDATE SET
         display_name=excluded.display_name, vendor=excluded.vendor,
         protocol=excluded.protocol, base_url=excluded.base_url,
         endpoint_path=excluded.endpoint_path, api_key_secret_ref=excluded.api_key_secret_ref,
         auth_mode=excluded.auth_mode, auth_header_name=excluded.auth_header_name,
         anthropic_version=excluded.anthropic_version, beta_headers=excluded.beta_headers,
         custom_headers=excluded.custom_headers, default_model=excluded.default_model,
         enabled=excluded.enabled, timeout_ms=excluded.timeout_ms,
         connect_timeout_ms=excluded.connect_timeout_ms, max_retries=excluded.max_retries,
         model_refresh_mode=excluded.model_refresh_mode, models_path=excluded.models_path,
         last_model_refresh_at=excluded.last_model_refresh_at,
         updated_at=excluded.updated_at",
        params![
            config.id,
            config.display_name,
            config.vendor,
            config.protocol,
            config.base_url,
            config.endpoint_path,
            "" as &str,
            config.auth_mode,
            config.auth_header_name,
            config.anthropic_version,
            beta_json,
            custom_json,
            config.default_model,
            true as i32,
            config.timeout_ms as i64,
            config.connect_timeout_ms as i64,
            config.max_retries as i64,
            config
                .model_refresh_mode
                .clone()
                .unwrap_or_else(|| "registry".to_string()),
            config.models_path,
            config.last_model_refresh_at,
            now,
            now
        ],
    )
    .map_err(|e| {
        AppErrorDto::new("DB_WRITE_FAILED", "Cannot save provider", true).with_detail(e.to_string())
    })?;

    Ok(())
}

/// Delete a provider from the app database.
pub fn delete_provider(conn: &Connection, provider_id: &str) -> Result<(), AppErrorDto> {
    conn.execute(
        "DELETE FROM llm_providers WHERE id = ?1",
        params![provider_id],
    )
    .map_err(|e| {
        AppErrorDto::new("DB_DELETE_FAILED", "Cannot delete provider", true)
            .with_detail(e.to_string())
    })?;
    Ok(())
}

// ── llm_models CRUD ──

/// List all models for a provider.
pub fn load_models(conn: &Connection, provider_id: &str) -> Result<Vec<ModelRecord>, AppErrorDto> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, model_name, display_name, context_window_tokens, max_output_tokens,
                supports_streaming, supports_tools, supports_json_object, supports_json_schema,
                supports_thinking, supports_reasoning_effort, supports_prompt_cache,
                status, source, user_overridden, last_seen_at, registry_version, created_at, updated_at
         FROM llm_models WHERE provider_id = ?1 ORDER BY model_name"
    ).map_err(|e| AppErrorDto::new("DB_READ_FAILED", "Cannot load models", true).with_detail(e.to_string()))?;

    let rows = stmt
        .query_map(params![provider_id], |row| {
            Ok(ModelRecord {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                model_name: row.get(2)?,
                display_name: row.get(3)?,
                context_window_tokens: row.get(4)?,
                max_output_tokens: row.get(5)?,
                supports_streaming: row.get::<_, i32>(6)? != 0,
                supports_tools: row.get::<_, i32>(7)? != 0,
                supports_json_object: row.get::<_, i32>(8)? != 0,
                supports_json_schema: row.get::<_, i32>(9)? != 0,
                supports_thinking: row.get::<_, i32>(10)? != 0,
                supports_reasoning_effort: row.get::<_, i32>(11)? != 0,
                supports_prompt_cache: row.get::<_, i32>(12)? != 0,
                status: row.get(13)?,
                source: row.get(14)?,
                user_overridden: row.get::<_, i32>(15)? != 0,
                last_seen_at: row.get(16)?,
                registry_version: row.get(17)?,
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
            })
        })
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot load models", true)
                .with_detail(e.to_string())
        })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "Error reading models", true).with_detail(e.to_string())
    })
}

/// Upsert a model record. Returns true if inserted, false if updated.
pub fn upsert_model(conn: &Connection, model: &ModelRecord) -> Result<bool, AppErrorDto> {
    let existing: bool = conn
        .query_row(
            "SELECT 1 FROM llm_models WHERE provider_id = ?1 AND model_name = ?2",
            params![model.provider_id, model.model_name],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if existing {
        conn.execute(
            "UPDATE llm_models SET display_name=?1, context_window_tokens=?2, max_output_tokens=?3,
             supports_streaming=?4, supports_tools=?5, supports_json_object=?6,
             supports_json_schema=?7, supports_thinking=?8, supports_reasoning_effort=?9,
             supports_prompt_cache=?10, status=?11, source=?12, user_overridden=?13,
             last_seen_at=?14, registry_version=?15, updated_at=?16
             WHERE provider_id=?17 AND model_name=?18",
            params![
                model.display_name,
                model.context_window_tokens,
                model.max_output_tokens,
                model.supports_streaming as i32,
                model.supports_tools as i32,
                model.supports_json_object as i32,
                model.supports_json_schema as i32,
                model.supports_thinking as i32,
                model.supports_reasoning_effort as i32,
                model.supports_prompt_cache as i32,
                model.status,
                model.source,
                model.user_overridden as i32,
                model.last_seen_at,
                model.registry_version,
                model.updated_at,
                model.provider_id,
                model.model_name
            ],
        )
        .map_err(|e| {
            AppErrorDto::new("DB_WRITE_FAILED", "Cannot update model", true)
                .with_detail(e.to_string())
        })?;
        Ok(false)
    } else {
        conn.execute(
            "INSERT INTO llm_models (id, provider_id, model_name, display_name, context_window_tokens,
             max_output_tokens, supports_streaming, supports_tools, supports_json_object,
             supports_json_schema, supports_thinking, supports_reasoning_effort,
             supports_prompt_cache, supports_batch, status, source, user_overridden,
             last_seen_at, registry_version, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,0,?14,?15,?16,?17,?18,?19,?20)",
            params![
                model.id, model.provider_id, model.model_name, model.display_name,
                model.context_window_tokens, model.max_output_tokens,
                model.supports_streaming as i32, model.supports_tools as i32,
                model.supports_json_object as i32, model.supports_json_schema as i32,
                model.supports_thinking as i32, model.supports_reasoning_effort as i32,
                model.supports_prompt_cache as i32, model.status, model.source,
                model.user_overridden as i32, model.last_seen_at, model.registry_version,
                model.created_at, model.updated_at
            ],
        ).map_err(|e| AppErrorDto::new("DB_WRITE_FAILED", "Cannot insert model", true).with_detail(e.to_string()))?;
        Ok(true)
    }
}

/// Insert a refresh log entry.
pub fn insert_refresh_log(
    conn: &Connection,
    log: &crate::adapters::llm_types::RefreshLog,
) -> Result<(), AppErrorDto> {
    conn.execute(
        "INSERT INTO llm_model_refresh_logs (id, provider_id, refresh_type, status,
         models_added, models_updated, models_removed, error_message, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            log.id,
            log.provider_id,
            log.refresh_type,
            log.status,
            log.models_added,
            log.models_updated,
            log.models_removed,
            log.error_message,
            log.created_at
        ],
    )
    .map_err(|e| {
        AppErrorDto::new("DB_WRITE_FAILED", "Cannot insert refresh log", true)
            .with_detail(e.to_string())
    })?;
    Ok(())
}

// ── llm_task_routes CRUD ──

/// Load all task routes.
pub fn load_task_routes(conn: &Connection) -> Result<Vec<TaskRoute>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_type, provider_id, model_id,
                fallback_provider_id, fallback_model_id, max_retries, created_at, updated_at
         FROM llm_task_routes ORDER BY task_type",
        )
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot load task routes", true)
                .with_detail(e.to_string())
        })?;

    let rows = stmt
        .query_map([], |row| {
            Ok(TaskRoute {
                id: row.get(0)?,
                task_type: row.get(1)?,
                provider_id: row.get(2)?,
                model_id: row.get(3)?,
                fallback_provider_id: row.get(4)?,
                fallback_model_id: row.get(5)?,
                max_retries: row.get(6)?,
                created_at: Some(row.get::<_, String>(7)?),
                updated_at: Some(row.get::<_, String>(8)?),
            })
        })
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot load task routes", true)
                .with_detail(e.to_string())
        })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "Error reading task routes", true)
            .with_detail(e.to_string())
    })
}

/// Upsert a task route.
pub fn upsert_task_route(
    conn: &Connection,
    route: &TaskRoute,
    now: &str,
) -> Result<(), AppErrorDto> {
    conn.execute(
        "INSERT INTO llm_task_routes (id, task_type, provider_id, model_id,
         fallback_provider_id, fallback_model_id, max_retries, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
         ON CONFLICT(id) DO UPDATE SET
         task_type=excluded.task_type, provider_id=excluded.provider_id,
         model_id=excluded.model_id, fallback_provider_id=excluded.fallback_provider_id,
         fallback_model_id=excluded.fallback_model_id, max_retries=excluded.max_retries,
         updated_at=excluded.updated_at",
        params![
            route.id,
            route.task_type,
            route.provider_id,
            route.model_id,
            route.fallback_provider_id,
            route.fallback_model_id,
            route.max_retries,
            now,
            now
        ],
    )
    .map_err(|e| {
        AppErrorDto::new("DB_WRITE_FAILED", "Cannot save task route", true)
            .with_detail(e.to_string())
    })?;
    Ok(())
}

/// Delete a task route.
pub fn delete_task_route(conn: &Connection, route_id: &str) -> Result<(), AppErrorDto> {
    conn.execute(
        "DELETE FROM llm_task_routes WHERE id = ?1",
        params![route_id],
    )
    .map_err(|e| {
        AppErrorDto::new("DB_DELETE_FAILED", "Cannot delete task route", true)
            .with_detail(e.to_string())
    })?;
    Ok(())
}

// ── app_settings CRUD ──

/// Load a single app setting by key. Returns None if not found.
pub fn load_app_setting(conn: &Connection, key: &str) -> Result<Option<String>, AppErrorDto> {
    let mut stmt = conn
        .prepare("SELECT value FROM app_settings WHERE key = ?1")
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot read app setting", true)
                .with_detail(e.to_string())
        })?;

    let result = stmt.query_row(params![key], |row| row.get::<_, String>(0));

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(
            AppErrorDto::new("DB_READ_FAILED", "Cannot read app setting", true)
                .with_detail(e.to_string()),
        ),
    }
}

/// Save (upsert) an app setting.
pub fn save_app_setting(
    conn: &Connection,
    key: &str,
    value: &str,
    now: &str,
) -> Result<(), AppErrorDto> {
    conn.execute(
        "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, now],
    )
    .map_err(|e| {
        AppErrorDto::new("DB_WRITE_FAILED", "Cannot save app setting", true)
            .with_detail(e.to_string())
    })?;
    Ok(())
}

/// Get recent refresh logs for a provider.
#[allow(dead_code)]
pub fn load_refresh_logs(
    conn: &Connection,
    provider_id: &str,
    limit: i64,
) -> Result<Vec<crate::adapters::llm_types::RefreshLog>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, provider_id, refresh_type, status,
                models_added, models_updated, models_removed, error_message, created_at
         FROM llm_model_refresh_logs WHERE provider_id = ?1
         ORDER BY created_at DESC LIMIT ?2",
        )
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot load refresh logs", true)
                .with_detail(e.to_string())
        })?;

    let rows = stmt
        .query_map(params![provider_id, limit], |row| {
            Ok(crate::adapters::llm_types::RefreshLog {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                refresh_type: row.get(2)?,
                status: row.get(3)?,
                models_added: row.get(4)?,
                models_updated: row.get(5)?,
                models_removed: row.get(6)?,
                error_message: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "Cannot load refresh logs", true)
                .with_detail(e.to_string())
        })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "Error reading refresh logs", true)
            .with_detail(e.to_string())
    })
}

fn ensure_schema_compatibility(conn: &Connection) -> Result<(), AppErrorDto> {
    ensure_column(
        conn,
        "llm_models",
        "source",
        "TEXT NOT NULL DEFAULT 'registry'",
    )?;
    ensure_column(
        conn,
        "llm_models",
        "user_overridden",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(conn, "llm_models", "last_seen_at", "TEXT")?;
    ensure_column(conn, "llm_models", "registry_version", "TEXT")?;
    ensure_column(
        conn,
        "llm_model_registry_state",
        "registry_updated_at",
        "TEXT",
    )?;
    ensure_column(conn, "llm_model_registry_state", "error_code", "TEXT")?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    ddl: &str,
) -> Result<(), AppErrorDto> {
    let pragma = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma).map_err(|e| {
        AppErrorDto::new(
            "APP_DB_SCHEMA_READ_FAILED",
            "Cannot read app database schema",
            false,
        )
        .with_detail(e.to_string())
    })?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| {
            AppErrorDto::new(
                "APP_DB_SCHEMA_READ_FAILED",
                "Cannot read app database schema",
                false,
            )
            .with_detail(e.to_string())
        })?;

    for name in rows {
        if name.map_err(|e| {
            AppErrorDto::new(
                "APP_DB_SCHEMA_READ_FAILED",
                "Cannot read app database schema",
                false,
            )
            .with_detail(e.to_string())
        })? == column
        {
            return Ok(());
        }
    }

    let alter = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, ddl);
    conn.execute(&alter, []).map_err(|e| {
        AppErrorDto::new(
            "APP_DB_MIGRATION_FAILED",
            "Cannot migrate app database schema",
            false,
        )
        .with_detail(e.to_string())
    })?;
    Ok(())
}

fn migrate_legacy_database_if_needed(target_db_path: &Path) -> Result<(), AppErrorDto> {
    if target_db_path.exists() {
        return Ok(());
    }

    let Some(legacy_dir) = crate::infra::app_paths::legacy_home_app_dir() else {
        return Ok(());
    };
    let legacy_db_path = legacy_dir.join("novelforge.db");
    if !legacy_db_path.exists() {
        return Ok(());
    }

    if let Some(parent) = target_db_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            AppErrorDto::new(
                "APP_DB_MIGRATION_FAILED",
                "Cannot create app data directory for database migration",
                false,
            )
            .with_detail(err.to_string())
        })?;
    }

    fs::copy(&legacy_db_path, target_db_path).map_err(|err| {
        AppErrorDto::new(
            "APP_DB_MIGRATION_FAILED",
            "Cannot migrate legacy app database",
            false,
        )
        .with_detail(err.to_string())
    })?;

    for suffix in [".wal", ".shm"] {
        let legacy_sidecar = legacy_db_path.with_extension(format!("db{}", suffix));
        if legacy_sidecar.exists() {
            let target_sidecar = target_db_path.with_extension(format!("db{}", suffix));
            let _ = fs::copy(legacy_sidecar, target_sidecar);
        }
    }

    info!(
        "[APP_DB] migrated legacy app database from {} to {}",
        legacy_db_path.to_string_lossy(),
        target_db_path.to_string_lossy()
    );

    Ok(())
}
