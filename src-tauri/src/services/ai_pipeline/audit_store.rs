use std::path::Path;

use rusqlite::params;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

#[derive(Clone, Default)]
pub struct PipelineAuditStore;

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
        let conn = open_database(Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("PIPELINE_DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
        })?;
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
        .map_err(|err| {
            AppErrorDto::new("PIPELINE_AUDIT_INSERT_FAILED", "记录 pipeline 运行失败", false)
                .with_detail(err.to_string())
        })?;
        Ok(())
    }

    pub fn update_pipeline_run(
        &self,
        project_root: &str,
        request_id: &str,
        status: &str,
        phase: &str,
        error_code: Option<&str>,
        error_message: Option<&str>,
        duration_ms: i64,
    ) {
        let conn = match open_database(Path::new(project_root)) {
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
                status,
                phase,
                error_code,
                error_message,
                duration_ms,
                now_iso(),
                request_id
            ],
        );
    }

    pub fn touch_pipeline_phase(&self, project_root: &str, request_id: &str, phase: &str) {
        let conn = match open_database(Path::new(project_root)) {
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
}
