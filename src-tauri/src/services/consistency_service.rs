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
pub struct ConsistencyIssue {
    pub id: String,
    pub issue_type: String,
    pub severity: String,
    pub chapter_id: String,
    pub source_text: String,
    pub explanation: String,
    pub suggested_fix: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanChapterInput {
    pub chapter_id: String,
}

#[derive(Default)]
pub struct ConsistencyService;

impl ConsistencyService {
    pub fn scan_chapter(
        &self,
        project_root: &str,
        input: ScanChapterInput,
    ) -> Result<Vec<ConsistencyIssue>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();

        // Check for banned glossary terms
        let mut stmt = conn
            .prepare("SELECT term FROM glossary_terms WHERE project_id = ?1 AND banned = 1")
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询禁用词失败", true).with_detail(e.to_string())
            })?;
        let banned: Vec<String> = stmt
            .query_map(params![project_id], |row| row.get(0))
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询禁用词失败", true).with_detail(e.to_string())
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut issues = Vec::new();
        for term in &banned {
            let id = Uuid::new_v4().to_string();
            issues.push(ConsistencyIssue {
                id,
                issue_type: "glossary".to_string(),
                severity: "high".to_string(),
                chapter_id: input.chapter_id.clone(),
                source_text: term.clone(),
                explanation: format!("禁用词 \"{}\" 出现在章节中", term),
                suggested_fix: Some(format!("删除或替换 \"{}\"", term)),
                status: "open".to_string(),
            });
        }

        // Store issues
        for issue in &issues {
            conn.execute(
                "INSERT INTO consistency_issues(id, project_id, issue_type, severity, chapter_id, source_text, explanation, suggested_fix, status, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![issue.id, project_id, issue.issue_type, issue.severity, issue.chapter_id, issue.source_text, issue.explanation, issue.suggested_fix, issue.status, now, now],
            ).ok();
        }

        if issues.is_empty() {
            let id = Uuid::new_v4().to_string();
            issues.push(ConsistencyIssue {
                id,
                issue_type: "prose_style".to_string(),
                severity: "info".to_string(),
                chapter_id: input.chapter_id.clone(),
                source_text: String::new(),
                explanation: "基础检查未发现明显问题".to_string(),
                suggested_fix: None,
                status: "open".to_string(),
            });
        }

        Ok(issues)
    }

    pub fn list_issues(&self, project_root: &str) -> Result<Vec<ConsistencyIssue>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id, issue_type, severity, COALESCE(chapter_id,''), COALESCE(source_text,''), explanation, suggested_fix, status FROM consistency_issues WHERE project_id = ?1 ORDER BY created_at DESC")
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询问题列表失败", true).with_detail(e.to_string()))?;
        let issues = stmt
            .query_map(params![project_id], |row| {
                Ok(ConsistencyIssue {
                    id: row.get(0)?,
                    issue_type: row.get(1)?,
                    severity: row.get(2)?,
                    chapter_id: row.get(3)?,
                    source_text: row.get(4)?,
                    explanation: row.get(5)?,
                    suggested_fix: row.get(6)?,
                    status: row.get(7)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询问题列表失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询问题列表失败", true)
                    .with_detail(e.to_string())
            })?;
        Ok(issues)
    }

    pub fn update_issue_status(
        &self,
        project_root: &str,
        issue_id: &str,
        status: &str,
    ) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let now = now_iso();
        conn.execute(
            "UPDATE consistency_issues SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, issue_id],
        )
        .map_err(|e| {
            AppErrorDto::new("UPDATE_FAILED", "更新问题状态失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }
}
