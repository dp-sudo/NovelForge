//! Database migration runner.
//!
//! Migration SQL files live as independent `*.sql` files in the `migrations/`
//! directory and are embedded into the binary at compile time via `include_str!`.
//!
//! File layout:
//!   migrations/project/0001_init.sql
//!   migrations/project/0002_task_route_unique.sql
//!   migrations/project/0003_pipeline_draft_pool.sql
//!   migrations/app/0001_init.sql
//!   migrations/app/0002_skill_index.sql
//!   migrations/app/0003_task_route_unique.sql

use log::{info, warn};
use rusqlite::Connection;

use crate::errors::AppErrorDto;

const MIGRATIONS_TABLE: &str = "schema_migrations";

/// A single migration: version name and embedded SQL.
struct Migration {
    pub version: &'static str,
    pub sql: &'static str,
}

/// Result of running pending migrations.
pub struct MigrationResult {
    pub applied: Vec<String>,
}

/// Return the ordered list of project-db migrations embedded at compile time.
fn project_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: "0001_init",
            sql: include_str!("../../migrations/project/0001_init.sql"),
        },
        Migration {
            version: "0002_task_route_unique",
            sql: include_str!("../../migrations/project/0002_task_route_unique.sql"),
        },
        Migration {
            version: "0003_pipeline_draft_pool",
            sql: include_str!("../../migrations/project/0003_pipeline_draft_pool.sql"),
        },
        Migration {
            version: "0004_ai_strategy_profile",
            sql: include_str!("../../migrations/project/0004_ai_strategy_profile.sql"),
        },
        Migration {
            version: "0005_entity_provenance",
            sql: include_str!("../../migrations/project/0005_entity_provenance.sql"),
        },
        Migration {
            version: "0006_story_state",
            sql: include_str!("../../migrations/project/0006_story_state.sql"),
        },
        Migration {
            version: "0007_blueprint_certainty_zones",
            sql: include_str!("../../migrations/project/0007_blueprint_certainty_zones.sql"),
        },
        Migration {
            version: "0008_pipeline_run_meta",
            sql: include_str!("../../migrations/project/0008_pipeline_run_meta.sql"),
        },
        Migration {
            version: "0009_user_review_actions",
            sql: include_str!("../../migrations/project/0009_user_review_actions.sql"),
        },
        Migration {
            version: "0010_feedback_events",
            sql: include_str!("../../migrations/project/0010_feedback_events.sql"),
        },
    ]
}

/// Return the ordered list of app-db migrations embedded at compile time.
fn app_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: "0001_init",
            sql: include_str!("../../migrations/app/0001_init.sql"),
        },
        Migration {
            version: "0002_skill_index",
            sql: include_str!("../../migrations/app/0002_skill_index.sql"),
        },
        Migration {
            version: "0003_task_route_unique",
            sql: include_str!("../../migrations/app/0003_task_route_unique.sql"),
        },
        Migration {
            version: "0004_model_pools",
            sql: include_str!("../../migrations/app/0004_model_pools.sql"),
        },
        Migration {
            version: "0005_promotion_policies",
            sql: include_str!("../../migrations/app/0005_promotion_policies.sql"),
        },
        Migration {
            version: "0006_feedback_rules",
            sql: include_str!("../../migrations/app/0006_feedback_rules.sql"),
        },
    ]
}

/// Run all pending project-db migrations on the given connection.
pub fn run_project_pending(conn: &Connection) -> Result<MigrationResult, AppErrorDto> {
    run_pending(conn, &project_migrations(), "project")
}

/// Run all pending app-db migrations on the given connection.
pub fn run_app_pending(conn: &Connection) -> Result<MigrationResult, AppErrorDto> {
    run_pending(conn, &app_migrations(), "app")
}

/// Core migration logic: apply pending migrations in order.
fn run_pending(
    conn: &Connection,
    migrations: &[Migration],
    label: &str,
) -> Result<MigrationResult, AppErrorDto> {
    // Ensure the migration tracking table exists
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {} (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL);",
        MIGRATIONS_TABLE
    ))
    .map_err(|e| {
        AppErrorDto::new(
            "MIGRATION_INIT_FAILED",
            &format!("{}: cannot init migration table", label),
            false,
        )
        .with_detail(e.to_string())
    })?;

    // Read already-applied versions
    let mut applied_versions: Vec<String> = {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT version FROM {} ORDER BY version",
                MIGRATIONS_TABLE
            ))
            .map_err(|e| {
                AppErrorDto::new(
                    "MIGRATION_READ_FAILED",
                    &format!("{}: cannot read applied migrations", label),
                    false,
                )
                .with_detail(e.to_string())
            })?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| {
                AppErrorDto::new(
                    "MIGRATION_READ_FAILED",
                    &format!("{}: cannot read applied migrations", label),
                    false,
                )
                .with_detail(e.to_string())
            })?;
        let mut versions: Vec<String> = rows.filter_map(|r| r.ok()).collect();
        versions.sort();
        versions
    };

    // Backward compatibility: if no migrations recorded but tables already exist,
    // mark the first migration as applied (pre-migration database)
    if applied_versions.is_empty() && has_any_table(conn) {
        if let Some(first) = migrations.first() {
            warn!(
                "[MIGRATOR] {}: DB has tables but no migration records — marking '{}' as applied",
                label, first.version
            );
            mark_applied(conn, first.version)?;
            applied_versions.push(first.version.to_string());
        }
    }

    // Determine pending migrations
    let pending: Vec<&Migration> = migrations
        .iter()
        .filter(|m| !applied_versions.iter().any(|v| v == m.version))
        .collect();

    if pending.is_empty() {
        info!(
            "[MIGRATOR] {}: all {} migrations already applied",
            label,
            applied_versions.len()
        );
        return Ok(MigrationResult { applied: vec![] });
    }

    // Apply each pending migration within a transaction
    let mut applied = Vec::new();
    for migration in &pending {
        info!(
            "[MIGRATOR] {}: applying migration '{}'",
            label, migration.version
        );

        match conn.execute_batch(migration.sql) {
            Ok(()) => {}
            Err(err) => {
                let detail = err.to_string();
                let duplicate_column = detail
                    .to_ascii_lowercase()
                    .contains("duplicate column name");
                if duplicate_column {
                    warn!(
                        "[MIGRATOR] {}: migration '{}' hit duplicate column, treat as already applied: {}",
                        label, migration.version, detail
                    );
                } else {
                    return Err(
                        AppErrorDto::new(
                            "MIGRATION_FAILED",
                            &format!("{}: migration '{}' failed", label, migration.version),
                            true,
                        )
                        .with_detail(detail),
                    );
                }
            }
        }

        mark_applied(conn, migration.version)?;
        applied.push(migration.version.to_string());
        info!(
            "[MIGRATOR] {}: migration '{}' applied successfully",
            label, migration.version
        );
    }

    Ok(MigrationResult { applied })
}

/// Record a migration version as applied.
fn mark_applied(conn: &Connection, version: &str) -> Result<(), AppErrorDto> {
    let now = crate::infra::time::now_iso();
    conn.execute(
        &format!(
            "INSERT OR IGNORE INTO {} (version, applied_at) VALUES (?1, ?2)",
            MIGRATIONS_TABLE
        ),
        rusqlite::params![version, now],
    )
    .map_err(|e| {
        AppErrorDto::new("MIGRATION_MARK_FAILED", "Cannot record migration", false)
            .with_detail(e.to_string())
    })?;
    Ok(())
}

/// Check if the database has any user tables (for backward compat detection).
fn has_any_table(conn: &Connection) -> bool {
    let count: Result<i64, _> = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations'",
        [],
        |row| row.get(0),
    );
    count.unwrap_or(0) > 0
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{run_app_pending, run_project_pending};

    fn insert_migration_marker(conn: &Connection, version: &str) {
        conn.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
            params![version, "2026-04-28T00:00:00Z"],
        )
        .expect("insert schema migration marker");
    }

    fn has_index(conn: &Connection, table: &str, index_name: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA index_list({table})"))
            .expect("prepare index list");
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query index list")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect index names");
        rows.iter().any(|name| name == index_name)
    }

    fn has_table(conn: &Connection, table: &str) -> bool {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get(0),
            )
            .expect("query table existence");
        count > 0
    }

    fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("prepare table info");
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query table info")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect table columns");
        columns.iter().any(|name| name == column)
    }

    fn insert_provider(conn: &Connection, provider_id: &str) {
        conn.execute(
            "INSERT INTO llm_providers(
                id, display_name, vendor, protocol, base_url, created_at, updated_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                provider_id,
                provider_id,
                "test-vendor",
                "openai_compatible",
                "https://example.invalid",
                "2026-04-28T00:00:00Z",
                "2026-04-28T00:00:00Z"
            ],
        )
        .expect("insert provider");
    }

    #[test]
    fn app_task_route_unique_migration_canonicalizes_and_deduplicates() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(include_str!("../../migrations/app/0001_init.sql"))
            .expect("apply app 0001");
        insert_migration_marker(&conn, "0001_init");
        insert_provider(&conn, "provider-a");
        insert_provider(&conn, "provider-b");

        conn.execute(
            "INSERT INTO llm_task_routes(id, task_type, provider_id, model_id, max_retries, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, 1, ?5, ?6)",
            params![
                "legacy-row",
                "chapter_draft",
                "provider-a",
                "model-a",
                "2026-04-27T10:00:00Z",
                "2026-04-27T10:00:00Z"
            ],
        )
        .expect("insert legacy app route");
        conn.execute(
            "INSERT INTO llm_task_routes(id, task_type, provider_id, model_id, max_retries, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, 1, ?5, ?6)",
            params![
                "canonical-row",
                "chapter.draft",
                "provider-b",
                "model-b",
                "2026-04-28T10:00:00Z",
                "2026-04-28T10:00:00Z"
            ],
        )
        .expect("insert canonical app route");

        run_app_pending(&conn).expect("apply app pending migrations");

        let canonical_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM llm_task_routes WHERE task_type = 'chapter.draft'",
                [],
                |row| row.get(0),
            )
            .expect("count canonical app routes");
        let legacy_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM llm_task_routes WHERE task_type = 'chapter_draft'",
                [],
                |row| row.get(0),
            )
            .expect("count legacy app routes");
        assert_eq!(canonical_count, 1);
        assert_eq!(legacy_count, 0);
        assert!(has_index(
            &conn,
            "llm_task_routes",
            "ux_llm_task_routes_task_type"
        ));
    }

    #[test]
    fn app_model_pool_migration_creates_table_and_task_route_pool_columns() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(include_str!("../../migrations/app/0001_init.sql"))
            .expect("apply app 0001");
        conn.execute_batch(include_str!("../../migrations/app/0002_skill_index.sql"))
            .expect("apply app 0002");
        conn.execute_batch(include_str!(
            "../../migrations/app/0003_task_route_unique.sql"
        ))
        .expect("apply app 0003");
        insert_migration_marker(&conn, "0001_init");
        insert_migration_marker(&conn, "0002_skill_index");
        insert_migration_marker(&conn, "0003_task_route_unique");
        insert_provider(&conn, "provider-a");
        conn.execute(
            "INSERT INTO llm_task_routes(id, task_type, provider_id, model_id, max_retries, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, 1, ?5, ?6)",
            params![
                "route-1",
                "chapter.draft",
                "provider-a",
                "model-a",
                "2026-05-01T10:00:00Z",
                "2026-05-01T10:00:00Z"
            ],
        )
        .expect("insert route");

        run_app_pending(&conn).expect("apply app pending migrations");

        assert!(has_table(&conn, "llm_model_pools"));
        assert!(has_column(&conn, "llm_task_routes", "model_pool_id"));
        assert!(has_column(
            &conn,
            "llm_task_routes",
            "fallback_model_pool_id"
        ));
        let seeded_pool_id: Option<String> = conn
            .query_row(
                "SELECT model_pool_id FROM llm_task_routes WHERE id = 'route-1'",
                [],
                |row| row.get(0),
            )
            .expect("read pool id");
        assert_eq!(seeded_pool_id.as_deref(), Some("drafter"));
    }

    #[test]
    fn app_feedback_rules_migration_tolerates_existing_post_tasks_column() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(include_str!("../../migrations/app/0001_init.sql"))
            .expect("apply app 0001");
        conn.execute_batch(include_str!("../../migrations/app/0002_skill_index.sql"))
            .expect("apply app 0002");
        conn.execute_batch(include_str!(
            "../../migrations/app/0003_task_route_unique.sql"
        ))
        .expect("apply app 0003");
        conn.execute_batch(include_str!("../../migrations/app/0004_model_pools.sql"))
            .expect("apply app 0004");
        conn.execute_batch(include_str!(
            "../../migrations/app/0005_promotion_policies.sql"
        ))
        .expect("apply app 0005");
        insert_migration_marker(&conn, "0001_init");
        insert_migration_marker(&conn, "0002_skill_index");
        insert_migration_marker(&conn, "0003_task_route_unique");
        insert_migration_marker(&conn, "0004_model_pools");
        insert_migration_marker(&conn, "0005_promotion_policies");

        conn.execute(
            "ALTER TABLE llm_task_routes ADD COLUMN post_tasks_json TEXT NOT NULL DEFAULT '[]'",
            [],
        )
        .expect("manually add post_tasks_json to simulate compatibility patch");

        run_app_pending(&conn).expect("apply app pending migrations");

        let marker_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = '0006_feedback_rules'",
                [],
                |row| row.get(0),
            )
            .expect("query migration marker");
        assert_eq!(marker_count, 1);
        assert!(has_table(&conn, "feedback_rules"));
    }

    #[test]
    fn project_task_route_unique_migration_canonicalizes_and_deduplicates() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(include_str!("../../migrations/project/0001_init.sql"))
            .expect("apply project 0001");
        insert_migration_marker(&conn, "0001_init");
        insert_provider(&conn, "provider-a");
        insert_provider(&conn, "provider-b");

        conn.execute(
            "INSERT INTO llm_task_routes(
                id, project_id, task_type, provider_id, model_id, priority, max_retries, created_at, updated_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8)",
            params![
                "legacy-row",
                "project-1",
                "chapter_continue",
                "provider-a",
                "model-a",
                0_i64,
                "2026-04-27T10:00:00Z",
                "2026-04-27T10:00:00Z"
            ],
        )
        .expect("insert legacy project route");
        conn.execute(
            "INSERT INTO llm_task_routes(
                id, project_id, task_type, provider_id, model_id, priority, max_retries, created_at, updated_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8)",
            params![
                "canonical-row",
                "project-1",
                "chapter.continue",
                "provider-b",
                "model-b",
                10_i64,
                "2026-04-28T10:00:00Z",
                "2026-04-28T10:00:00Z"
            ],
        )
        .expect("insert canonical project route");

        run_project_pending(&conn).expect("apply project pending migrations");

        let canonical_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM llm_task_routes
                 WHERE project_id = 'project-1' AND task_type = 'chapter.continue'",
                [],
                |row| row.get(0),
            )
            .expect("count canonical project routes");
        let legacy_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM llm_task_routes
                 WHERE project_id = 'project-1' AND task_type = 'chapter_continue'",
                [],
                |row| row.get(0),
            )
            .expect("count legacy project routes");
        assert_eq!(canonical_count, 1);
        assert_eq!(legacy_count, 0);
        assert!(has_index(
            &conn,
            "llm_task_routes",
            "ux_llm_task_routes_project_task_type"
        ));
    }

    #[test]
    fn project_pipeline_draft_pool_migration_creates_schema_and_enforces_pending_dedupe() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(include_str!("../../migrations/project/0001_init.sql"))
            .expect("apply project 0001");
        conn.execute_batch(include_str!(
            "../../migrations/project/0002_task_route_unique.sql"
        ))
        .expect("apply project 0002");
        insert_migration_marker(&conn, "0001_init");
        insert_migration_marker(&conn, "0002_task_route_unique");

        conn.execute(
            "INSERT INTO projects(
                id, name, project_path, schema_version, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "project-1",
                "Project 1",
                "F:/NovelForge/tests/project-1",
                "1.0.0",
                "2026-04-28T00:00:00Z",
                "2026-04-28T00:00:00Z"
            ],
        )
        .expect("insert project");
        conn.execute(
            "INSERT INTO chapters(
                id, project_id, chapter_index, title, content_path, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "chapter-1",
                "project-1",
                1_i64,
                "Chapter 1",
                "chapters/chapter-1.md",
                "2026-04-28T00:00:00Z",
                "2026-04-28T00:00:00Z"
            ],
        )
        .expect("insert chapter");

        run_project_pending(&conn).expect("apply project pending migrations");

        assert!(has_table(&conn, "ai_pipeline_runs"));
        assert!(has_column(&conn, "ai_pipeline_runs", "meta_json"));
        assert!(has_table(&conn, "structured_draft_batches"));
        assert!(has_table(&conn, "structured_draft_items"));
        assert!(has_table(&conn, "story_state"));
        assert!(has_index(
            &conn,
            "structured_draft_items",
            "idx_sdi_project_chapter_kind"
        ));
        assert!(has_index(
            &conn,
            "structured_draft_items",
            "idx_sdi_status_created"
        ));
        assert!(has_index(
            &conn,
            "structured_draft_items",
            "ux_sdi_project_kind_key_pending"
        ));
        assert!(has_index(&conn, "story_state", "idx_story_state_lookup"));

        conn.execute(
            "INSERT INTO ai_pipeline_runs(
                id, project_id, chapter_id, task_type, ui_action, status, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "run-1",
                "project-1",
                "chapter-1",
                "chapter.continue",
                "continue_chapter",
                "done",
                "2026-04-28T11:00:00Z"
            ],
        )
        .expect("insert run");
        conn.execute(
            "INSERT INTO structured_draft_batches(
                id, run_id, project_id, chapter_id, source_task_type, content_hash, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "batch-1",
                "run-1",
                "project-1",
                "chapter-1",
                "chapter.continue",
                "hash-1",
                "pending",
                "2026-04-28T11:00:00Z",
                "2026-04-28T11:00:00Z"
            ],
        )
        .expect("insert batch");
        conn.execute(
            "INSERT INTO structured_draft_items(
                id, batch_id, run_id, project_id, chapter_id, draft_kind, source_label, target_label,
                normalized_key, confidence, occurrences, evidence_text, payload_json, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "item-1",
                "batch-1",
                "run-1",
                "project-1",
                "chapter-1",
                "relationship",
                "Alice",
                "Bob",
                "rel:alice|bob|ally",
                0.91_f64,
                1_i64,
                "Alice and Bob form an alliance.",
                "{\"relationship_type\":\"ally\"}",
                "pending",
                "2026-04-28T11:00:00Z",
                "2026-04-28T11:00:00Z"
            ],
        )
        .expect("insert first pending item");

        let duplicate_pending = conn.execute(
            "INSERT INTO structured_draft_items(
                id, batch_id, run_id, project_id, chapter_id, draft_kind, source_label, target_label,
                normalized_key, confidence, occurrences, evidence_text, payload_json, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "item-2",
                "batch-1",
                "run-1",
                "project-1",
                "chapter-1",
                "relationship",
                "Alice",
                "Bob",
                "rel:alice|bob|ally",
                0.86_f64,
                1_i64,
                "Duplicate extraction for the same relationship.",
                "{\"relationship_type\":\"ally\"}",
                "pending",
                "2026-04-28T11:01:00Z",
                "2026-04-28T11:01:00Z"
            ],
        );
        assert!(duplicate_pending.is_err());

        conn.execute(
            "INSERT INTO structured_draft_items(
                id, batch_id, run_id, project_id, chapter_id, draft_kind, source_label, target_label,
                normalized_key, confidence, occurrences, evidence_text, payload_json, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "item-3",
                "batch-1",
                "run-1",
                "project-1",
                "chapter-1",
                "relationship",
                "Alice",
                "Bob",
                "rel:alice|bob|ally",
                0.86_f64,
                1_i64,
                "Approved version retained for history.",
                "{\"relationship_type\":\"ally\"}",
                "applied",
                "2026-04-28T11:02:00Z",
                "2026-04-28T11:02:00Z"
            ],
        )
        .expect("insert applied item with same key");
    }
}
