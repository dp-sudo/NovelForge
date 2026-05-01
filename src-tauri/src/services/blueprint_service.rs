use rusqlite::{params, Connection};
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
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn.prepare("SELECT id, project_id, step_key, title, COALESCE(content,''), COALESCE(content_path,''), status, ai_generated, completed_at, created_at, updated_at FROM blueprint_steps WHERE project_id = ?1")
            .map_err(query_steps_error)?;
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
            .map_err(query_steps_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(query_steps_error)?;
        Ok(steps)
    }

    pub fn save_step(
        &self,
        project_root: &str,
        input: SaveBlueprintStepInput,
    ) -> Result<BlueprintStep, AppErrorDto> {
        let conn = open_project_database(project_root)?;
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
            .map_err(update_step_error)?;
        } else {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO blueprint_steps(id, project_id, step_key, title, content, content_path, status, ai_generated, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![id, project_id, input.step_key, "", input.content, "", status, ai_gen as i32, now, now],
            )
            .map_err(insert_step_error)?;
        }

        load_step_by_key(&conn, &project_id, &input.step_key)
    }

    pub fn mark_completed(&self, project_root: &str, step_key: &str) -> Result<(), AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        conn.execute(
            "UPDATE blueprint_steps SET status = 'completed', completed_at = ?1, updated_at = ?2 WHERE project_id = ?3 AND step_key = ?4",
            params![now, now, project_id, step_key],
        )
        .map_err(mark_completed_error)?;
        Ok(())
    }

    pub fn reset_step(&self, project_root: &str, step_key: &str) -> Result<(), AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        conn.execute(
            "UPDATE blueprint_steps SET content = '', status = 'not_started', ai_generated = 0, completed_at = NULL, updated_at = ?1 WHERE project_id = ?2 AND step_key = ?3",
            params![now, project_id, step_key],
        )
        .map_err(reset_step_error)?;
        Ok(())
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

fn open_project_database(project_root: &str) -> Result<Connection, AppErrorDto> {
    let normalized_root = normalize_project_root(project_root)?;
    open_database(Path::new(normalized_root)).map_err(|e| {
        AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
    })
}

fn query_steps_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("QUERY_FAILED", "查询蓝图步骤失败", true).with_detail(err.to_string())
}

fn update_step_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATE_FAILED", "更新蓝图步骤失败", true).with_detail(err.to_string())
}

fn mark_completed_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATE_FAILED", "标记完成失败", true).with_detail(err.to_string())
}

fn reset_step_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATE_FAILED", "重置蓝图步骤失败", true).with_detail(err.to_string())
}

fn insert_step_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("INSERT_FAILED", "创建蓝图步骤失败", true).with_detail(err.to_string())
}

fn load_step_by_key(
    conn: &Connection,
    project_id: &str,
    step_key: &str,
) -> Result<BlueprintStep, AppErrorDto> {
    conn.query_row(
        "SELECT id, project_id, step_key, title, COALESCE(content,''), COALESCE(content_path,''), status, ai_generated, completed_at, created_at, updated_at FROM blueprint_steps WHERE project_id = ?1 AND step_key = ?2",
        params![project_id, step_key],
        |row| {
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
        },
    )
    .map_err(query_steps_error)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    use super::{BlueprintService, SaveBlueprintStepInput};
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
    fn blueprint_save_and_mark_complete_succeeds() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "蓝图测试".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        bs.save_step(
            &project.project_root,
            SaveBlueprintStepInput {
                step_key: "step-01-anchor".into(),
                content: "核心灵感：秩序与代价。".into(),
                ai_generated: None,
            },
        )
        .expect("save step");

        let steps = bs.list_steps(&project.project_root).expect("list steps");
        let step = steps
            .iter()
            .find(|s| s.step_key == "step-01-anchor")
            .unwrap();
        assert_eq!(step.content, "核心灵感：秩序与代价。");

        bs.mark_completed(&project.project_root, "step-01-anchor")
            .expect("mark completed");
        let steps = bs.list_steps(&project.project_root).expect("list steps");
        let step = steps
            .iter()
            .find(|s| s.step_key == "step-01-anchor")
            .unwrap();
        assert_eq!(step.status, "completed");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn blueprint_methods_accept_trimmed_project_root() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "蓝图路径空白测试".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let wrapped_root = format!("  {}  ", project.project_root);
        let saved = bs
            .save_step(
                &wrapped_root,
                SaveBlueprintStepInput {
                    step_key: "step-01-anchor".into(),
                    content: "测试内容".into(),
                    ai_generated: None,
                },
            )
            .expect("save step with trimmed root");
        assert_eq!(saved.step_key, "step-01-anchor");

        let steps = bs
            .list_steps(&wrapped_root)
            .expect("list steps with trimmed root");
        assert_eq!(steps.len(), 1);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn blueprint_methods_reject_blank_project_root() {
        let bs = BlueprintService;
        let err = bs
            .list_steps("   ")
            .expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
