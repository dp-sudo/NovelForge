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
pub struct GlossaryTermRecord {
    pub id: String,
    pub project_id: String,
    pub term: String,
    pub term_type: String,
    pub aliases: String,
    pub description: Option<String>,
    pub locked: bool,
    pub banned: bool,
    pub preferred_usage: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGlossaryTermInput {
    pub term: String,
    pub term_type: String,
    pub aliases: Option<Vec<String>>,
    pub description: Option<String>,
    pub locked: Option<bool>,
    pub banned: Option<bool>,
}

#[derive(Default)]
pub struct GlossaryService;

impl GlossaryService {
    pub fn list(&self, project_root: &str) -> Result<Vec<GlossaryTermRecord>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id, project_id, term, term_type, COALESCE(aliases,'[]'), description, locked, banned, preferred_usage, created_at, updated_at FROM glossary_terms WHERE project_id = ?1")
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询名词失败", true).with_detail(e.to_string()))?;
        let terms = stmt
            .query_map(params![project_id], |row| {
                Ok(GlossaryTermRecord {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    term: row.get(2)?,
                    term_type: row.get(3)?,
                    aliases: row.get(4)?,
                    description: row.get(5)?,
                    locked: row.get::<_, i32>(6)? != 0,
                    banned: row.get::<_, i32>(7)? != 0,
                    preferred_usage: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询名词失败", true).with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询名词失败", true).with_detail(e.to_string())
            })?;
        Ok(terms)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreateGlossaryTermInput,
    ) -> Result<String, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let aliases = serde_json::to_string(&input.aliases.unwrap_or_default()).unwrap_or_default();
        let locked = if input.locked.unwrap_or(false) { 1 } else { 0 };
        let banned = if input.banned.unwrap_or(false) { 1 } else { 0 };
        conn.execute(
            "INSERT INTO glossary_terms(id, project_id, term, term_type, aliases, description, locked, banned, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![id, project_id, input.term, input.term_type, aliases, input.description, locked, banned, now, now],
        )
        .map_err(|e| AppErrorDto::new("INSERT_FAILED", "创建名词失败", true).with_detail(e.to_string()))?;
        Ok(id)
    }
}
