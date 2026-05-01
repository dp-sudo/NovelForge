use std::path::Path;

use rusqlite::{params, Connection};

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

#[derive(Clone, Default)]
pub struct PipelineAuditStore;

pub struct PipelineRunUpdate<'a> {
    pub status: &'a str,
    pub phase: &'a str,
    pub error_code: Option<&'a str>,
    pub error_message: Option<&'a str>,
    pub duration_ms: i64,
}

fn normalize_project_root(project_root: &str) -> Result<&str, AppErrorDto> {
    let normalized_root = project_root.trim();
    if normalized_root.is_empty() {
        return Err(
            AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录不能为空", true)
                .with_suggested_action("请输入有效的项目目录路径"),
        );
    }
    Ok(normalized_root)
}

fn pipeline_db_open_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("PIPELINE_DB_OPEN_FAILED", "数据库打开失败", false)
        .with_detail(err.to_string())
}

fn pipeline_insert_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new(
        "PIPELINE_AUDIT_INSERT_FAILED",
        "记录 pipeline 运行失败",
        false,
    )
    .with_detail(err.to_string())
}

fn open_project_database(project_root: &str) -> Result<Connection, AppErrorDto> {
    let normalized_root = normalize_project_root(project_root)?;
    open_database(Path::new(normalized_root)).map_err(pipeline_db_open_error)
}

impl PipelineAuditStore {
    pub fn insert_pipeline_run(
        &self,
        project_root: &str,
        request_id: &str,
        chapter_id: Option<&str>,
        task_type: &str,
        ui_action: Option<&str>,
        phase: &str,
    ) -> Result<(), AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();

        conn.execute(
            "INSERT INTO ai_pipeline_runs(id, project_id, chapter_id, task_type, ui_action, status, phase, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6, ?7)",
            params![
                request_id,
                project_id,
                chapter_id.map(str::trim).filter(|v| !v.is_empty()),
                task_type,
                ui_action.map(str::trim).filter(|v| !v.is_empty()),
                phase,
                now
            ],
        )
        .map_err(pipeline_insert_error)?;
        Ok(())
    }

    pub fn update_pipeline_run(
        &self,
        project_root: &str,
        request_id: &str,
        update: PipelineRunUpdate<'_>,
    ) {
        let conn = match open_project_database(project_root) {
            Ok(conn) => conn,
            Err(_) => return,
        };
        let _ = conn.execute(
            "UPDATE ai_pipeline_runs
             SET status = ?1,
                 phase = ?2,
                 error_code = ?3,
                 error_message = ?4,
                 duration_ms = ?5,
                 completed_at = ?6
             WHERE id = ?7",
            params![
                update.status,
                update.phase,
                update.error_code,
                update.error_message,
                update.duration_ms,
                now_iso(),
                request_id
            ],
        );
    }

    pub fn touch_pipeline_phase(&self, project_root: &str, request_id: &str, phase: &str) {
        let conn = match open_project_database(project_root) {
            Ok(conn) => conn,
            Err(_) => return,
        };
        let _ = conn.execute(
            "UPDATE ai_pipeline_runs
             SET phase = ?1
             WHERE id = ?2
               AND status = 'running'",
            params![phase, request_id],
        );
    }

    pub fn update_pipeline_meta(
        &self,
        project_root: &str,
        request_id: &str,
        meta: &serde_json::Value,
    ) {
        let conn = match open_project_database(project_root) {
            Ok(conn) => conn,
            Err(_) => return,
        };
        let meta_json = match serde_json::to_string(meta) {
            Ok(raw) => raw,
            Err(_) => return,
        };
        let _ = conn.execute(
            "UPDATE ai_pipeline_runs
             SET meta_json = ?1
             WHERE id = ?2",
            params![meta_json, request_id],
        );
    }

    pub fn update_post_task_results(
        &self,
        project_root: &str,
        request_id: &str,
        post_task_results: &serde_json::Value,
    ) {
        let conn = match open_project_database(project_root) {
            Ok(conn) => conn,
            Err(_) => return,
        };
        let post_task_results_json = match serde_json::to_string(post_task_results) {
            Ok(raw) => raw,
            Err(_) => return,
        };
        let _ = conn.execute(
            "UPDATE ai_pipeline_runs
             SET post_task_results = ?1
             WHERE id = ?2",
            params![post_task_results_json, request_id],
        );
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use rusqlite::params;
    use uuid::Uuid;

    use super::PipelineAuditStore;
    use crate::infra::database::open_database;
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn insert_pipeline_run_accepts_trimmed_project_root() {
        let workspace = create_temp_workspace();
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: "pipeline-audit-trimmed-root".into(),
                author: None,
                genre: "悬疑".into(),
                target_words: None,
                save_directory: workspace.to_string_lossy().into(),
            })
            .expect("create project");
        let wrapped_root = format!("  {}  ", project.project_root);
        let store = PipelineAuditStore;

        store
            .insert_pipeline_run(
                &wrapped_root,
                "req-trimmed-root",
                None,
                "chapter",
                Some("manual"),
                "phase-start",
            )
            .expect("insert with trimmed root");

        let conn = open_database(Path::new(&project.project_root)).expect("open db");
        let run_count: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM ai_pipeline_runs WHERE id = ?1",
                params!["req-trimmed-root"],
                |row| row.get(0),
            )
            .expect("query run count");
        assert_eq!(run_count, 1);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn insert_pipeline_run_rejects_blank_project_root() {
        let store = PipelineAuditStore;
        let err = store
            .insert_pipeline_run(
                "   ",
                "req-blank-root",
                None,
                "chapter",
                None,
                "phase-start",
            )
            .expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
