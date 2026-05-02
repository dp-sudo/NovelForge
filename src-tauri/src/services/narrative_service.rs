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
pub struct NarrativeObligation {
    pub id: String,
    pub project_id: String,
    pub obligation_type: String,
    pub description: String,
    pub planted_chapter_id: Option<String>,
    pub expected_payoff_chapter_id: Option<String>,
    pub actual_payoff_chapter_id: Option<String>,
    pub payoff_status: String,
    pub severity: String,
    pub related_entities: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateObligationInput {
    pub obligation_type: String,
    pub description: String,
    pub planted_chapter_id: Option<String>,
    pub expected_payoff_chapter_id: Option<String>,
    pub actual_payoff_chapter_id: Option<String>,
    pub payoff_status: Option<String>,
    pub severity: Option<String>,
    pub related_entities: Option<String>,
}

#[derive(Default)]
pub struct NarrativeService;

fn insert_manual_provenance(
    conn: &Connection,
    project_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), AppErrorDto> {
    conn.execute(
        "INSERT INTO entity_provenance(id, project_id, entity_type, entity_id, source_kind, source_ref, request_id, created_at)
         VALUES (?1, ?2, ?3, ?4, 'user_input', ?5, NULL, ?6)",
        params![
            Uuid::new_v4().to_string(),
            project_id,
            entity_type,
            entity_id,
            format!("manual_crud:{entity_type}:create"),
            now_iso(),
        ],
    )
    .map_err(|e| {
        AppErrorDto::new("INSERT_FAILED", "写入来源轨迹失败", true).with_detail(e.to_string())
    })?;
    Ok(())
}

impl NarrativeService {
    pub fn list(&self, project_root: &str) -> Result<Vec<NarrativeObligation>, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, obligation_type, description, \
                 planted_chapter_id, expected_payoff_chapter_id, actual_payoff_chapter_id, \
                 payoff_status, severity, related_entities, created_at, updated_at \
                 FROM narrative_obligations WHERE project_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(query_obligation_error)?;
        let obligations = stmt
            .query_map(params![project_id], |row| {
                Ok(NarrativeObligation {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    obligation_type: row.get(2)?,
                    description: row.get(3)?,
                    planted_chapter_id: row.get::<_, Option<String>>(4)?,
                    expected_payoff_chapter_id: row.get::<_, Option<String>>(5)?,
                    actual_payoff_chapter_id: row.get::<_, Option<String>>(6)?,
                    payoff_status: row.get(7)?,
                    severity: row.get(8)?,
                    related_entities: row.get::<_, Option<String>>(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(query_obligation_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(query_obligation_error)?;
        Ok(obligations)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreateObligationInput,
    ) -> Result<String, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let status = input.payoff_status.unwrap_or_else(|| "open".to_string());
        let severity = input.severity.unwrap_or_else(|| "medium".to_string());
        conn.execute(
            "INSERT INTO narrative_obligations(id, project_id, obligation_type, description, \
             planted_chapter_id, expected_payoff_chapter_id, actual_payoff_chapter_id, \
             payoff_status, severity, related_entities, created_at, updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                id,
                project_id,
                input.obligation_type,
                input.description,
                input.planted_chapter_id,
                input.expected_payoff_chapter_id,
                input.actual_payoff_chapter_id,
                status,
                severity,
                input.related_entities,
                now,
                now
            ],
        )
        .map_err(insert_obligation_error)?;
        insert_manual_provenance(&conn, &project_id, "narrative_obligation", &id)?;
        Ok(id)
    }

    pub fn update_status(
        &self,
        project_root: &str,
        id: &str,
        status: &str,
    ) -> Result<(), AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let now = now_iso();
        conn.execute(
            "UPDATE narrative_obligations SET payoff_status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )
        .map_err(update_obligation_status_error)?;
        Ok(())
    }

    pub fn delete(&self, project_root: &str, id: &str) -> Result<(), AppErrorDto> {
        let conn = open_project_database(project_root)?;
        conn.execute(
            "DELETE FROM narrative_obligations WHERE id = ?1",
            params![id],
        )
        .map_err(delete_obligation_error)?;
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

fn query_obligation_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("QUERY_FAILED", "查询叙事义务失败", true).with_detail(err.to_string())
}

fn insert_obligation_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("INSERT_FAILED", "创建叙事义务失败", true).with_detail(err.to_string())
}

fn update_obligation_status_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATE_FAILED", "更新叙事义务状态失败", true).with_detail(err.to_string())
}

fn delete_obligation_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("DELETE_FAILED", "删除叙事义务失败", true).with_detail(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    use super::{CreateObligationInput, NarrativeService};
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let w = std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&w).expect("create temp workspace");
        w
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn create_test_project(ws: &Path) -> String {
        let ps = ProjectService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "叙事测试".into(),
                author: None,
                genre: "悬疑".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");
        project.project_root
    }

    #[test]
    fn narrative_obligation_create_and_list_succeeds() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let ns = NarrativeService;

        let id = ns
            .create(
                &project_root,
                CreateObligationInput {
                    obligation_type: "foreshadowing".into(),
                    description: "主角在第一章发现的钥匙".into(),
                    planted_chapter_id: None,
                    expected_payoff_chapter_id: None,
                    actual_payoff_chapter_id: None,
                    payoff_status: Some("open".into()),
                    severity: Some("high".into()),
                    related_entities: Some(r#"["主角"]"#.into()),
                },
            )
            .expect("create obligation");
        assert!(!id.is_empty());

        let list = ns.list(&project_root).expect("list obligations");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].obligation_type, "foreshadowing");
        assert_eq!(list[0].payoff_status, "open");
        assert_eq!(list[0].severity, "high");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn narrative_obligation_update_status_succeeds() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let ns = NarrativeService;

        let id = ns
            .create(
                &project_root,
                CreateObligationInput {
                    obligation_type: "promise".into(),
                    description: "主角发誓复仇".into(),
                    planted_chapter_id: None,
                    expected_payoff_chapter_id: None,
                    actual_payoff_chapter_id: None,
                    payoff_status: Some("open".into()),
                    severity: Some("medium".into()),
                    related_entities: None,
                },
            )
            .expect("create obligation");

        ns.update_status(&project_root, &id, "paid_off")
            .expect("update status");

        let list = ns.list(&project_root).expect("list obligations");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].payoff_status, "paid_off");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn narrative_obligation_delete_succeeds() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let ns = NarrativeService;

        let id = ns
            .create(
                &project_root,
                CreateObligationInput {
                    obligation_type: "mystery".into(),
                    description: "密室之谜".into(),
                    planted_chapter_id: None,
                    expected_payoff_chapter_id: None,
                    actual_payoff_chapter_id: None,
                    payoff_status: None,
                    severity: None,
                    related_entities: None,
                },
            )
            .expect("create obligation");

        ns.delete(&project_root, &id).expect("delete obligation");
        let list = ns.list(&project_root).expect("list obligations");
        assert_eq!(list.len(), 0);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn narrative_obligation_defaults_to_open_and_medium() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let ns = NarrativeService;

        let id = ns
            .create(
                &project_root,
                CreateObligationInput {
                    obligation_type: "setup".into(),
                    description: "伏笔".into(),
                    planted_chapter_id: None,
                    expected_payoff_chapter_id: None,
                    actual_payoff_chapter_id: None,
                    payoff_status: None,
                    severity: None,
                    related_entities: None,
                },
            )
            .expect("create obligation");

        let list = ns.list(&project_root).expect("list obligations");
        let ob = list.iter().find(|o| o.id == id).expect("obligation found");
        assert_eq!(ob.payoff_status, "open");
        assert_eq!(ob.severity, "medium");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn narrative_methods_accept_trimmed_project_root() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let wrapped_root = format!("  {}  ", project_root);
        let ns = NarrativeService;

        let id = ns
            .create(
                &wrapped_root,
                CreateObligationInput {
                    obligation_type: "setup".into(),
                    description: "测试内容".into(),
                    planted_chapter_id: None,
                    expected_payoff_chapter_id: None,
                    actual_payoff_chapter_id: None,
                    payoff_status: None,
                    severity: None,
                    related_entities: None,
                },
            )
            .expect("create obligation with trimmed root");
        assert!(!id.is_empty());

        let list = ns
            .list(&wrapped_root)
            .expect("list obligations with trimmed root");
        assert_eq!(list.len(), 1);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn narrative_methods_reject_blank_project_root() {
        let ns = NarrativeService;
        let err = ns.list("   ").expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
