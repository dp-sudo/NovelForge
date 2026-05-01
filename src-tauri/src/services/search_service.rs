use std::path::Path;

use rusqlite::params;
use serde::Serialize;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub body_snippet: String,
    pub rank: f64,
}

#[derive(Default)]
pub struct SearchService;

impl SearchService {
    /// Search across all indexed entities. Returns results ranked by relevance.
    pub fn search(
        &self,
        project_root: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, AppErrorDto> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        let conn = open_project_database(project_root)?;

        let fts_query = query
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut stmt = conn
            .prepare("SELECT entity_type, entity_id, title, snippet(search_index, 1, '…', '…', 32, 16) as body_snippet, rank FROM search_index WHERE search_index MATCH ?1 ORDER BY rank LIMIT ?2")
            .map_err(search_query_error)?;

        let results = stmt
            .query_map(params![fts_query, limit as i64], |row| {
                Ok(SearchResult {
                    entity_type: row.get(0)?,
                    entity_id: row.get(1)?,
                    title: row.get(2)?,
                    body_snippet: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    rank: row.get::<_, f64>(4)?,
                })
            })
            .map_err(search_query_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(search_query_error)?;

        Ok(results)
    }

    /// Rebuild the entire search index from project data.
    pub fn rebuild_index(&self, project_root: &str) -> Result<usize, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        conn.execute("DELETE FROM search_index", [])
            .map_err(search_rebuild_error)?;
        let mut indexed = 0usize;

        for (stmt_sql, etype, title_col, body_col) in [
            ("SELECT id, title, COALESCE(summary,'') FROM chapters WHERE is_deleted=0", "chapter", 1usize, 2usize),
            ("SELECT id, name, COALESCE(motivation,'')||' '||COALESCE(identity_text,'') FROM characters WHERE is_deleted=0", "character", 1, 2),
            ("SELECT id, title, COALESCE(description,'') FROM world_rules WHERE is_deleted=0", "world_rule", 1, 2),
            ("SELECT id, term, COALESCE(description,'') FROM glossary_terms", "glossary", 1, 2),
            ("SELECT id, title, COALESCE(goal,'')||' '||COALESCE(conflict,'') FROM plot_nodes", "plot_node", 1, 2),
        ] {
            let mut stmt = conn.prepare(stmt_sql).map_err(search_rebuild_error)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(title_col)?,
                        row.get::<_, String>(body_col)?,
                    ))
                })
                .map_err(search_rebuild_error)?;
            for row in rows {
                let row = row.map_err(search_rebuild_error)?;
                conn.execute(
                    "INSERT INTO search_index(entity_type, entity_id, title, body) VALUES (?1, ?2, ?3, ?4)",
                    params![etype, row.0, row.1, row.2],
                )
                .map_err(search_rebuild_error)?;
                indexed += 1;
            }
        }
        Ok(indexed)
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

fn search_query_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("SEARCH_QUERY_FAILED", "搜索查询失败", true).with_detail(err.to_string())
}

fn search_rebuild_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("SEARCH_REBUILD_FAILED", "重建索引失败", true).with_detail(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use super::SearchService;
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
    fn search_methods_accept_trimmed_project_root() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let ss = SearchService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "搜索路径空白测试".into(),
                author: None,
                genre: "悬疑".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");
        let wrapped_root = format!("  {}  ", project.project_root);

        let _indexed = ss
            .rebuild_index(&wrapped_root)
            .expect("rebuild with trimmed root");

        let _ = ss
            .search(&wrapped_root, "开端", 10)
            .expect("search with trimmed root");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn search_methods_reject_blank_project_root() {
        let ss = SearchService;
        let err = ss
            .search("   ", "关键字", 10)
            .expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
