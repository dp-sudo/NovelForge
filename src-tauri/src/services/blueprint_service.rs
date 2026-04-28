use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintStep {
    pub id: String,
    pub project_id: String,
    pub step_key: String,
    pub title: String,
    pub content: String,
    pub content_path: String,
    pub status: String,
    pub ai_generated: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBlueprintStepInput {
    pub step_key: String,
    pub content: String,
    pub ai_generated: Option<bool>,
}

#[derive(Default)]
pub struct BlueprintService;

impl BlueprintService {
    pub fn list_steps(&self, project_root: &str) -> Result<Vec<BlueprintStep>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id, project_id, step_key, title, COALESCE(content,''), COALESCE(content_path,''), status, ai_generated, completed_at, created_at, updated_at FROM blueprint_steps WHERE project_id = ?1")
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询蓝图步骤失败", true).with_detail(e.to_string()))?;
        let steps = stmt
            .query_map(params![project_id], |row| {
                Ok(BlueprintStep {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    step_key: row.get(2)?,
                    title: row.get(3)?,
                    content: row.get(4)?,
                    content_path: row.get(5)?,
                    status: row.get(6)?,
                    ai_generated: row.get::<_, i32>(7)? != 0,
                    completed_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询蓝图步骤失败", true).with_detail(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询蓝图步骤失败", true).with_detail(e.to_string()))?;
        Ok(steps)
    }

    pub fn save_step(
        &self,
        project_root: &str,
        input: SaveBlueprintStepInput,
    ) -> Result<BlueprintStep, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        let ai_gen = input.ai_generated.unwrap_or(false);
        let status = if input.content.trim().is_empty() {
            "not_started"
        } else {
            "in_progress"
        };

        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM blueprint_steps WHERE project_id = ?1 AND step_key = ?2",
                params![project_id, input.step_key],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            conn.execute(
                "UPDATE blueprint_steps SET content = ?1, status = ?2, ai_generated = ?3, updated_at = ?4 WHERE id = ?5",
                params![input.content, status, ai_gen as i32, now, id],
            )
            .map_err(|e| AppErrorDto::new("UPDATE_FAILED", "更新蓝图步骤失败", true).with_detail(e.to_string()))?;
        } else {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO blueprint_steps(id, project_id, step_key, title, content, content_path, status, ai_generated, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![id, project_id, input.step_key, "", input.content, "", status, ai_gen as i32, now, now],
            )
            .map_err(|e| AppErrorDto::new("INSERT_FAILED", "创建蓝图步骤失败", true).with_detail(e.to_string()))?;
        }

        self.list_steps(project_root)
            .map(|steps| steps.into_iter().find(|s| s.step_key == input.step_key).unwrap())
    }

    pub fn mark_completed(
        &self,
        project_root: &str,
        step_key: &str,
    ) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        conn.execute(
            "UPDATE blueprint_steps SET status = 'completed', completed_at = ?1, updated_at = ?2 WHERE project_id = ?3 AND step_key = ?4",
            params![now, now, project_id, step_key],
        )
        .map_err(|e| AppErrorDto::new("UPDATE_FAILED", "标记完成失败", true).with_detail(e.to_string()))?;
        Ok(())
    }

    pub fn reset_step(&self, project_root: &str, step_key: &str) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        conn.execute(
            "UPDATE blueprint_steps SET content = '', status = 'not_started', ai_generated = 0, completed_at = NULL, updated_at = ?1 WHERE project_id = ?2 AND step_key = ?3",
            params![now, project_id, step_key],
        )
        .map_err(|e| AppErrorDto::new("UPDATE_FAILED", "重置蓝图步骤失败", true).with_detail(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    use crate::services::project_service::{CreateProjectInput, ProjectService};
    use super::{BlueprintService, SaveBlueprintStepInput};

    fn create_temp_workspace() -> PathBuf {
        let w = std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&w).expect("create temp workspace");
        w
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn blueprint_save_and_mark_complete_succeeds() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps.create_project(CreateProjectInput {
            name: "蓝图测试".into(), author: None, genre: "测试".into(),
            target_words: None, save_directory: ws.to_string_lossy().into(),
        }).expect("project created");

        bs.save_step(&project.project_root, SaveBlueprintStepInput {
            step_key: "step-01-anchor".into(),
            content: "核心灵感：秩序与代价。".into(),
            ai_generated: None,
        }).expect("save step");

        let steps = bs.list_steps(&project.project_root).expect("list steps");
        let step = steps.iter().find(|s| s.step_key == "step-01-anchor").unwrap();
        assert_eq!(step.content, "核心灵感：秩序与代价。");

        bs.mark_completed(&project.project_root, "step-01-anchor").expect("mark completed");
        let steps = bs.list_steps(&project.project_root).expect("list steps");
        let step = steps.iter().find(|s| s.step_key == "step-01-anchor").unwrap();
        assert_eq!(step.status, "completed");

        remove_temp_workspace(&ws);
    }
}
