use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::domain::feedback::FeedbackEventStatus;
use crate::errors::AppErrorDto;
use crate::infra::app_database;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackEventRecord {
    pub id: String,
    pub project_id: String,
    pub chapter_id: Option<String>,
    pub event_type: String,
    pub rule_type: String,
    pub severity: String,
    pub condition_summary: String,
    pub suggested_action: Option<String>,
    pub context: Option<Value>,
    pub status: String,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
    pub resolution_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Default, Clone)]
pub struct FeedbackService;

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

fn feedback_query_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("FEEDBACK_QUERY_FAILED", "查询回报事件失败", true).with_detail(err.to_string())
}

fn feedback_write_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("FEEDBACK_WRITE_FAILED", "写入回报事件失败", true).with_detail(err.to_string())
}

fn feedback_state_error(code: &'static str, message: &'static str) -> AppErrorDto {
    AppErrorDto::new(code, message, true)
}

fn normalize_event_id(event_id: &str) -> Result<&str, AppErrorDto> {
    let normalized = event_id.trim();
    if normalized.is_empty() {
        return Err(feedback_state_error(
            "FEEDBACK_EVENT_ID_REQUIRED",
            "事件ID不能为空",
        ));
    }
    Ok(normalized)
}

fn normalize_resolution_note(
    value: &str,
    field_label: &'static str,
) -> Result<String, AppErrorDto> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(feedback_state_error(
            "FEEDBACK_RESOLUTION_NOTE_REQUIRED",
            field_label,
        ));
    }
    Ok(normalized.to_string())
}

fn default_feedback_rules() -> Vec<app_database::FeedbackRuleRecord> {
    vec![
        app_database::FeedbackRuleRecord {
            id: "feedback-rule-character-overflow-default".to_string(),
            rule_type: "character_overflow".to_string(),
            threshold_value: 10,
            enabled: true,
            suggestion_template: "角色数量超过阈值，建议回收边缘角色并强化核心角色关系。"
                .to_string(),
            created_at: None,
            updated_at: None,
        },
        app_database::FeedbackRuleRecord {
            id: "feedback-rule-relationship-complexity-default".to_string(),
            rule_type: "relationship_complexity".to_string(),
            threshold_value: 30,
            enabled: true,
            suggestion_template: "关系网络复杂度升高，建议拆分卷级关系主线并补充关系图约束。"
                .to_string(),
            created_at: None,
            updated_at: None,
        },
        app_database::FeedbackRuleRecord {
            id: "feedback-rule-foreshadow-unfulfilled-default".to_string(),
            rule_type: "foreshadow_unfulfilled".to_string(),
            threshold_value: 1,
            enabled: true,
            suggestion_template: "检测到伏笔兑现风险，建议新增回收章节计划或调整窗口目标。"
                .to_string(),
            created_at: None,
            updated_at: None,
        },
    ]
}

fn load_rule_map() -> HashMap<String, app_database::FeedbackRuleRecord> {
    let mut map = HashMap::new();
    if let Ok(conn) = app_database::open_or_create() {
        if let Ok(rules) = app_database::load_feedback_rules(&conn) {
            for rule in rules {
                if rule.enabled {
                    map.insert(rule.rule_type.clone(), rule);
                }
            }
        }
    }
    if map.is_empty() {
        for rule in default_feedback_rules() {
            map.insert(rule.rule_type.clone(), rule);
        }
    }
    map
}

impl FeedbackService {
    pub fn get_feedback_events(
        &self,
        project_root: &str,
    ) -> Result<Vec<FeedbackEventRecord>, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, chapter_id, event_type, rule_type, severity,
                        condition_summary, suggested_action, context_json, status,
                        resolved_at, resolved_by, resolution_note, created_at, updated_at
                 FROM feedback_events
                 WHERE project_id = ?1
                 ORDER BY created_at DESC
                 LIMIT 200",
            )
            .map_err(feedback_query_error)?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                let context_raw: Option<String> = row.get(8)?;
                Ok(FeedbackEventRecord {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    chapter_id: row.get(2)?,
                    event_type: row.get(3)?,
                    rule_type: row.get(4)?,
                    severity: row.get(5)?,
                    condition_summary: row.get(6)?,
                    suggested_action: row.get(7)?,
                    context: context_raw.and_then(|raw| serde_json::from_str(&raw).ok()),
                    status: row.get(9)?,
                    resolved_at: row.get(10)?,
                    resolved_by: row.get(11)?,
                    resolution_note: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })
            .map_err(feedback_query_error)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(feedback_query_error)
    }

    fn get_feedback_event_by_id(
        conn: &Connection,
        project_id: &str,
        event_id: &str,
    ) -> Result<FeedbackEventRecord, AppErrorDto> {
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, chapter_id, event_type, rule_type, severity,
                        condition_summary, suggested_action, context_json, status,
                        resolved_at, resolved_by, resolution_note, created_at, updated_at
                 FROM feedback_events
                 WHERE project_id = ?1 AND id = ?2
                 LIMIT 1",
            )
            .map_err(feedback_query_error)?;
        let row = stmt
            .query_row(params![project_id, event_id], |row| {
                let context_raw: Option<String> = row.get(8)?;
                Ok(FeedbackEventRecord {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    chapter_id: row.get(2)?,
                    event_type: row.get(3)?,
                    rule_type: row.get(4)?,
                    severity: row.get(5)?,
                    condition_summary: row.get(6)?,
                    suggested_action: row.get(7)?,
                    context: context_raw.and_then(|raw| serde_json::from_str(&raw).ok()),
                    status: row.get(9)?,
                    resolved_at: row.get(10)?,
                    resolved_by: row.get(11)?,
                    resolution_note: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })
            .optional()
            .map_err(feedback_query_error)?;
        row.ok_or_else(|| feedback_state_error("FEEDBACK_EVENT_NOT_FOUND", "回报事件不存在"))
    }

    fn ensure_transition_allowed(
        current: FeedbackEventStatus,
        target: FeedbackEventStatus,
    ) -> Result<(), AppErrorDto> {
        let allowed = match current {
            FeedbackEventStatus::Open => matches!(
                target,
                FeedbackEventStatus::Acknowledged
                    | FeedbackEventStatus::Resolved
                    | FeedbackEventStatus::Ignored
            ),
            FeedbackEventStatus::Acknowledged => {
                matches!(
                    target,
                    FeedbackEventStatus::Resolved | FeedbackEventStatus::Ignored
                )
            }
            FeedbackEventStatus::Resolved | FeedbackEventStatus::Ignored => false,
        };
        if allowed {
            return Ok(());
        }
        Err(feedback_state_error(
            "FEEDBACK_EVENT_INVALID_STATUS_TRANSITION",
            "当前事件状态不允许该操作",
        ))
    }

    fn build_closed_loop_note(rule_type: &str, note: &str) -> String {
        let followup = match rule_type.trim().to_ascii_lowercase().as_str() {
            "character_overflow" => {
                Some("闭环动作：已登记蓝图规划修正任务（blueprint.generate_step）。")
            }
            "relationship_complexity" => {
                Some("闭环动作：已登记关系图生成任务（relationship.review）。")
            }
            _ => None,
        };
        match followup {
            Some(extra) => format!("{note}\n{extra}"),
            None => note.to_string(),
        }
    }

    fn transition_feedback_event_status(
        &self,
        project_root: &str,
        event_id: &str,
        target_status: FeedbackEventStatus,
        resolution_note: Option<String>,
    ) -> Result<FeedbackEventRecord, AppErrorDto> {
        let normalized_event_id = normalize_event_id(event_id)?;
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let existing = Self::get_feedback_event_by_id(&conn, &project_id, normalized_event_id)?;
        let current_status = FeedbackEventStatus::from_str(&existing.status);
        Self::ensure_transition_allowed(current_status, target_status)?;

        let now = now_iso();
        let (resolved_at, resolved_by, note_value) = if matches!(
            target_status,
            FeedbackEventStatus::Resolved | FeedbackEventStatus::Ignored
        ) {
            (Some(now.clone()), Some("user".to_string()), resolution_note)
        } else {
            (None, None, None)
        };
        conn.execute(
            "UPDATE feedback_events
             SET status = ?1,
                 resolved_at = ?2,
                 resolved_by = ?3,
                 resolution_note = ?4,
                 updated_at = ?5
             WHERE id = ?6",
            params![
                target_status.as_str(),
                resolved_at,
                resolved_by,
                note_value,
                now,
                normalized_event_id
            ],
        )
        .map_err(feedback_write_error)?;

        Self::get_feedback_event_by_id(&conn, &project_id, normalized_event_id)
    }

    pub fn acknowledge_feedback_event(
        &self,
        project_root: &str,
        event_id: &str,
    ) -> Result<FeedbackEventRecord, AppErrorDto> {
        self.transition_feedback_event_status(
            project_root,
            event_id,
            FeedbackEventStatus::Acknowledged,
            None,
        )
    }

    pub fn resolve_feedback_event(
        &self,
        project_root: &str,
        event_id: &str,
        note: &str,
    ) -> Result<FeedbackEventRecord, AppErrorDto> {
        let normalized_note = normalize_resolution_note(note, "解决备注不能为空")?;
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let existing =
            Self::get_feedback_event_by_id(&conn, &project_id, normalize_event_id(event_id)?)?;
        let final_note = Self::build_closed_loop_note(&existing.rule_type, &normalized_note);
        self.transition_feedback_event_status(
            project_root,
            event_id,
            FeedbackEventStatus::Resolved,
            Some(final_note),
        )
    }

    pub fn ignore_feedback_event(
        &self,
        project_root: &str,
        event_id: &str,
        reason: &str,
    ) -> Result<FeedbackEventRecord, AppErrorDto> {
        let normalized_reason = normalize_resolution_note(reason, "忽略原因不能为空")?;
        self.transition_feedback_event_status(
            project_root,
            event_id,
            FeedbackEventStatus::Ignored,
            Some(normalized_reason),
        )
    }

    pub fn trigger_character_overflow_async(project_root: String) {
        std::thread::spawn(move || {
            let _ = FeedbackService::detect_character_overflow(&project_root);
        });
    }

    pub fn trigger_relationship_complexity_async(project_root: String) {
        std::thread::spawn(move || {
            let _ = FeedbackService::detect_relationship_complexity(&project_root);
        });
    }

    pub fn trigger_foreshadow_unfulfilled_async(project_root: String, chapter_id: String) {
        std::thread::spawn(move || {
            let _ = FeedbackService::detect_foreshadow_unfulfilled(&project_root, &chapter_id);
        });
    }

    fn detect_character_overflow(project_root: &str) -> Result<(), AppErrorDto> {
        let rules = load_rule_map();
        let Some(rule) = rules.get("character_overflow") else {
            return Ok(());
        };

        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let character_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM characters WHERE project_id = ?1 AND is_deleted = 0",
                params![project_id],
                |row| row.get(0),
            )
            .map_err(feedback_query_error)?;
        if character_count <= rule.threshold_value {
            return Ok(());
        }

        let summary = format!(
            "当前角色数量 {} 超过阈值 {}",
            character_count, rule.threshold_value
        );
        let context = serde_json::json!({
            "ruleType": "character_overflow",
            "characterCount": character_count,
            "threshold": rule.threshold_value,
        });
        Self::insert_or_refresh_open_event(
            &conn,
            &project_id,
            None,
            "character_overflow",
            "character_overflow",
            "warning",
            &summary,
            Some(rule.suggestion_template.as_str()),
            Some(&context),
        )?;
        Ok(())
    }

    fn detect_relationship_complexity(project_root: &str) -> Result<(), AppErrorDto> {
        let rules = load_rule_map();
        let Some(rule) = rules.get("relationship_complexity") else {
            return Ok(());
        };

        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let relationship_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM character_relationships WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .map_err(feedback_query_error)?;
        if relationship_count <= rule.threshold_value {
            return Ok(());
        }

        let summary = format!(
            "当前关系数量 {} 超过阈值 {}",
            relationship_count, rule.threshold_value
        );
        let context = serde_json::json!({
            "ruleType": "relationship_complexity",
            "relationshipCount": relationship_count,
            "threshold": rule.threshold_value,
        });
        Self::insert_or_refresh_open_event(
            &conn,
            &project_id,
            None,
            "relationship_complexity",
            "relationship_complexity",
            "warning",
            &summary,
            Some(rule.suggestion_template.as_str()),
            Some(&context),
        )?;
        Ok(())
    }

    fn detect_foreshadow_unfulfilled(
        project_root: &str,
        chapter_id: &str,
    ) -> Result<(), AppErrorDto> {
        let rules = load_rule_map();
        let Some(rule) = rules.get("foreshadow_unfulfilled") else {
            return Ok(());
        };

        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let current_chapter_index: i64 = conn
            .query_row(
                "SELECT chapter_index FROM chapters WHERE project_id = ?1 AND id = ?2",
                params![project_id, chapter_id.trim()],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let unfulfilled_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM narrative_obligations ob
                 LEFT JOIN chapters expected_ch ON expected_ch.id = ob.expected_payoff_chapter_id
                 WHERE ob.project_id = ?1
                   AND ob.payoff_status = 'open'
                   AND (LOWER(ob.obligation_type) LIKE '%foreshadow%' OR ob.obligation_type LIKE '%伏笔%')
                   AND (
                        ob.expected_payoff_chapter_id IS NULL
                        OR expected_ch.chapter_index IS NULL
                        OR expected_ch.chapter_index <= ?2
                   )",
                params![project_id, current_chapter_index],
                |row| row.get(0),
            )
            .map_err(feedback_query_error)?;
        if unfulfilled_count < rule.threshold_value {
            return Ok(());
        }

        let summary = format!(
            "检测到未兑现伏笔 {} 条（阈值 {}）",
            unfulfilled_count, rule.threshold_value
        );
        let context = serde_json::json!({
            "ruleType": "foreshadow_unfulfilled",
            "unfulfilledCount": unfulfilled_count,
            "threshold": rule.threshold_value,
            "chapterId": chapter_id.trim(),
            "chapterIndex": current_chapter_index,
        });
        Self::insert_or_refresh_open_event(
            &conn,
            &project_id,
            Some(chapter_id.trim()),
            "foreshadow_unfulfilled",
            "foreshadow_unfulfilled",
            "warning",
            &summary,
            Some(rule.suggestion_template.as_str()),
            Some(&context),
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_or_refresh_open_event(
        conn: &Connection,
        project_id: &str,
        chapter_id: Option<&str>,
        event_type: &str,
        rule_type: &str,
        severity: &str,
        condition_summary: &str,
        suggested_action: Option<&str>,
        context: Option<&Value>,
    ) -> Result<(), AppErrorDto> {
        let now = now_iso();
        let existing_id = conn
            .query_row(
                "SELECT id
                 FROM feedback_events
                 WHERE project_id = ?1
                   AND rule_type = ?2
                   AND status = 'open'
                   AND condition_summary = ?3
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![project_id, rule_type, condition_summary],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(feedback_query_error)?;

        let context_json = context.and_then(|value| serde_json::to_string(value).ok());
        if let Some(existing_id) = existing_id {
            conn.execute(
                "UPDATE feedback_events
                 SET chapter_id = ?1,
                     severity = ?2,
                     suggested_action = ?3,
                     context_json = ?4,
                     updated_at = ?5
                 WHERE id = ?6",
                params![
                    chapter_id.filter(|value| !value.is_empty()),
                    severity,
                    suggested_action,
                    context_json,
                    now,
                    existing_id
                ],
            )
            .map_err(feedback_write_error)?;
            return Ok(());
        }

        conn.execute(
            "INSERT INTO feedback_events(
                id, project_id, chapter_id, event_type, rule_type, severity,
                condition_summary, suggested_action, context_json, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'open', ?10, ?11)",
            params![
                Uuid::new_v4().to_string(),
                project_id,
                chapter_id.filter(|value| !value.is_empty()),
                event_type,
                rule_type,
                severity,
                condition_summary,
                suggested_action,
                context_json,
                now,
                now,
            ],
        )
        .map_err(feedback_write_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, Instant};

    use super::FeedbackService;
    use crate::services::chapter_service::{ChapterInput, ChapterService};
    use crate::services::character_service::{
        CharacterService, CreateCharacterInput, CreateRelationshipInput, RelationshipService,
    };
    use crate::services::narrative_service::{CreateObligationInput, NarrativeService};
    use crate::services::project_service::{CreateProjectInput, ProjectService};
    use uuid::Uuid;

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn create_test_project(workspace: &Path) -> String {
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: format!("反馈测试-{}", Uuid::new_v4()),
                author: Some("tester".to_string()),
                genre: "fantasy".to_string(),
                target_words: Some(120_000),
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project");
        project.project_root
    }

    fn wait_for_open_feedback_event(project_root: &str, rule_type: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(6);
        while Instant::now() < deadline {
            if let Ok(events) = FeedbackService.get_feedback_events(project_root) {
                if events
                    .iter()
                    .any(|event| event.rule_type == rule_type && event.status == "open")
                {
                    return true;
                }
            }
            thread::sleep(Duration::from_millis(80));
        }
        false
    }

    #[test]
    fn character_create_triggers_character_overflow_feedback_event() {
        let workspace = create_temp_workspace();
        let project_root = create_test_project(&workspace);
        let character_service = CharacterService;

        for idx in 0..11 {
            character_service
                .create(
                    &project_root,
                    CreateCharacterInput {
                        name: format!("角色{}", idx + 1),
                        aliases: None,
                        role_type: "supporting".to_string(),
                        age: None,
                        gender: None,
                        identity_text: None,
                        appearance: None,
                        motivation: None,
                        desire: None,
                        fear: None,
                        flaw: None,
                        arc_stage: None,
                        locked_fields: None,
                        notes: None,
                    },
                )
                .expect("create character");
        }

        assert!(
            wait_for_open_feedback_event(&project_root, "character_overflow"),
            "expected open character_overflow feedback event"
        );

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn relationship_create_triggers_relationship_complexity_feedback_event() {
        let workspace = create_temp_workspace();
        let project_root = create_test_project(&workspace);
        let character_service = CharacterService;
        let relationship_service = RelationshipService;

        let source_id = character_service
            .create(
                &project_root,
                CreateCharacterInput {
                    name: "关系源".to_string(),
                    aliases: None,
                    role_type: "lead".to_string(),
                    age: None,
                    gender: None,
                    identity_text: None,
                    appearance: None,
                    motivation: None,
                    desire: None,
                    fear: None,
                    flaw: None,
                    arc_stage: None,
                    locked_fields: None,
                    notes: None,
                },
            )
            .expect("create source character");
        let target_id = character_service
            .create(
                &project_root,
                CreateCharacterInput {
                    name: "关系目标".to_string(),
                    aliases: None,
                    role_type: "lead".to_string(),
                    age: None,
                    gender: None,
                    identity_text: None,
                    appearance: None,
                    motivation: None,
                    desire: None,
                    fear: None,
                    flaw: None,
                    arc_stage: None,
                    locked_fields: None,
                    notes: None,
                },
            )
            .expect("create target character");

        for _ in 0..31 {
            relationship_service
                .create(
                    &project_root,
                    CreateRelationshipInput {
                        source_character_id: source_id.clone(),
                        target_character_id: target_id.clone(),
                        relationship_type: "ally".to_string(),
                        description: Some("测试关系复杂度".to_string()),
                    },
                )
                .expect("create relationship");
        }

        assert!(
            wait_for_open_feedback_event(&project_root, "relationship_complexity"),
            "expected open relationship_complexity feedback event"
        );

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn save_chapter_triggers_foreshadow_unfulfilled_feedback_event() {
        let workspace = create_temp_workspace();
        let project_root = create_test_project(&workspace);
        let chapter_service = ChapterService;

        let chapter = chapter_service
            .create_chapter(
                &project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: Some("伏笔测试".to_string()),
                    target_words: Some(1200),
                    status: Some("draft".to_string()),
                },
            )
            .expect("create chapter");

        NarrativeService
            .create(
                &project_root,
                CreateObligationInput {
                    obligation_type: "伏笔".to_string(),
                    description: "测试未兑现伏笔".to_string(),
                    planted_chapter_id: Some(chapter.id.clone()),
                    expected_payoff_chapter_id: None,
                    actual_payoff_chapter_id: None,
                    payoff_status: Some("open".to_string()),
                    severity: Some("medium".to_string()),
                    related_entities: None,
                },
            )
            .expect("create narrative obligation");

        chapter_service
            .save_chapter_content(&project_root, &chapter.id, "正文触发保存")
            .expect("save chapter");

        assert!(
            wait_for_open_feedback_event(&project_root, "foreshadow_unfulfilled"),
            "expected open foreshadow_unfulfilled feedback event"
        );

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn feedback_event_lifecycle_supports_ack_resolve_ignore() {
        let workspace = create_temp_workspace();
        let project_root = create_test_project(&workspace);
        let character_service = CharacterService;

        for idx in 0..11 {
            character_service
                .create(
                    &project_root,
                    CreateCharacterInput {
                        name: format!("生命周期角色{}", idx + 1),
                        aliases: None,
                        role_type: "supporting".to_string(),
                        age: None,
                        gender: None,
                        identity_text: None,
                        appearance: None,
                        motivation: None,
                        desire: None,
                        fear: None,
                        flaw: None,
                        arc_stage: None,
                        locked_fields: None,
                        notes: None,
                    },
                )
                .expect("create character");
        }

        assert!(
            wait_for_open_feedback_event(&project_root, "character_overflow"),
            "expected open event before lifecycle transition"
        );
        let event = FeedbackService
            .get_feedback_events(&project_root)
            .expect("load feedback events")
            .into_iter()
            .find(|item| item.rule_type == "character_overflow" && item.status == "open")
            .expect("open event not found");

        let acknowledged = FeedbackService
            .acknowledge_feedback_event(&project_root, &event.id)
            .expect("acknowledge event");
        assert_eq!(acknowledged.status, "acknowledged");
        assert!(acknowledged.resolution_note.is_none());

        let resolved = FeedbackService
            .resolve_feedback_event(&project_root, &event.id, "已收敛角色结构")
            .expect("resolve event");
        assert_eq!(resolved.status, "resolved");
        assert!(resolved.resolved_at.is_some());
        assert_eq!(resolved.resolved_by.as_deref(), Some("user"));
        assert!(
            resolved
                .resolution_note
                .as_deref()
                .unwrap_or_default()
                .contains("闭环动作"),
            "resolved note should include closed-loop hint"
        );

        let err = FeedbackService
            .ignore_feedback_event(&project_root, &event.id, "重复事件")
            .expect_err("resolved event should not allow ignore transition");
        assert_eq!(err.code, "FEEDBACK_EVENT_INVALID_STATUS_TRANSITION");

        character_service
            .create(
                &project_root,
                CreateCharacterInput {
                    name: "触发忽略路径角色".to_string(),
                    aliases: None,
                    role_type: "supporting".to_string(),
                    age: None,
                    gender: None,
                    identity_text: None,
                    appearance: None,
                    motivation: None,
                    desire: None,
                    fear: None,
                    flaw: None,
                    arc_stage: None,
                    locked_fields: None,
                    notes: None,
                },
            )
            .expect("create character for ignore flow");
        assert!(
            wait_for_open_feedback_event(&project_root, "character_overflow"),
            "expected a new open event for ignore flow"
        );
        let open_for_ignore = FeedbackService
            .get_feedback_events(&project_root)
            .expect("load feedback events for ignore")
            .into_iter()
            .find(|item| item.rule_type == "character_overflow" && item.status == "open")
            .expect("expected open event for ignore");
        let ignored = FeedbackService
            .ignore_feedback_event(&project_root, &open_for_ignore.id, "暂不处理")
            .expect("ignore event");
        assert_eq!(ignored.status, "ignored");
        assert_eq!(ignored.resolution_note.as_deref(), Some("暂不处理"));

        remove_temp_workspace(&workspace);
    }
}
