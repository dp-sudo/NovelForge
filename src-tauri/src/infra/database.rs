use std::fs;
use std::path::{Path, PathBuf};

use log::info;
use rusqlite::{Connection, Result as SqlResult};

pub fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join("database").join("project.sqlite")
}

pub fn initialize_database(project_root: &Path) -> SqlResult<()> {
    let db_path = get_database_path(project_root);
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(db_path)?;
    // Run migrations (will create tables and track versions)
    let result = crate::infra::migrator::run_project_pending(&conn).map_err(|e| {
        let detail = e
            .detail
            .unwrap_or_else(|| "unknown migration error".to_string());
        rusqlite::Error::InvalidParameterName(format!("Migration failed: {} ({detail})", e.message))
    })?;
    for v in &result.applied {
        info!("[DB] Applied project migration: {}", v);
    }
    ensure_compatible_schema(&conn)?;
    Ok(())
}

pub fn open_database(project_root: &Path) -> SqlResult<Connection> {
    let conn = Connection::open(get_database_path(project_root))?;
    // Run pending migrations on open (idempotent)
    let result = crate::infra::migrator::run_project_pending(&conn).map_err(|e| {
        let detail = e
            .detail
            .unwrap_or_else(|| "unknown migration error".to_string());
        rusqlite::Error::InvalidParameterName(format!("Migration failed: {} ({detail})", e.message))
    })?;
    for v in &result.applied {
        info!("[DB] Applied project migration on open: {}", v);
    }
    ensure_compatible_schema(&conn)?;
    Ok(conn)
}

fn ensure_compatible_schema(conn: &Connection) -> SqlResult<()> {
    ensure_column(conn, "projects", "narrative_pov", "TEXT")?;
    ensure_column(conn, "projects", "writing_style", "TEXT")?;
    ensure_column(conn, "projects", "ai_strategy_profile", "TEXT")?;
    ensure_column(conn, "chapters", "target_words", "INTEGER DEFAULT 0")?;
    ensure_column(conn, "chapters", "current_words", "INTEGER DEFAULT 0")?;
    ensure_column(conn, "chapters", "version", "INTEGER NOT NULL DEFAULT 1")?;
    ensure_column(conn, "chapters", "is_deleted", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "chapters", "volume_id", "TEXT")?;
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> SqlResult<()> {
    if table_has_column(conn, table, column)? {
        return Ok(());
    }
    let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    conn.execute(&sql, [])?;
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> SqlResult<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use rusqlite::Connection;
    use uuid::Uuid;

    use super::{get_database_path, open_database};

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-db-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("prepare table info");
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query table info")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect columns");
        rows.iter().any(|name| name == column)
    }

    #[test]
    fn open_database_upgrades_legacy_missing_columns() {
        let workspace = create_temp_workspace();
        let db_path = get_database_path(&workspace);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).expect("create db parent");
        }

        let conn = Connection::open(&db_path).expect("open legacy db");
        conn.execute_batch(
            r#"
            CREATE TABLE projects (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              author TEXT,
              genre TEXT,
              target_words INTEGER DEFAULT 0,
              current_words INTEGER DEFAULT 0,
              project_path TEXT NOT NULL,
              schema_version TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            CREATE TABLE chapters (
              id TEXT PRIMARY KEY,
              project_id TEXT NOT NULL,
              chapter_index INTEGER NOT NULL,
              title TEXT NOT NULL,
              summary TEXT,
              status TEXT NOT NULL DEFAULT 'drafting',
              content_path TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              UNIQUE(project_id, chapter_index)
            );
            CREATE TABLE llm_providers (
              id TEXT PRIMARY KEY,
              display_name TEXT NOT NULL,
              vendor TEXT NOT NULL,
              protocol TEXT NOT NULL,
              base_url TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            CREATE TABLE llm_task_routes (
              id TEXT PRIMARY KEY,
              project_id TEXT NOT NULL,
              task_type TEXT NOT NULL,
              provider_id TEXT NOT NULL,
              model_id TEXT NOT NULL,
              priority INTEGER NOT NULL DEFAULT 0,
              max_retries INTEGER NOT NULL DEFAULT 1,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            "#,
        )
        .expect("create legacy schema");
        drop(conn);

        let upgraded = open_database(&workspace).expect("open upgraded db");
        assert!(column_exists(&upgraded, "projects", "narrative_pov"));
        assert!(column_exists(&upgraded, "projects", "writing_style"));
        assert!(column_exists(&upgraded, "projects", "ai_strategy_profile"));
        assert!(column_exists(&upgraded, "chapters", "target_words"));
        assert!(column_exists(&upgraded, "chapters", "current_words"));
        assert!(column_exists(&upgraded, "chapters", "version"));
        assert!(column_exists(&upgraded, "chapters", "is_deleted"));
        assert!(column_exists(&upgraded, "chapters", "volume_id"));

        remove_temp_workspace(&workspace);
    }
}
