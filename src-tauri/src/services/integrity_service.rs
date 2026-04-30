use std::path::Path;

use rusqlite::params;
use serde::Serialize;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::fs_utils::read_text_if_exists;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub status: String,
    pub issues: Vec<IntegrityIssue>,
    pub summary: IntegritySummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegritySummary {
    pub chapters_ok: usize,
    pub chapters_missing: usize,
    pub orphan_drafts: usize,
    pub schema_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityIssue {
    pub severity: String,
    pub category: String,
    pub message: String,
    pub detail: Option<String>,
    pub auto_fixable: bool,
}

#[derive(Default)]
pub struct IntegrityService;

impl IntegrityService {
    pub fn check_project(&self, project_root: &str) -> Result<IntegrityReport, AppErrorDto> {
        let root = Path::new(project_root);
        let mut issues: Vec<IntegrityIssue> = Vec::new();
        let mut chapters_ok = 0usize;
        let mut chapters_missing = 0usize;

        // (a) Check project.json exists and is valid JSON
        let project_json_path = root.join("project.json");
        if !project_json_path.exists() {
            issues.push(IntegrityIssue {
                severity: "error".into(),
                category: "project_file".into(),
                message: "project.json 文件不存在".into(),
                detail: Some(project_json_path.to_string_lossy().into()),
                auto_fixable: false,
            });
        } else {
            match read_text_if_exists(&project_json_path) {
                Ok(Some(content)) => {
                    if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                        issues.push(IntegrityIssue {
                            severity: "error".into(),
                            category: "project_file".into(),
                            message: "project.json 格式无效，不是合法 JSON".into(),
                            detail: None,
                            auto_fixable: false,
                        });
                    }
                }
                Ok(None) => {
                    issues.push(IntegrityIssue {
                        severity: "error".into(),
                        category: "project_file".into(),
                        message: "project.json 文件不可读".into(),
                        detail: None,
                        auto_fixable: false,
                    });
                }
                Err(e) => {
                    issues.push(IntegrityIssue {
                        severity: "error".into(),
                        category: "project_file".into(),
                        message: "project.json 读取失败".into(),
                        detail: Some(e.to_string()),
                        auto_fixable: false,
                    });
                }
            }
        }

        // (b) Check DB is openable
        let conn = match open_database(root) {
            Ok(c) => c,
            Err(e) => {
                issues.push(IntegrityIssue {
                    severity: "error".into(),
                    category: "database".into(),
                    message: "数据库无法打开".into(),
                    detail: Some(e.to_string()),
                    auto_fixable: false,
                });
                return Ok(IntegrityReport {
                    status: "corrupted".into(),
                    issues,
                    summary: IntegritySummary {
                        chapters_ok: 0,
                        chapters_missing: 0,
                        orphan_drafts: 0,
                        schema_version: "unknown".into(),
                    },
                });
            }
        };

        // Query schema version for summary
        let schema_version: String = conn
            .query_row(
                "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "unknown".into());

        // (e) Check schema_migrations up to date
        if let Ok(mut stmt) = conn.prepare("SELECT version FROM schema_migrations ORDER BY version")
        {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                let versions: Vec<String> = rows.flatten().collect();
                if versions.is_empty() {
                    issues.push(IntegrityIssue {
                        severity: "error".into(),
                        category: "schema_migration".into(),
                        message: "数据库 schema_migrations 表为空，缺少迁移记录".into(),
                        detail: None,
                        auto_fixable: false,
                    });
                } else if !versions.contains(&"0001_init".to_string()) {
                    issues.push(IntegrityIssue {
                        severity: "error".into(),
                        category: "schema_migration".into(),
                        message: "缺少初始迁移记录 0001_init".into(),
                        detail: Some(format!("当前迁移版本: {}", versions.join(", "))),
                        auto_fixable: false,
                    });
                }
            }
        }

        // (c) Check all chapters have existing content_path files
        if let Ok(mut stmt) =
            conn.prepare("SELECT id, title, content_path FROM chapters WHERE is_deleted = 0")
        {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            }) {
                for row in rows.flatten() {
                    if root.join(&row.2).exists() {
                        chapters_ok += 1;
                    } else {
                        chapters_missing += 1;
                        issues.push(IntegrityIssue {
                            severity: "error".into(),
                            category: "chapter_file".into(),
                            message: format!("章节「{}」的内容文件不存在", row.1),
                            detail: Some(root.join(&row.2).to_string_lossy().into()),
                            auto_fixable: false,
                        });
                    }
                }
            }
        }

        // (d) Check for orphan drafts
        let drafts_dir = root.join("manuscript").join("drafts");
        let mut orphan_drafts = 0usize;
        if drafts_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&drafts_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    // Drafts are named "<content_filename>.autosave.md"
                    if let Some(stem) = file_name.strip_suffix(".autosave.md") {
                        let exists = conn
                            .query_row(
                                "SELECT COUNT(*) FROM chapters WHERE content_path LIKE ?1",
                                params![format!("%{}", stem)],
                                |row| row.get::<_, i64>(0),
                            )
                            .unwrap_or(0)
                            > 0;
                        if !exists {
                            orphan_drafts += 1;
                            issues.push(IntegrityIssue {
                                severity: "warning".into(),
                                category: "orphan_draft".into(),
                                message: format!("存在孤立的自动保存草稿文件: {}", file_name),
                                detail: Some(format!("对应章节内容文件「{}」不存在", stem)),
                                auto_fixable: true,
                            });
                        }
                    }
                }
            }
        }

        // Determine overall status
        let has_errors = issues.iter().any(|i| i.severity == "error");
        let status = if issues.is_empty() {
            "healthy"
        } else if has_errors {
            "corrupted"
        } else {
            "issues_found"
        };

        Ok(IntegrityReport {
            status: status.into(),
            issues,
            summary: IntegritySummary {
                chapters_ok,
                chapters_missing,
                orphan_drafts,
                schema_version,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    use super::IntegrityService;
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
    fn fresh_project_is_healthy() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let isvc = IntegrityService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "完整性测试".into(),
                author: None,
                genre: "玄幻".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let report = isvc
            .check_project(&project.project_root)
            .expect("check_project failed");
        assert_eq!(report.status, "healthy");
        assert!(report.issues.is_empty());
        assert_eq!(report.summary.schema_version, "0006_story_state");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn missing_project_json_reported_as_corrupted() {
        let ws = create_temp_workspace();
        let isvc = IntegrityService;

        let bad_root = ws.join("bad-project");
        fs::create_dir_all(&bad_root).expect("create bad root");

        let report = isvc
            .check_project(&bad_root.to_string_lossy())
            .expect("check_project failed");
        assert_eq!(report.status, "corrupted");
        assert!(report.issues.iter().any(|i| i.category == "project_file"));

        remove_temp_workspace(&ws);
    }
}
