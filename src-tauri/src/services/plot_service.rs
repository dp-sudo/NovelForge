use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_project_db;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotNodeRecord {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub node_type: String,
    pub sort_order: i64,
    pub goal: Option<String>,
    pub conflict: Option<String>,
    pub emotional_curve: Option<String>,
    pub status: String,
    pub related_characters: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlotNodeInput {
    pub title: String,
    pub node_type: String,
    pub sort_order: i64,
    pub goal: Option<String>,
    pub conflict: Option<String>,
    pub emotional_curve: Option<String>,
    pub status: Option<String>,
    pub related_characters: Option<Vec<String>>,
}

#[derive(Default)]
pub struct PlotService;

impl PlotService {
    pub fn list(&self, project_root: &str) -> Result<Vec<PlotNodeRecord>, AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id, project_id, title, node_type, sort_order, goal, conflict, emotional_curve, status, COALESCE(related_characters,'[]'), created_at, updated_at FROM plot_nodes WHERE project_id = ?1 ORDER BY sort_order")
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询剧情节点失败", true).with_detail(e.to_string()))?;
        let nodes = stmt
            .query_map(params![project_id], |row| {
                Ok(PlotNodeRecord {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    node_type: row.get(3)?,
                    sort_order: row.get(4)?,
                    goal: row.get(5)?,
                    conflict: row.get(6)?,
                    emotional_curve: row.get(7)?,
                    status: row.get(8)?,
                    related_characters: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询剧情节点失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询剧情节点失败", true)
                    .with_detail(e.to_string())
            })?;
        Ok(nodes)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreatePlotNodeInput,
    ) -> Result<String, AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let rc = serde_json::to_string(&input.related_characters.unwrap_or_default())
            .unwrap_or_default();
        let status = input.status.unwrap_or_else(|| "planning".to_string());
        conn.execute(
            "INSERT INTO plot_nodes(id, project_id, title, node_type, sort_order, goal, conflict, emotional_curve, status, related_characters, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![id, project_id, input.title, input.node_type, input.sort_order, input.goal, input.conflict, input.emotional_curve, status, rc, now, now],
        )
        .map_err(|e| AppErrorDto::new("INSERT_FAILED", "创建剧情节点失败", true).with_detail(e.to_string()))?;
        Ok(id)
    }

    pub fn reorder(&self, project_root: &str, ordered_ids: Vec<String>) -> Result<(), AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let now = now_iso();
        for (i, node_id) in ordered_ids.iter().enumerate() {
            let order = (i + 1) as i64;
            conn.execute(
                "UPDATE plot_nodes SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                params![order, now, node_id],
            )
            .map_err(|e| {
                AppErrorDto::new("REORDER_FAILED", "重排序失败", true).with_detail(e.to_string())
            })?;
        }
        Ok(())
    }

    pub fn next_sort_order(&self, project_root: &str) -> Result<i64, AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let project_id = get_project_id(&conn)?;
        conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM plot_nodes WHERE project_id = ?1",
            params![project_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| {
            AppErrorDto::new("PIPELINE_DB_QUERY_FAILED", "读取剧情节点顺序失败", true)
                .with_detail(e.to_string())
        })
    }
}


#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    use super::{CreatePlotNodeInput, PlotService};
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
    fn plot_create_reorder_list_succeeds() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let pl = PlotService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "剧情测试".into(),
                author: None,
                genre: "悬疑".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let id1 = pl
            .create(
                &project.project_root,
                CreatePlotNodeInput {
                    title: "开端".into(),
                    node_type: "开端".into(),
                    sort_order: 1,
                    ..Default::default()
                },
            )
            .expect("create node 1");
        let id2 = pl
            .create(
                &project.project_root,
                CreatePlotNodeInput {
                    title: "高潮".into(),
                    node_type: "高潮".into(),
                    sort_order: 2,
                    ..Default::default()
                },
            )
            .expect("create node 2");

        let nodes = pl.list(&project.project_root).expect("list nodes");
        assert_eq!(nodes.len(), 2);

        pl.reorder(&project.project_root, vec![id2, id1])
            .expect("reorder");
        let nodes = pl.list(&project.project_root).expect("list nodes");
        assert_eq!(nodes[0].sort_order, 1);
        assert_eq!(nodes[0].title, "高潮");

        remove_temp_workspace(&ws);
    }
}
