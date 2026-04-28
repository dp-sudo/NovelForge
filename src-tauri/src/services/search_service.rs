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
    /// Index a single entity into the FTS5 search index.
    pub fn index_entity(
        &self,
        project_root: &str,
        entity_type: &str,
        entity_id: &str,
        title: &str,
        body: &str,
    ) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        conn.execute(
            "DELETE FROM search_index WHERE entity_type = ?1 AND entity_id = ?2",
            params![entity_type, entity_id],
        )
        .map_err(|e| {
            AppErrorDto::new("SEARCH_INDEX_FAILED", "索引写入失败", true).with_detail(e.to_string())
        })?;
        conn.execute(
            "INSERT INTO search_index(entity_type, entity_id, title, body) VALUES (?1, ?2, ?3, ?4)",
            params![entity_type, entity_id, title, body],
        )
        .map_err(|e| {
            AppErrorDto::new("SEARCH_INDEX_FAILED", "索引写入失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }

    /// Delete an entity from the search index.
    pub fn delete_entity(
        &self,
        project_root: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        conn.execute(
            "DELETE FROM search_index WHERE entity_type = ?1 AND entity_id = ?2",
            params![entity_type, entity_id],
        )
        .map_err(|e| {
            AppErrorDto::new("SEARCH_DELETE_FAILED", "索引删除失败", true)
                .with_detail(e.to_string())
        })?;
        Ok(())
    }

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

        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;

        let fts_query = query
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT entity_type, entity_id, title, snippet(search_index, 1, '…', '…', 32, 16) as body_snippet, rank FROM search_index WHERE search_index MATCH ?1 ORDER BY rank LIMIT ?2"
            ))
            .map_err(|e| {
                AppErrorDto::new("SEARCH_QUERY_FAILED", "搜索查询失败", true)
                    .with_detail(e.to_string())
            })?;

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
            .map_err(|e| {
                AppErrorDto::new("SEARCH_QUERY_FAILED", "搜索查询失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("SEARCH_QUERY_FAILED", "搜索查询失败", true)
                    .with_detail(e.to_string())
            })?;

        Ok(results)
    }

    /// Rebuild the entire search index from project data.
    pub fn rebuild_index(&self, project_root: &str) -> Result<usize, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        conn.execute("DELETE FROM search_index", []).map_err(|e| {
            AppErrorDto::new("SEARCH_REBUILD_FAILED", "重建索引失败", true)
                .with_detail(e.to_string())
        })?;
        let mut indexed = 0usize;

        for (stmt_sql, etype, title_col, body_col) in [
            ("SELECT id, title, COALESCE(summary,'') FROM chapters WHERE is_deleted=0", "chapter", 1usize, 2usize),
            ("SELECT id, name, COALESCE(motivation,'')||' '||COALESCE(identity_text,'') FROM characters WHERE is_deleted=0", "character", 1, 2),
            ("SELECT id, title, COALESCE(description,'') FROM world_rules WHERE is_deleted=0", "world_rule", 1, 2),
            ("SELECT id, term, COALESCE(description,'') FROM glossary_terms", "glossary", 1, 2),
            ("SELECT id, title, COALESCE(goal,'')||' '||COALESCE(conflict,'') FROM plot_nodes", "plot_node", 1, 2),
        ] {
            let mut stmt = conn.prepare(stmt_sql).map_err(|e| {
                AppErrorDto::new("SEARCH_REBUILD_FAILED", "重建索引失败", true)
                    .with_detail(e.to_string())
            })?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(title_col)?,
                        row.get::<_, String>(body_col)?,
                    ))
                })
                .map_err(|e| {
                    AppErrorDto::new("SEARCH_REBUILD_FAILED", "重建索引失败", true)
                        .with_detail(e.to_string())
                })?;
            for row in rows {
                let row = row.map_err(|e| {
                    AppErrorDto::new("SEARCH_REBUILD_FAILED", "重建索引失败", true)
                        .with_detail(e.to_string())
                })?;
                conn.execute(
                    "INSERT INTO search_index(entity_type, entity_id, title, body) VALUES (?1, ?2, ?3, ?4)",
                    params![etype, row.0, row.1, row.2],
                ).map_err(|e| {
                    AppErrorDto::new("SEARCH_REBUILD_FAILED", "重建索引失败", true)
                        .with_detail(e.to_string())
                })?;
                indexed += 1;
            }
        }
        Ok(indexed)
    }
}
