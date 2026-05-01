use rusqlite::params;
use serde::Serialize;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
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
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;

        let total_words = query_i64_or_default(
            &conn,
            "SELECT COALESCE(SUM(current_words), 0) FROM chapters WHERE project_id = ?1 AND is_deleted = 0",
            &project_id,
            0,
        );
        let chapter_count = query_i64_or_default(
            &conn,
            "SELECT COUNT(*) FROM chapters WHERE project_id = ?1 AND is_deleted = 0",
            &project_id,
            0,
        );
        let character_count = query_i64_or_default(
            &conn,
            "SELECT COUNT(*) FROM characters WHERE project_id = ?1 AND is_deleted = 0",
            &project_id,
            0,
        );
        let world_rule_count = query_i64_or_default(
            &conn,
            "SELECT COUNT(*) FROM world_rules WHERE project_id = ?1 AND is_deleted = 0",
            &project_id,
            0,
        );
        let plot_node_count = query_i64_or_default(
            &conn,
            "SELECT COUNT(*) FROM plot_nodes WHERE project_id = ?1",
            &project_id,
            0,
        );
        let open_issue_count = query_i64_or_default(
            &conn,
            "SELECT COUNT(*) FROM consistency_issues WHERE project_id = ?1 AND status = 'open'",
            &project_id,
            0,
        );
        let completed_blueprint_count = query_i64_or_default(
            &conn,
            "SELECT COUNT(*) FROM blueprint_steps WHERE project_id = ?1 AND status = 'completed'",
            &project_id,
            0,
        );
        let total_blueprint_steps = query_i64_or_default(
            &conn,
            "SELECT COUNT(*) FROM blueprint_steps WHERE project_id = ?1",
            &project_id,
            8,
        );

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

fn open_project_database(project_root: &str) -> Result<rusqlite::Connection, AppErrorDto> {
    let normalized_root = normalize_project_root(project_root)?;
    open_database(Path::new(normalized_root)).map_err(|e| {
        AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
    })
}

fn query_i64_or_default(
    conn: &rusqlite::Connection,
    sql: &str,
    project_id: &str,
    default: i64,
) -> i64 {
    conn.query_row(sql, params![project_id], |row| row.get(0))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use super::DashboardService;
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let w = std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&w).expect("create temp workspace");
        w
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn dashboard_methods_accept_trimmed_project_root() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let ds = DashboardService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "仪表盘路径空白测试".into(),
                author: None,
                genre: "悬疑".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");
        let wrapped_root = format!("  {}  ", project.project_root);

        let stats = ds
            .get_stats(&wrapped_root)
            .expect("get stats with trimmed root");
        assert_eq!(stats.chapter_count, 0);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn dashboard_methods_reject_blank_project_root() {
        let ds = DashboardService;
        let err = ds
            .get_stats("   ")
            .expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
