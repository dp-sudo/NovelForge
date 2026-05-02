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
pub struct WorldRuleRecord {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub constraint_level: String,
    pub related_entities: String,
    pub examples: Option<String>,
    pub contradiction_policy: Option<String>,
    pub is_deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorldRuleInput {
    pub title: String,
    pub category: String,
    pub description: String,
    pub constraint_level: String,
    pub related_entities: Option<Vec<String>>,
    pub examples: Option<String>,
    pub contradiction_policy: Option<String>,
}

#[derive(Default)]
pub struct WorldService;

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

impl WorldService {
    pub fn list(&self, project_root: &str) -> Result<Vec<WorldRuleRecord>, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id, project_id, title, category, description, constraint_level, COALESCE(related_entities,'[]'), examples, contradiction_policy, is_deleted, created_at, updated_at FROM world_rules WHERE project_id = ?1 AND is_deleted = 0")
            .map_err(query_world_rules_error)?;
        let rules = stmt
            .query_map(params![project_id], |row| {
                Ok(WorldRuleRecord {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    category: row.get(3)?,
                    description: row.get(4)?,
                    constraint_level: row.get(5)?,
                    related_entities: row.get(6)?,
                    examples: row.get(7)?,
                    contradiction_policy: row.get(8)?,
                    is_deleted: row.get::<_, i32>(9)? != 0,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(query_world_rules_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(query_world_rules_error)?;
        Ok(rules)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreateWorldRuleInput,
    ) -> Result<String, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let related =
            serde_json::to_string(&input.related_entities.unwrap_or_default()).map_err(|e| {
                AppErrorDto::new("SERIALIZE_ERROR", "序列化关联实体失败", true)
                    .with_detail(e.to_string())
            })?;
        conn.execute(
            "INSERT INTO world_rules(id, project_id, title, category, description, constraint_level, related_entities, examples, contradiction_policy, is_deleted, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0,?10,?11)",
            params![id, project_id, input.title, input.category, input.description, input.constraint_level, related, input.examples, input.contradiction_policy, now, now],
        )
        .map_err(insert_world_rule_error)?;
        insert_manual_provenance(&conn, &project_id, "world_rule", &id)?;
        Ok(id)
    }

    pub fn soft_delete(&self, project_root: &str, id: &str) -> Result<(), AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let now = now_iso();
        conn.execute(
            "UPDATE world_rules SET is_deleted = 1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(delete_world_rule_error)?;
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

fn query_world_rules_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("QUERY_FAILED", "查询世界规则失败", true).with_detail(err.to_string())
}

fn insert_world_rule_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("INSERT_FAILED", "创建世界规则失败", true).with_detail(err.to_string())
}

fn delete_world_rule_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("DELETE_FAILED", "删除世界规则失败", true).with_detail(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    use super::{CreateWorldRuleInput, WorldService};
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
                name: "世界观测试".into(),
                author: None,
                genre: "奇幻".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");
        project.project_root
    }

    #[test]
    fn world_rules_accept_trimmed_project_root() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let wrapped_root = format!("  {}  ", project_root);
        let service = WorldService;

        let id = service
            .create(
                &wrapped_root,
                CreateWorldRuleInput {
                    title: "代价法则".into(),
                    category: "magic".into(),
                    description: "任何强力法术都需要付出代价".into(),
                    constraint_level: "hard".into(),
                    related_entities: Some(vec!["法师公会".into()]),
                    examples: None,
                    contradiction_policy: None,
                },
            )
            .expect("create world rule with trimmed root");
        assert!(!id.is_empty());

        let rules = service
            .list(&wrapped_root)
            .expect("list world rules with trimmed root");
        assert_eq!(rules.len(), 1);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn world_rules_reject_blank_project_root() {
        let service = WorldService;
        let err = service
            .list("   ")
            .expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
