//! Database migration runner.
//!
//! Migration SQL files live as independent `*.sql` files in the `migrations/`
//! directory and are embedded into the binary at compile time via `include_str!`.
//!
//! File layout:
//!   migrations/project/0001_init.sql
//!   migrations/app/0001_init.sql
//!   migrations/app/0002_add_model_fields.sql

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
    pub skipped: Vec<String>,
}

/// Return the ordered list of project-db migrations embedded at compile time.
fn project_migrations() -> Vec<Migration> {
    vec![Migration {
        version: "0001_init",
        sql: include_str!("../../migrations/project/0001_init.sql"),
    }]
}

/// Return the ordered list of app-db migrations embedded at compile time.
fn app_migrations() -> Vec<Migration> {
    vec![Migration {
        version: "0001_init",
        sql: include_str!("../../migrations/app/0001_init.sql"),
    }]
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
            .prepare(&format!("SELECT version FROM {} ORDER BY version", MIGRATIONS_TABLE))
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
        return Ok(MigrationResult {
            applied: vec![],
            skipped: applied_versions,
        });
    }

    // Apply each pending migration within a transaction
    let mut applied = Vec::new();
    for migration in &pending {
        info!(
            "[MIGRATOR] {}: applying migration '{}'",
            label, migration.version
        );

        conn.execute_batch(migration.sql).map_err(|e| {
            AppErrorDto::new(
                "MIGRATION_FAILED",
                &format!("{}: migration '{}' failed", label, migration.version),
                true,
            )
            .with_detail(e.to_string())
        })?;

        mark_applied(conn, migration.version)?;
        applied.push(migration.version.to_string());
        info!(
            "[MIGRATOR] {}: migration '{}' applied successfully",
            label, migration.version
        );
    }

    Ok(MigrationResult { applied, skipped: applied_versions })
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
