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

impl GlossaryService {
    pub fn list(&self, project_root: &str) -> Result<Vec<GlossaryTermRecord>, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id, project_id, term, term_type, COALESCE(aliases,'[]'), description, locked, banned, preferred_usage, created_at, updated_at FROM glossary_terms WHERE project_id = ?1")
            .map_err(query_terms_error)?;
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
            .map_err(query_terms_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(query_terms_error)?;
        Ok(terms)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreateGlossaryTermInput,
    ) -> Result<String, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let aliases = serde_json::to_string(&input.aliases.unwrap_or_default()).map_err(|e| {
            AppErrorDto::new("SERIALIZE_ERROR", "序列化别名失败", true).with_detail(e.to_string())
        })?;
        let locked = if input.locked.unwrap_or(false) { 1 } else { 0 };
        let banned = if input.banned.unwrap_or(false) { 1 } else { 0 };
        conn.execute(
            "INSERT INTO glossary_terms(id, project_id, term, term_type, aliases, description, locked, banned, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![id, project_id, input.term, input.term_type, aliases, input.description, locked, banned, now, now],
        )
        .map_err(insert_term_error)?;
        insert_manual_provenance(&conn, &project_id, "glossary_term", &id)?;
        Ok(id)
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

fn query_terms_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("QUERY_FAILED", "查询名词失败", true).with_detail(err.to_string())
}

fn insert_term_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("INSERT_FAILED", "创建名词失败", true).with_detail(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    use super::{CreateGlossaryTermInput, GlossaryService};
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
                name: "名词测试".into(),
                author: None,
                genre: "悬疑".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");
        project.project_root
    }

    #[test]
    fn glossary_methods_accept_trimmed_project_root() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let wrapped_root = format!("  {}  ", project_root);
        let service = GlossaryService;

        let id = service
            .create(
                &wrapped_root,
                CreateGlossaryTermInput {
                    term: "夜潮".into(),
                    term_type: "concept".into(),
                    aliases: Some(vec!["夜之潮汐".into()]),
                    description: Some("秘密组织".into()),
                    locked: Some(true),
                    banned: Some(false),
                },
            )
            .expect("create glossary term with trimmed root");
        assert!(!id.is_empty());

        let terms = service
            .list(&wrapped_root)
            .expect("list glossary terms with trimmed root");
        assert_eq!(terms.len(), 1);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn glossary_methods_reject_blank_project_root() {
        let service = GlossaryService;
        let err = service
            .list("   ")
            .expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
