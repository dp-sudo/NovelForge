//! App-level SQLite database (separate from project databases).
//!
//! Stores provider configurations, model registry, task routes, and app settings.
//! Located at the per-user app data directory (`%LOCALAPPDATA%\\NovelForge` on Windows).

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use rusqlite::{params, Connection};

use crate::adapters::llm_types::{
    ModelPoolEntry, ModelPoolRecord, ModelRecord, ProviderConfig, TaskRoute,
};
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
        AppErrorDto::new("APP_DB_OPEN_FAILED", "无法打开应用数据库", true)
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

fn ensure_default_task_routes_initialized(conn: &Connection) -> Result<(), AppErrorDto> {
    let routes = load_task_routes(conn)?;
    let normalized_routes = normalize_task_routes(routes);

    let existing_task_types: HashSet<String> = normalized_routes
        .iter()
        .map(|route| route.task_type.clone())
        .collect();
    let missing_task_types = task_routing::TASK_ROUTE_TYPES_WITH_CUSTOM
        .iter()
        .copied()
        .filter(|task_type| !existing_task_types.contains(*task_type))
        .collect::<Vec<_>>();
    if missing_task_types.is_empty() {
        return Ok(());
    }

    let providers = load_all_providers(conn)?;
    let Some((provider_id, model_id)) = pick_primary_route_seed(&normalized_routes, &providers)
    else {
        return Ok(());
    };

    let now = crate::infra::time::now_iso();
    for task_type in missing_task_types {
        let route = TaskRoute {
            id: Uuid::new_v4().to_string(),
            task_type: task_type.to_string(),
            provider_id: provider_id.clone(),
            model_id: model_id.clone(),
            fallback_provider_id: None,
            fallback_model_id: None,
            model_pool_id: None,
            fallback_model_pool_id: None,
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
            AppErrorDto::new("DB_READ_FAILED", "无法读取供应商配置", true)
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
            AppErrorDto::new("DB_READ_FAILED", "无法读取供应商配置", true)
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
            AppErrorDto::new("DB_READ_FAILED", "无法列出供应商配置", true)
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
            AppErrorDto::new("DB_READ_FAILED", "无法列出供应商配置", true)
                .with_detail(e.to_string())
        })?;

    providers.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "读取供应商配置失败", true).with_detail(e.to_string())
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
        AppErrorDto::new("DB_WRITE_FAILED", "无法保存供应商配置", true).with_detail(e.to_string())
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
        AppErrorDto::new("DB_DELETE_FAILED", "无法删除供应商配置", true).with_detail(e.to_string())
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
    ).map_err(|e| AppErrorDto::new("DB_READ_FAILED", "无法加载模型列表", true).with_detail(e.to_string()))?;

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
            AppErrorDto::new("DB_READ_FAILED", "无法加载模型列表", true).with_detail(e.to_string())
        })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "读取模型列表失败", true).with_detail(e.to_string())
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
            AppErrorDto::new("DB_WRITE_FAILED", "无法更新模型信息", true).with_detail(e.to_string())
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
        ).map_err(|e| AppErrorDto::new("DB_WRITE_FAILED", "无法写入模型信息", true).with_detail(e.to_string()))?;
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
        AppErrorDto::new("DB_WRITE_FAILED", "无法写入刷新日志", true).with_detail(e.to_string())
    })?;
    Ok(())
}

// ── llm_task_routes CRUD ──

/// Load all task routes.
pub fn load_task_routes(conn: &Connection) -> Result<Vec<TaskRoute>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_type, provider_id, model_id,
                fallback_provider_id, fallback_model_id, model_pool_id, fallback_model_pool_id,
                max_retries, created_at, updated_at
         FROM llm_task_routes ORDER BY task_type",
        )
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "无法加载任务路由", true).with_detail(e.to_string())
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
                model_pool_id: row.get(6)?,
                fallback_model_pool_id: row.get(7)?,
                max_retries: row.get(8)?,
                created_at: Some(row.get::<_, String>(9)?),
                updated_at: Some(row.get::<_, String>(10)?),
            })
        })
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "无法加载任务路由", true).with_detail(e.to_string())
        })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "读取任务路由失败", true).with_detail(e.to_string())
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
         fallback_provider_id, fallback_model_id, model_pool_id, fallback_model_pool_id,
         max_retries, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(id) DO UPDATE SET
         task_type=excluded.task_type, provider_id=excluded.provider_id,
         model_id=excluded.model_id, fallback_provider_id=excluded.fallback_provider_id,
         fallback_model_id=excluded.fallback_model_id,
         model_pool_id=excluded.model_pool_id,
         fallback_model_pool_id=excluded.fallback_model_pool_id,
         max_retries=excluded.max_retries,
         updated_at=excluded.updated_at",
        params![
            route.id,
            route.task_type,
            route.provider_id,
            route.model_id,
            route.fallback_provider_id,
            route.fallback_model_id,
            route.model_pool_id,
            route.fallback_model_pool_id,
            route.max_retries,
            now,
            now
        ],
    )
    .map_err(|e| {
        AppErrorDto::new("DB_WRITE_FAILED", "无法保存任务路由", true).with_detail(e.to_string())
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
        AppErrorDto::new("DB_DELETE_FAILED", "无法删除任务路由", true).with_detail(e.to_string())
    })?;
    Ok(())
}

pub fn load_model_pools(conn: &Connection) -> Result<Vec<ModelPoolRecord>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, display_name, role, enabled, entries_json, fallback_pool_id, created_at, updated_at
             FROM llm_model_pools
             ORDER BY role, id",
        )
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "无法加载模型池配置", true)
                .with_detail(e.to_string())
        })?;
    let rows = stmt
        .query_map([], |row| {
            let entries_raw: String = row.get(4)?;
            let entries =
                serde_json::from_str::<Vec<ModelPoolEntry>>(&entries_raw).unwrap_or_default();
            Ok(ModelPoolRecord {
                id: row.get(0)?,
                display_name: row.get(1)?,
                role: row.get(2)?,
                enabled: row.get::<_, i64>(3)? != 0,
                entries,
                fallback_pool_id: row.get(5)?,
                created_at: Some(row.get::<_, String>(6)?),
                updated_at: Some(row.get::<_, String>(7)?),
            })
        })
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "无法加载模型池配置", true)
                .with_detail(e.to_string())
        })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "读取模型池配置失败", true).with_detail(e.to_string())
    })
}

#[allow(dead_code)]
pub fn upsert_model_pool(
    conn: &Connection,
    pool: &ModelPoolRecord,
    now: &str,
) -> Result<(), AppErrorDto> {
    let entries = normalize_model_pool_entries(&pool.entries);
    let entries_json = serde_json::to_string(&entries).map_err(|e| {
        AppErrorDto::new("DB_WRITE_FAILED", "模型池条目序列化失败", true).with_detail(e.to_string())
    })?;
    conn.execute(
        "INSERT INTO llm_model_pools(
            id, display_name, role, enabled, entries_json, fallback_pool_id, created_at, updated_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
            display_name = excluded.display_name,
            role = excluded.role,
            enabled = excluded.enabled,
            entries_json = excluded.entries_json,
            fallback_pool_id = excluded.fallback_pool_id,
            updated_at = excluded.updated_at",
        params![
            pool.id,
            pool.display_name,
            pool.role,
            if pool.enabled { 1_i64 } else { 0_i64 },
            entries_json,
            pool.fallback_pool_id,
            now,
            now,
        ],
    )
    .map_err(|e| {
        AppErrorDto::new("DB_WRITE_FAILED", "无法保存模型池配置", true).with_detail(e.to_string())
    })?;
    Ok(())
}

#[allow(dead_code)]
fn normalize_model_pool_entries(entries: &[ModelPoolEntry]) -> Vec<ModelPoolEntry> {
    let mut normalized = Vec::new();
    for entry in entries {
        let provider_id = entry.provider_id.trim();
        let model_id = entry.model_id.trim();
        if provider_id.is_empty() || model_id.is_empty() {
            continue;
        }
        let duplicate = normalized.iter().any(|existing: &ModelPoolEntry| {
            existing.provider_id == provider_id && existing.model_id == model_id
        });
        if duplicate {
            continue;
        }
        normalized.push(ModelPoolEntry {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
        });
    }
    normalized
}

// ── app_settings CRUD ──

/// Load a single app setting by key. Returns None if not found.
pub fn load_app_setting(conn: &Connection, key: &str) -> Result<Option<String>, AppErrorDto> {
    let mut stmt = conn
        .prepare("SELECT value FROM app_settings WHERE key = ?1")
        .map_err(|e| {
            AppErrorDto::new("DB_READ_FAILED", "无法读取应用设置", true).with_detail(e.to_string())
        })?;

    let result = stmt.query_row(params![key], |row| row.get::<_, String>(0));

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => {
            Err(AppErrorDto::new("DB_READ_FAILED", "无法读取应用设置", true)
                .with_detail(e.to_string()))
        }
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
        AppErrorDto::new("DB_WRITE_FAILED", "无法保存应用设置", true).with_detail(e.to_string())
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
            AppErrorDto::new("DB_READ_FAILED", "无法加载刷新日志", true).with_detail(e.to_string())
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
            AppErrorDto::new("DB_READ_FAILED", "无法加载刷新日志", true).with_detail(e.to_string())
        })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        AppErrorDto::new("DB_READ_FAILED", "读取刷新日志失败", true).with_detail(e.to_string())
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
        AppErrorDto::new("APP_DB_SCHEMA_READ_FAILED", "无法读取应用数据库结构", false)
            .with_detail(e.to_string())
    })?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| {
            AppErrorDto::new("APP_DB_SCHEMA_READ_FAILED", "无法读取应用数据库结构", false)
                .with_detail(e.to_string())
        })?;

    for name in rows {
        if name.map_err(|e| {
            AppErrorDto::new("APP_DB_SCHEMA_READ_FAILED", "无法读取应用数据库结构", false)
                .with_detail(e.to_string())
        })? == column
        {
            return Ok(());
        }
    }

    let alter = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, ddl);
    conn.execute(&alter, []).map_err(|e| {
        AppErrorDto::new("APP_DB_MIGRATION_FAILED", "无法迁移应用数据库结构", false)
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
                "无法为数据库迁移创建应用数据目录",
                false,
            )
            .with_detail(err.to_string())
        })?;
    }

    fs::copy(&legacy_db_path, target_db_path).map_err(|err| {
        AppErrorDto::new("APP_DB_MIGRATION_FAILED", "无法迁移旧版应用数据库", false)
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rusqlite::Connection;
    use uuid::Uuid;

    use super::*;
    use crate::adapters::llm_types::ProviderConfig;

    fn setup_app_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory app db");
        crate::infra::migrator::run_app_pending(&conn).expect("run app migrations");
        ensure_schema_compatibility(&conn).expect("ensure schema compatibility");
        conn
    }

    fn sample_provider(default_model: &str) -> ProviderConfig {
        ProviderConfig {
            id: "deepseek".to_string(),
            display_name: "DeepSeek".to_string(),
            vendor: "openai".to_string(),
            protocol: "openai_responses".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            endpoint_path: None,
            api_key: None,
            auth_mode: "bearer".to_string(),
            auth_header_name: None,
            anthropic_version: None,
            beta_headers: None,
            custom_headers: None,
            default_model: Some(default_model.to_string()),
            timeout_ms: 120_000,
            connect_timeout_ms: 15_000,
            max_retries: 2,
            model_refresh_mode: Some("registry".to_string()),
            models_path: None,
            last_model_refresh_at: None,
        }
    }

    #[test]
    fn ensure_default_task_routes_backfills_missing_core_routes() {
        let conn = setup_app_conn();
        let now = crate::infra::time::now_iso();
        upsert_provider(&conn, &sample_provider("deepseek-chat"), &now)
            .expect("upsert provider should succeed");

        let chapter_route = TaskRoute {
            id: Uuid::new_v4().to_string(),
            task_type: "chapter.draft".to_string(),
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-chat".to_string(),
            fallback_provider_id: None,
            fallback_model_id: None,
            model_pool_id: None,
            fallback_model_pool_id: None,
            max_retries: 1,
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };
        upsert_task_route(&conn, &chapter_route, &now).expect("upsert route should succeed");

        ensure_default_task_routes_initialized(&conn).expect("backfill routes should succeed");

        let routes = load_task_routes(&conn).expect("load routes");
        let route_types = routes
            .iter()
            .map(|route| route.task_type.clone())
            .collect::<HashSet<_>>();
        for task_type in task_routing::TASK_ROUTE_TYPES_WITH_CUSTOM {
            assert!(
                route_types.contains(*task_type),
                "missing task route {}",
                task_type
            );
        }
    }

    #[test]
    fn model_pool_upsert_and_load_roundtrip() {
        let conn = setup_app_conn();
        let now = crate::infra::time::now_iso();
        let pool = ModelPoolRecord {
            id: "drafter".to_string(),
            display_name: "Drafter Pool".to_string(),
            role: "drafter".to_string(),
            enabled: true,
            entries: vec![
                ModelPoolEntry {
                    provider_id: "deepseek".to_string(),
                    model_id: "deepseek-chat".to_string(),
                },
                // duplicate should be deduped on write
                ModelPoolEntry {
                    provider_id: "deepseek".to_string(),
                    model_id: "deepseek-chat".to_string(),
                },
            ],
            fallback_pool_id: Some("reviewer".to_string()),
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };

        upsert_model_pool(&conn, &pool, &now).expect("upsert model pool");
        let loaded = load_model_pools(&conn).expect("load model pools");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "drafter");
        assert_eq!(loaded[0].entries.len(), 1);
        assert_eq!(loaded[0].entries[0].provider_id, "deepseek");
        assert_eq!(loaded[0].entries[0].model_id, "deepseek-chat");
        assert_eq!(loaded[0].fallback_pool_id.as_deref(), Some("reviewer"));
    }
}
