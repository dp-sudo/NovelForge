use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
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
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, chapter_id, entity_type, entity_id, draft_item_id, action, reason, detail_json, created_at
                 FROM user_review_actions
                 WHERE project_id = ?1 AND entity_type = ?2 AND entity_id = ?3
                 ORDER BY created_at DESC
                 LIMIT 200",
            )
            .map_err(|e| {
                AppErrorDto::new("REVIEW_TRAIL_QUERY_FAILED", "查询审查轨迹失败", true)
                    .with_detail(e.to_string())
            })?;
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
                    detail: detail_raw.and_then(|raw| serde_json::from_str(&raw).ok()),
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("REVIEW_TRAIL_QUERY_FAILED", "查询审查轨迹失败", true)
                    .with_detail(e.to_string())
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
            AppErrorDto::new("REVIEW_TRAIL_QUERY_FAILED", "读取审查轨迹失败", true)
                .with_detail(e.to_string())
        })
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

    pub fn latest_draft_action(
        &self,
        project_root: &str,
        draft_item_id: &str,
    ) -> Result<Option<ReviewTrailRecord>, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        conn.query_row(
            "SELECT id, project_id, chapter_id, entity_type, entity_id, draft_item_id, action, reason, detail_json, created_at
             FROM user_review_actions
             WHERE project_id = ?1 AND draft_item_id = ?2
             ORDER BY created_at DESC
             LIMIT 1",
            params![project_id, draft_item_id.trim()],
            |row| {
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
                    detail: detail_raw.and_then(|raw| serde_json::from_str(&raw).ok()),
                    created_at: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(|e| {
            AppErrorDto::new("REVIEW_TRAIL_QUERY_FAILED", "查询审查轨迹失败", true)
                .with_detail(e.to_string())
        })
    }
}
