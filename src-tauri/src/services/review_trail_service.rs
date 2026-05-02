use std::path::Path;

use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewTrailRecord {
    pub id: String,
    pub project_id: String,
    pub chapter_id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub draft_item_id: Option<String>,
    pub action: String,
    pub reason: Option<String>,
    pub detail: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct RecordReviewActionInput {
    pub chapter_id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub draft_item_id: Option<String>,
    pub action: String,
    pub reason: Option<String>,
    pub detail: Option<Value>,
}

#[derive(Clone, Default)]
pub struct ReviewTrailService;

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

fn review_invalid_error(message: &'static str) -> AppErrorDto {
    AppErrorDto::new("REVIEW_TRAIL_INVALID", message, true)
}

fn review_query_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("REVIEW_TRAIL_QUERY_FAILED", "查询审查轨迹失败", true)
        .with_detail(err.to_string())
}

fn review_read_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("REVIEW_TRAIL_QUERY_FAILED", "读取审查轨迹失败", true)
        .with_detail(err.to_string())
}

fn parse_optional_json(raw: Option<String>) -> Option<Value> {
    raw.and_then(|value| serde_json::from_str(&value).ok())
}

fn parse_optional_json_or_string(raw: Option<String>) -> Option<Value> {
    raw.map(|value| match serde_json::from_str::<Value>(&value) {
        Ok(parsed) => parsed,
        Err(_) => Value::String(value),
    })
}

fn normalize_optional_text(raw: Option<String>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn pipeline_action(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "succeeded" => "ai_generated".to_string(),
        "failed" => "ai_failed".to_string(),
        "running" => "ai_running".to_string(),
        "cancelled" | "canceled" => "ai_canceled".to_string(),
        other => format!("ai_{other}"),
    }
}

fn feedback_action(status: &str) -> String {
    let normalized = status.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return "feedback_open".to_string();
    }
    format!("feedback_{normalized}")
}

fn load_user_review_records(
    conn: &Connection,
    project_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<ReviewTrailRecord>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, chapter_id, entity_type, entity_id, draft_item_id, action, reason, detail_json, created_at
             FROM user_review_actions
             WHERE project_id = ?1 AND entity_type = ?2 AND entity_id = ?3
             ORDER BY created_at DESC
             LIMIT 200",
        )
        .map_err(review_query_error)?;
    let rows = stmt
        .query_map(params![project_id, entity_type, entity_id], |row| {
            let detail_raw: Option<String> = row.get(8)?;
            Ok(ReviewTrailRecord {
                id: row.get(0)?,
                project_id: row.get(1)?,
                chapter_id: row.get(2)?,
                entity_type: row.get(3)?,
                entity_id: row.get(4)?,
                draft_item_id: row.get(5)?,
                action: row.get(6)?,
                reason: row.get(7)?,
                detail: parse_optional_json(detail_raw),
                created_at: row.get(9)?,
            })
        })
        .map_err(review_query_error)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(review_read_error)
}

fn load_chapter_pipeline_records(
    conn: &Connection,
    project_id: &str,
    chapter_id: &str,
) -> Result<Vec<ReviewTrailRecord>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_type, ui_action, status, phase, error_code, error_message,
                    duration_ms, created_at, completed_at, meta_json, post_task_results
             FROM ai_pipeline_runs
             WHERE project_id = ?1 AND chapter_id = ?2
             ORDER BY created_at DESC
             LIMIT 120",
        )
        .map_err(review_query_error)?;
    let rows = stmt
        .query_map(params![project_id, chapter_id], |row| {
            let run_id: String = row.get(0)?;
            let task_type: String = row.get(1)?;
            let ui_action: Option<String> = row.get(2)?;
            let status: String = row.get(3)?;
            let phase: Option<String> = row.get(4)?;
            let error_code: Option<String> = row.get(5)?;
            let error_message: Option<String> = row.get(6)?;
            let duration_ms: i64 = row.get(7)?;
            let created_at: String = row.get(8)?;
            let completed_at: Option<String> = row.get(9)?;
            let meta_json: Option<String> = row.get(10)?;
            let post_task_results: Option<String> = row.get(11)?;
            let timeline_time = normalize_optional_text(completed_at.clone()).unwrap_or(created_at);
            let reason = if status.trim().eq_ignore_ascii_case("failed") {
                normalize_optional_text(error_message.clone())
            } else {
                None
            };
            let detail = serde_json::json!({
                "source": "ai_pipeline_runs",
                "runId": run_id,
                "taskType": task_type,
                "uiAction": normalize_optional_text(ui_action),
                "status": status,
                "phase": normalize_optional_text(phase),
                "errorCode": normalize_optional_text(error_code),
                "errorMessage": normalize_optional_text(error_message),
                "durationMs": duration_ms,
                "createdAt": timeline_time.clone(),
                "completedAt": completed_at,
                "meta": parse_optional_json_or_string(meta_json),
                "postTaskResults": parse_optional_json_or_string(post_task_results),
            });
            Ok(ReviewTrailRecord {
                id: format!("ai_pipeline_run:{run_id}"),
                project_id: project_id.to_string(),
                chapter_id: Some(chapter_id.to_string()),
                entity_type: "ai_pipeline_run".to_string(),
                entity_id: detail
                    .get("runId")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                draft_item_id: None,
                action: pipeline_action(
                    detail
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("running"),
                ),
                reason,
                detail: Some(detail),
                created_at: timeline_time,
            })
        })
        .map_err(review_query_error)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(review_read_error)
}

fn load_chapter_feedback_records(
    conn: &Connection,
    project_id: &str,
    chapter_id: &str,
) -> Result<Vec<ReviewTrailRecord>, AppErrorDto> {
    let mut stmt = conn
        .prepare(
            "SELECT id, event_type, rule_type, severity, condition_summary, suggested_action,
                    context_json, status, resolved_at, resolved_by, resolution_note, created_at, updated_at
             FROM feedback_events
             WHERE project_id = ?1 AND chapter_id = ?2
             ORDER BY created_at DESC
             LIMIT 120",
        )
        .map_err(review_query_error)?;
    let rows = stmt
        .query_map(params![project_id, chapter_id], |row| {
            let event_id: String = row.get(0)?;
            let event_type: String = row.get(1)?;
            let rule_type: String = row.get(2)?;
            let severity: String = row.get(3)?;
            let condition_summary: String = row.get(4)?;
            let suggested_action: Option<String> = row.get(5)?;
            let context_json: Option<String> = row.get(6)?;
            let status: String = row.get(7)?;
            let resolved_at: Option<String> = row.get(8)?;
            let resolved_by: Option<String> = row.get(9)?;
            let resolution_note: Option<String> = row.get(10)?;
            let created_at: String = row.get(11)?;
            let updated_at: String = row.get(12)?;
            let timeline_time = normalize_optional_text(resolved_at.clone())
                .or_else(|| normalize_optional_text(Some(updated_at.clone())))
                .unwrap_or(created_at.clone());
            let reason = normalize_optional_text(resolution_note.clone())
                .or_else(|| normalize_optional_text(Some(condition_summary.clone())));
            let detail = serde_json::json!({
                "source": "feedback_events",
                "eventId": event_id,
                "eventType": event_type,
                "ruleType": rule_type,
                "severity": severity,
                "conditionSummary": condition_summary,
                "suggestedAction": normalize_optional_text(suggested_action),
                "context": parse_optional_json_or_string(context_json),
                "status": status,
                "resolvedAt": resolved_at,
                "resolvedBy": normalize_optional_text(resolved_by),
                "resolutionNote": normalize_optional_text(resolution_note),
                "createdAt": created_at,
                "updatedAt": updated_at,
            });
            Ok(ReviewTrailRecord {
                id: format!("feedback_event:{event_id}"),
                project_id: project_id.to_string(),
                chapter_id: Some(chapter_id.to_string()),
                entity_type: "feedback_event".to_string(),
                entity_id: detail
                    .get("eventId")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                draft_item_id: None,
                action: feedback_action(
                    detail
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("open"),
                ),
                reason,
                detail: Some(detail),
                created_at: timeline_time,
            })
        })
        .map_err(review_query_error)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(review_read_error)
}

impl ReviewTrailService {
    pub fn get_review_trail(
        &self,
        project_root: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<ReviewTrailRecord>, AppErrorDto> {
        let entity_type = entity_type.trim().to_ascii_lowercase();
        let entity_id = entity_id.trim();
        if entity_type.is_empty() || entity_id.is_empty() {
            return Err(review_invalid_error("entityType/entityId 不能为空"));
        }

        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut records = load_user_review_records(&conn, &project_id, &entity_type, entity_id)?;
        if entity_type == "chapter" {
            records.extend(load_chapter_pipeline_records(
                &conn,
                &project_id,
                entity_id,
            )?);
            records.extend(load_chapter_feedback_records(
                &conn,
                &project_id,
                entity_id,
            )?);
        }
        records.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        if records.len() > 200 {
            records.truncate(200);
        }
        Ok(records)
    }

    pub fn record_action(
        &self,
        project_root: &str,
        input: RecordReviewActionInput,
    ) -> Result<(), AppErrorDto> {
        let mut conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let tx = conn.transaction().map_err(|e| {
            AppErrorDto::new("REVIEW_TRAIL_WRITE_FAILED", "记录审查轨迹失败", true)
                .with_detail(e.to_string())
        })?;
        Self::record_action_in_transaction(&tx, &project_id, input, &now_iso())?;
        tx.commit().map_err(|e| {
            AppErrorDto::new("REVIEW_TRAIL_WRITE_FAILED", "记录审查轨迹失败", true)
                .with_detail(e.to_string())
        })?;
        Ok(())
    }

    pub fn record_action_in_transaction(
        tx: &Transaction<'_>,
        project_id: &str,
        input: RecordReviewActionInput,
        now: &str,
    ) -> Result<(), AppErrorDto> {
        let entity_type = input.entity_type.trim().to_ascii_lowercase();
        let entity_id = input.entity_id.trim().to_string();
        let action = input.action.trim().to_ascii_lowercase();
        if entity_type.is_empty() || entity_id.is_empty() || action.is_empty() {
            return Err(review_invalid_error("entityType/entityId/action 不能为空"));
        }
        let reason = input
            .reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let detail_json = input
            .detail
            .as_ref()
            .and_then(|value| serde_json::to_string(value).ok());
        tx.execute(
            "INSERT INTO user_review_actions(
                id, project_id, chapter_id, entity_type, entity_id, draft_item_id, action, reason, detail_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                Uuid::new_v4().to_string(),
                project_id,
                input.chapter_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                entity_type,
                entity_id,
                input
                    .draft_item_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                action,
                reason,
                detail_json,
                now,
            ],
        )
        .map_err(|e| {
            AppErrorDto::new("REVIEW_TRAIL_WRITE_FAILED", "记录审查轨迹失败", true)
                .with_detail(e.to_string())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use rusqlite::params;
    use uuid::Uuid;

    use super::ReviewTrailService;
    use crate::infra::database::open_database;
    use crate::services::chapter_service::{ChapterInput, ChapterService};
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn chapter_review_trail_aggregates_user_pipeline_and_feedback_records() {
        let workspace = create_temp_workspace();
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: "review-trail-aggregate".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: workspace.to_string_lossy().into(),
            })
            .expect("create project");
        let chapter = ChapterService
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".into(),
                    summary: None,
                    target_words: None,
                    status: None,
                },
            )
            .expect("create chapter");

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let project_id: String = conn
            .query_row("SELECT id FROM projects LIMIT 1", [], |row| row.get(0))
            .expect("query project id");

        conn.execute(
            "INSERT INTO user_review_actions(
                id, project_id, chapter_id, entity_type, entity_id, draft_item_id, action, reason, detail_json, created_at
             ) VALUES (?1, ?2, ?3, 'chapter', ?4, NULL, 'approved', '人工确认', '{}', ?5)",
            params![
                Uuid::new_v4().to_string(),
                &project_id,
                &chapter.id,
                &chapter.id,
                "2026-05-02T10:00:00Z"
            ],
        )
        .expect("insert review action");

        let run_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO ai_pipeline_runs(
                id, project_id, chapter_id, task_type, ui_action, status, phase, error_code, error_message, duration_ms, created_at, completed_at, meta_json, post_task_results
             ) VALUES (?1, ?2, ?3, 'chapter.draft', 'manual_generate', 'succeeded', 'done', NULL, NULL, 820, ?4, ?5, ?6, ?7)",
            params![
                &run_id,
                &project_id,
                &chapter.id,
                "2026-05-02T09:59:00Z",
                "2026-05-02T09:59:10Z",
                r#"{"route":"drafter-pool"}"#,
                r#"{"extract_state":{"status":"ok"}}"#
            ],
        )
        .expect("insert pipeline run");

        let feedback_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO feedback_events(
                id, project_id, chapter_id, event_type, rule_type, severity, condition_summary, suggested_action, context_json, status, created_at, updated_at, resolved_at, resolved_by, resolution_note
             ) VALUES (?1, ?2, ?3, 'post_task', 'foreshadow_unfulfilled', 'warning', '伏笔尚未兑现', '补写承接段', ?4, 'resolved', ?5, ?6, ?6, 'user', '已补写并确认')",
            params![
                &feedback_id,
                &project_id,
                &chapter.id,
                r#"{"chapterId":"test"}"#,
                "2026-05-02T09:58:00Z",
                "2026-05-02T10:01:00Z"
            ],
        )
        .expect("insert feedback event");
        drop(conn);

        let trail = ReviewTrailService
            .get_review_trail(&project.project_root, "chapter", &chapter.id)
            .expect("load review trail");

        assert!(trail
            .iter()
            .any(|row| row.entity_type == "chapter" && row.action == "approved"));
        assert!(trail.iter().any(|row| {
            row.entity_type == "ai_pipeline_run"
                && row.entity_id == run_id
                && row.action == "ai_generated"
        }));
        assert!(trail.iter().any(|row| {
            row.entity_type == "feedback_event"
                && row.entity_id == feedback_id
                && row.action == "feedback_resolved"
        }));
        assert_eq!(
            trail.first().map(|row| row.entity_type.as_str()),
            Some("feedback_event")
        );

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn non_chapter_review_trail_stays_user_review_only() {
        let workspace = create_temp_workspace();
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: "review-trail-user-only".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: workspace.to_string_lossy().into(),
            })
            .expect("create project");

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let project_id: String = conn
            .query_row("SELECT id FROM projects LIMIT 1", [], |row| row.get(0))
            .expect("query project id");
        let character_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO user_review_actions(
                id, project_id, chapter_id, entity_type, entity_id, draft_item_id, action, reason, detail_json, created_at
             ) VALUES (?1, ?2, NULL, 'character', ?3, NULL, 'edited', '手工修订角色设定', '{}', ?4)",
            params![
                Uuid::new_v4().to_string(),
                &project_id,
                &character_id,
                "2026-05-02T11:00:00Z"
            ],
        )
        .expect("insert review action");
        drop(conn);

        let trail = ReviewTrailService
            .get_review_trail(&project.project_root, "character", &character_id)
            .expect("load review trail");
        assert_eq!(trail.len(), 1);
        assert_eq!(trail[0].entity_type, "character");
        assert_eq!(trail[0].action, "edited");

        remove_temp_workspace(&workspace);
    }
}
