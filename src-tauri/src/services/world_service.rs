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
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id, project_id, title, category, description, constraint_level, COALESCE(related_entities,'[]'), examples, contradiction_policy, is_deleted, created_at, updated_at FROM world_rules WHERE project_id = ?1 AND is_deleted = 0")
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询世界规则失败", true).with_detail(e.to_string()))?;
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
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询世界规则失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询世界规则失败", true)
                    .with_detail(e.to_string())
            })?;
        Ok(rules)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreateWorldRuleInput,
    ) -> Result<String, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let related =
            serde_json::to_string(&input.related_entities.unwrap_or_default()).unwrap_or_default();
        conn.execute(
            "INSERT INTO world_rules(id, project_id, title, category, description, constraint_level, related_entities, examples, contradiction_policy, is_deleted, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0,?10,?11)",
            params![id, project_id, input.title, input.category, input.description, input.constraint_level, related, input.examples, input.contradiction_policy, now, now],
        )
        .map_err(|e| AppErrorDto::new("INSERT_FAILED", "创建世界规则失败", true).with_detail(e.to_string()))?;
        insert_manual_provenance(&conn, &project_id, "world_rule", &id)?;
        Ok(id)
    }

    pub fn soft_delete(&self, project_root: &str, id: &str) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let now = now_iso();
        conn.execute(
            "UPDATE world_rules SET is_deleted = 1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| {
            AppErrorDto::new("DELETE_FAILED", "删除世界规则失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }
}
