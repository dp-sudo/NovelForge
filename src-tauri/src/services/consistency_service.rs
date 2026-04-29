use std::fs;
use std::path::Path;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::path_utils::resolve_project_relative_path;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

const AI_STYLE_PATTERNS: &[&str] = &[
    "命运的齿轮",
    "这一刻，他明白了",
    "不禁让人",
    "仿佛一切都",
];

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

#[derive(Debug, Clone)]
struct ChapterScanTarget {
    id: String,
    content_path: String,
}

#[derive(Default)]
pub struct ConsistencyService;

impl ConsistencyService {
    pub fn scan_chapter(
        &self,
        project_root: &str,
        input: ScanChapterInput,
    ) -> Result<Vec<ConsistencyIssue>, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let banned_terms = self.load_banned_terms(&conn, &project_id)?;
        let chapter = self.load_chapter_target(&conn, &project_id, &input.chapter_id)?;
        let chapter_content = self.read_chapter_content(project_root_path, &chapter)?;

        let issues = self.build_chapter_issues(&chapter.id, &chapter_content, &banned_terms);
        self.replace_chapter_issues(&conn, &project_id, &chapter.id, &issues)?;
        Ok(issues)
    }

    pub fn scan_full(&self, project_root: &str) -> Result<Vec<ConsistencyIssue>, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let banned_terms = self.load_banned_terms(&conn, &project_id)?;
        let chapters = self.list_chapter_targets(&conn, &project_id)?;

        let mut issues = Vec::new();
        for chapter in chapters {
            let chapter_content = self.read_chapter_content(project_root_path, &chapter)?;
            issues.extend(self.build_chapter_issues(
                &chapter.id,
                &chapter_content,
                &banned_terms,
            ));
        }

        self.replace_project_issues(&conn, &project_id, &issues)?;
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

    fn load_banned_terms(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<Vec<String>, AppErrorDto> {
        let mut stmt = conn
            .prepare("SELECT term FROM glossary_terms WHERE project_id = ?1 AND banned = 1")
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询禁用词失败", true).with_detail(e.to_string())
            })?;
        let terms = stmt
            .query_map(params![project_id], |row| row.get::<_, String>(0))
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询禁用词失败", true).with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询禁用词失败", true).with_detail(e.to_string())
            })?;
        Ok(terms)
    }

    fn load_chapter_target(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
        chapter_id: &str,
    ) -> Result<ChapterScanTarget, AppErrorDto> {
        conn.query_row(
            "SELECT id, content_path FROM chapters WHERE id = ?1 AND project_id = ?2 AND is_deleted = 0",
            params![chapter_id, project_id],
            |row| {
                Ok(ChapterScanTarget {
                    id: row.get(0)?,
                    content_path: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|e| {
            AppErrorDto::new("QUERY_FAILED", "查询章节失败", true).with_detail(e.to_string())
        })?
        .ok_or_else(|| AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true))
    }

    fn list_chapter_targets(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<Vec<ChapterScanTarget>, AppErrorDto> {
        let mut stmt = conn
            .prepare(
                "SELECT id, content_path FROM chapters WHERE project_id = ?1 AND is_deleted = 0 ORDER BY chapter_index",
            )
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询章节失败", true).with_detail(e.to_string())
            })?;

        let chapters = stmt
            .query_map(params![project_id], |row| {
                Ok(ChapterScanTarget {
                    id: row.get(0)?,
                    content_path: row.get(1)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询章节失败", true).with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询章节失败", true).with_detail(e.to_string())
            })?;
        Ok(chapters)
    }

    fn read_chapter_content(
        &self,
        project_root_path: &Path,
        chapter: &ChapterScanTarget,
    ) -> Result<String, AppErrorDto> {
        let chapter_file = resolve_project_relative_path(project_root_path, &chapter.content_path)
            .map_err(|detail| {
            AppErrorDto::new("CHAPTER_PATH_INVALID", "章节路径无效", true).with_detail(detail)
        })?;
        fs::read_to_string(&chapter_file).map_err(|e| {
            AppErrorDto::new("CHAPTER_READ_FAILED", "读取章节正文失败", true)
                .with_detail(e.to_string())
        })
    }

    fn build_chapter_issues(
        &self,
        chapter_id: &str,
        chapter_content: &str,
        banned_terms: &[String],
    ) -> Vec<ConsistencyIssue> {
        let mut issues = Vec::new();
        for term in banned_terms {
            if chapter_content.contains(term) {
                issues.push(ConsistencyIssue {
                    id: Uuid::new_v4().to_string(),
                    issue_type: "glossary".to_string(),
                    severity: "high".to_string(),
                    chapter_id: chapter_id.to_string(),
                    source_text: term.clone(),
                    explanation: format!("检测到禁用词：{}", term),
                    suggested_fix: Some(format!("删除或替换 \"{}\"", term)),
                    status: "open".to_string(),
                });
            }
        }

        for pattern in AI_STYLE_PATTERNS {
            if chapter_content.contains(pattern) {
                issues.push(ConsistencyIssue {
                    id: Uuid::new_v4().to_string(),
                    issue_type: "prose_style".to_string(),
                    severity: "medium".to_string(),
                    chapter_id: chapter_id.to_string(),
                    source_text: pattern.to_string(),
                    explanation: format!("检测到可能模板化表达：{}", pattern),
                    suggested_fix: Some("改为具体动作、场景或对话".to_string()),
                    status: "open".to_string(),
                });
            }
        }

        if issues.is_empty() {
            issues.push(ConsistencyIssue {
                id: Uuid::new_v4().to_string(),
                issue_type: "prose_style".to_string(),
                severity: "info".to_string(),
                chapter_id: chapter_id.to_string(),
                source_text: String::new(),
                explanation: "基础检查未发现明显问题".to_string(),
                suggested_fix: None,
                status: "open".to_string(),
            });
        }

        issues
    }

    fn replace_chapter_issues(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
        chapter_id: &str,
        issues: &[ConsistencyIssue],
    ) -> Result<(), AppErrorDto> {
        conn.execute(
            "DELETE FROM consistency_issues WHERE project_id = ?1 AND chapter_id = ?2",
            params![project_id, chapter_id],
        )
        .map_err(|e| {
            AppErrorDto::new("DELETE_FAILED", "清理旧问题失败", true).with_detail(e.to_string())
        })?;
        self.insert_issues(conn, project_id, issues)
    }

    fn replace_project_issues(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
        issues: &[ConsistencyIssue],
    ) -> Result<(), AppErrorDto> {
        conn.execute(
            "DELETE FROM consistency_issues WHERE project_id = ?1",
            params![project_id],
        )
        .map_err(|e| {
            AppErrorDto::new("DELETE_FAILED", "清理旧问题失败", true).with_detail(e.to_string())
        })?;
        self.insert_issues(conn, project_id, issues)
    }

    fn insert_issues(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
        issues: &[ConsistencyIssue],
    ) -> Result<(), AppErrorDto> {
        let now = now_iso();
        for issue in issues {
            conn.execute(
                "INSERT INTO consistency_issues(id, project_id, issue_type, severity, chapter_id, source_text, explanation, suggested_fix, status, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![issue.id, project_id, issue.issue_type, issue.severity, issue.chapter_id, issue.source_text, issue.explanation, issue.suggested_fix, issue.status, now, now],
            )
            .map_err(|e| {
                AppErrorDto::new("INSERT_FAILED", "写入一致性问题失败", true)
                    .with_detail(e.to_string())
            })?;
        }
        Ok(())
    }
}
