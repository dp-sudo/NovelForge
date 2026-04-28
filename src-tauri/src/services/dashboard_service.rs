use rusqlite::params;
use serde::Serialize;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_words: i64,
    pub chapter_count: i64,
    pub character_count: i64,
    pub world_rule_count: i64,
    pub plot_node_count: i64,
    pub open_issue_count: i64,
    pub completed_blueprint_count: i64,
    pub total_blueprint_steps: i64,
}

#[derive(Default)]
pub struct DashboardService;

impl DashboardService {
    pub fn get_stats(&self, project_root: &str) -> Result<DashboardStats, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;

        let total_words: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(current_words), 0) FROM chapters WHERE project_id = ?1 AND is_deleted = 0",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let chapter_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chapters WHERE project_id = ?1 AND is_deleted = 0",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let character_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM characters WHERE project_id = ?1 AND is_deleted = 0",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let world_rule_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM world_rules WHERE project_id = ?1 AND is_deleted = 0",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let plot_node_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM plot_nodes WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let open_issue_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM consistency_issues WHERE project_id = ?1 AND status = 'open'",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let completed_blueprint_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blueprint_steps WHERE project_id = ?1 AND status = 'completed'",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let total_blueprint_steps: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blueprint_steps WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(8);

        Ok(DashboardStats {
            total_words,
            chapter_count,
            character_count,
            world_rule_count,
            plot_node_count,
            open_issue_count,
            completed_blueprint_count,
            total_blueprint_steps,
        })
    }
}
