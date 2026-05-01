use std::path::Path;

use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

const ACTIVE_STATUS: &str = "active";
const SUPERSEDED_STATUS: &str = "superseded";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryStateRow {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub scope: String,
    pub state_kind: String,
    pub payload_json: Value,
    pub source_chapter_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryStateInput {
    pub subject_type: String,
    pub subject_id: String,
    pub scope: String,
    pub state_kind: String,
    pub payload_json: Value,
    pub source_chapter_id: Option<String>,
}

#[derive(Default)]
pub struct StoryStateService;

impl StoryStateService {
    pub fn upsert_state(
        &self,
        project_root: &str,
        input: StoryStateInput,
    ) -> Result<StoryStateRow, AppErrorDto> {
        if !input.payload_json.is_object() {
            return Err(AppErrorDto::new(
                "INVALID_STORY_STATE_PAYLOAD",
                "状态载荷必须是 JSON 对象",
                true,
            ));
        }

        let mut conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        let tx = conn.transaction().map_err(story_state_save_error)?;
        let row = Self::upsert_state_in_transaction(&tx, &project_id, input, &now)?;
        tx.commit().map_err(story_state_save_error)?;
        Ok(row)
    }

    pub(crate) fn upsert_state_in_transaction(
        tx: &rusqlite::Transaction<'_>,
        project_id: &str,
        input: StoryStateInput,
        now: &str,
    ) -> Result<StoryStateRow, AppErrorDto> {
        if !input.payload_json.is_object() {
            return Err(AppErrorDto::new(
                "INVALID_STORY_STATE_PAYLOAD",
                "状态载荷必须是 JSON 对象",
                true,
            ));
        }

        let state_id = Uuid::new_v4().to_string();
        let payload_json = serde_json::to_string(&input.payload_json).map_err(|err| {
            AppErrorDto::new("INVALID_STORY_STATE_PAYLOAD", "状态载荷序列化失败", true)
                .with_detail(err.to_string())
        })?;

        tx.execute(
            "UPDATE story_state
             SET status = ?1, updated_at = ?2
             WHERE project_id = ?3
               AND subject_type = ?4
               AND subject_id = ?5
               AND scope = ?6
               AND state_kind = ?7
               AND status = ?8",
            params![
                SUPERSEDED_STATUS,
                now,
                project_id,
                input.subject_type,
                input.subject_id,
                input.scope,
                input.state_kind,
                ACTIVE_STATUS
            ],
        )
        .map_err(story_state_save_error)?;
        tx.execute(
            "INSERT INTO story_state(
                id, project_id, subject_type, subject_id, scope, state_kind, payload_json, source_chapter_id, status, created_at, updated_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                state_id,
                project_id,
                input.subject_type,
                input.subject_id,
                input.scope,
                input.state_kind,
                payload_json,
                input.source_chapter_id,
                ACTIVE_STATUS,
                now,
                now
            ],
        )
        .map_err(story_state_save_error)?;

        Ok(StoryStateRow {
            id: state_id,
            subject_type: input.subject_type,
            subject_id: input.subject_id,
            scope: input.scope,
            state_kind: input.state_kind,
            payload_json: input.payload_json,
            source_chapter_id: input.source_chapter_id,
            status: ACTIVE_STATUS.to_string(),
            created_at: now.to_string(),
            updated_at: now.to_string(),
        })
    }

    pub fn list_latest_states(
        &self,
        project_root: &str,
        subject_type: Option<&str>,
        subject_id: Option<&str>,
    ) -> Result<Vec<StoryStateRow>, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, subject_type, subject_id, scope, state_kind, payload_json, source_chapter_id, status, created_at, updated_at
                 FROM story_state
                 WHERE project_id = ?1
                   AND status = 'active'
                   AND (?2 IS NULL OR subject_type = ?2)
                   AND (?3 IS NULL OR subject_id = ?3)
                 ORDER BY updated_at DESC, created_at DESC",
            )
            .map_err(story_state_query_error)?;

        let rows = stmt
            .query_map(params![project_id, subject_type, subject_id], |row| {
                let payload_raw: String = row.get(5)?;
                let payload_json = serde_json::from_str(&payload_raw).unwrap_or(Value::Null);
                Ok(StoryStateRow {
                    id: row.get(0)?,
                    subject_type: row.get(1)?,
                    subject_id: row.get(2)?,
                    scope: row.get(3)?,
                    state_kind: row.get(4)?,
                    payload_json,
                    source_chapter_id: row.get(6)?,
                    status: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(story_state_query_error)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(story_state_query_error)
    }

    pub fn list_chapter_states(
        &self,
        project_root: &str,
        chapter_id: &str,
    ) -> Result<Vec<StoryStateRow>, AppErrorDto> {
        self.list_latest_states(project_root, None, None)
            .map(|rows| {
                rows.into_iter()
                    .filter(|row| {
                        row.source_chapter_id.as_deref() == Some(chapter_id)
                            || (row.subject_type == "window" && row.scope == "global")
                    })
                    .collect()
            })
    }

    pub fn record_window_progress(
        &self,
        project_root: &str,
        chapter_id: &str,
        chapter_index: i64,
        word_count: i64,
    ) -> Result<StoryStateRow, AppErrorDto> {
        self.upsert_state(
            project_root,
            StoryStateInput {
                subject_type: "window".to_string(),
                subject_id: "current_window".to_string(),
                scope: "global".to_string(),
                state_kind: "progress".to_string(),
                payload_json: json!({
                    "chapterId": chapter_id,
                    "chapterIndex": chapter_index,
                    "wordCount": word_count
                }),
                source_chapter_id: Some(chapter_id.to_string()),
            },
        )
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
    open_database(Path::new(normalized_root)).map_err(|err| {
        AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(err.to_string())
    })
}

fn story_state_save_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("STORY_STATE_SAVE_FAILED", "写入状态账本失败", true)
        .with_detail(err.to_string())
}

fn story_state_query_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("STORY_STATE_QUERY_FAILED", "查询状态账本失败", true)
        .with_detail(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use rusqlite::params;
    use serde_json::json;
    use uuid::Uuid;

    use super::{StoryStateInput, StoryStateService};
    use crate::infra::database::open_database;
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
    fn story_state_upsert_and_latest_lookup_succeeds() {
        let workspace = create_temp_workspace();
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: "state-ledger-demo".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");
        let svc = StoryStateService;

        svc.upsert_state(
            &project.project_root,
            StoryStateInput {
                subject_type: "character".to_string(),
                subject_id: "char-1".to_string(),
                scope: "chapter".to_string(),
                state_kind: "emotion".to_string(),
                payload_json: json!({ "value": "anger" }),
                source_chapter_id: Some("chapter-1".to_string()),
            },
        )
        .expect("save first state");

        svc.upsert_state(
            &project.project_root,
            StoryStateInput {
                subject_type: "character".to_string(),
                subject_id: "char-1".to_string(),
                scope: "chapter".to_string(),
                state_kind: "emotion".to_string(),
                payload_json: json!({ "value": "calm" }),
                source_chapter_id: Some("chapter-1".to_string()),
            },
        )
        .expect("save second state");

        let rows = svc
            .list_latest_states(&project.project_root, Some("character"), Some("char-1"))
            .expect("states");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].state_kind, "emotion");
        assert_eq!(
            rows[0]
                .payload_json
                .get("value")
                .and_then(|value| value.as_str()),
            Some("calm")
        );

        let chapter_states = svc
            .list_chapter_states(&project.project_root, "chapter-1")
            .expect("chapter states");
        assert_eq!(chapter_states.len(), 1);

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let superseded_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM story_state
                 WHERE subject_type = 'character'
                   AND subject_id = 'char-1'
                   AND state_kind = 'emotion'
                   AND status = 'superseded'",
                params![],
                |row| row.get(0),
            )
            .expect("query superseded count");
        assert_eq!(superseded_count, 1);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn story_state_methods_accept_trimmed_project_root() {
        let workspace = create_temp_workspace();
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: "state-ledger-trimmed-root".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");
        let svc = StoryStateService;
        let wrapped_root = format!("  {}  ", project.project_root);

        svc.upsert_state(
            &wrapped_root,
            StoryStateInput {
                subject_type: "window".to_string(),
                subject_id: "current_window".to_string(),
                scope: "global".to_string(),
                state_kind: "progress".to_string(),
                payload_json: json!({ "chapterIndex": 1, "wordCount": 1200 }),
                source_chapter_id: Some("chapter-1".to_string()),
            },
        )
        .expect("save state with trimmed root");

        let rows = svc
            .list_latest_states(&wrapped_root, Some("window"), Some("current_window"))
            .expect("list state with trimmed root");
        assert_eq!(rows.len(), 1);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn story_state_methods_reject_blank_project_root() {
        let svc = StoryStateService;
        let err = svc
            .list_latest_states("   ", None, None)
            .expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
