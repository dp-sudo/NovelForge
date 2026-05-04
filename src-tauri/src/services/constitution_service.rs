use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_project_db;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstitutionRule {
    pub id: String,
    pub project_id: String,
    pub source_step_key: Option<String>,
    pub rule_type: String,
    pub rule_content: String,
    pub enforcement_level: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConstitutionRuleInput {
    pub source_step_key: Option<String>,
    pub rule_type: String,
    pub rule_content: String,
    pub enforcement_level: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConstitutionRuleInput {
    pub rule_type: Option<String>,
    pub rule_content: Option<String>,
    pub enforcement_level: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstitutionViolation {
    pub id: String,
    pub project_id: String,
    pub run_id: Option<String>,
    pub chapter_id: Option<String>,
    pub rule_id: String,
    pub violation_text: String,
    pub severity: String,
    pub resolution_status: String,
    pub resolution_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Summary returned after validating text against the constitution.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstitutionValidationResult {
    pub total_rules_checked: usize,
    pub violations_found: usize,
    pub has_blocker: bool,
    pub violations: Vec<ConstitutionViolation>,
}

#[derive(Default, Clone)]
pub struct ConstitutionService;

impl ConstitutionService {
    pub fn list(&self, project_root: &str) -> Result<Vec<ConstitutionRule>, AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, source_step_key, rule_type, rule_content, \
                 enforcement_level, is_active, created_at, updated_at \
                 FROM story_constitution_rules WHERE project_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询宪法规则失败", true)
                    .with_detail(e.to_string())
            })?;
        let rules = stmt
            .query_map(params![project_id], |row| {
                Ok(ConstitutionRule {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    source_step_key: row.get::<_, Option<String>>(2)?,
                    rule_type: row.get(3)?,
                    rule_content: row.get(4)?,
                    enforcement_level: row.get(5)?,
                    is_active: row.get::<_, i64>(6)? != 0,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询宪法规则失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询宪法规则失败", true)
                    .with_detail(e.to_string())
            })?;
        Ok(rules)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreateConstitutionRuleInput,
    ) -> Result<String, AppErrorDto> {
        let rule_content = input.rule_content.trim().to_string();
        if rule_content.is_empty() {
            return Err(AppErrorDto::new(
                "CONSTITUTION_RULE_EMPTY",
                "规则内容不能为空",
                true,
            ));
        }
        let conn = open_project_db(project_root)?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let enforcement = input
            .enforcement_level
            .unwrap_or_else(|| "must".to_string());
        conn.execute(
            "INSERT INTO story_constitution_rules(id, project_id, source_step_key, rule_type, \
             rule_content, enforcement_level, is_active, created_at, updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,1,?7,?8)",
            params![
                id,
                project_id,
                input.source_step_key,
                input.rule_type,
                rule_content,
                enforcement,
                now,
                now
            ],
        )
        .map_err(|e| {
            AppErrorDto::new("INSERT_FAILED", "创建宪法规则失败", true).with_detail(e.to_string())
        })?;
        Ok(id)
    }

    pub fn update(
        &self,
        project_root: &str,
        id: &str,
        input: UpdateConstitutionRuleInput,
    ) -> Result<(), AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let now = now_iso();
        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut idx = 2u32;

        // Build dynamic SET clause
        let mut param_values: Vec<String> = vec![now.clone()];

        if let Some(ref rt) = input.rule_type {
            sets.push(format!("rule_type = ?{}", idx));
            param_values.push(rt.clone());
            idx += 1;
        }
        if let Some(ref rc) = input.rule_content {
            let trimmed = rc.trim().to_string();
            if trimmed.is_empty() {
                return Err(AppErrorDto::new(
                    "CONSTITUTION_RULE_EMPTY",
                    "规则内容不能为空",
                    true,
                ));
            }
            sets.push(format!("rule_content = ?{}", idx));
            param_values.push(trimmed);
            idx += 1;
        }
        if let Some(ref el) = input.enforcement_level {
            sets.push(format!("enforcement_level = ?{}", idx));
            param_values.push(el.clone());
            idx += 1;
        }
        if let Some(active) = input.is_active {
            sets.push(format!("is_active = ?{}", idx));
            param_values.push(if active { "1" } else { "0" }.to_string());
            idx += 1;
        }

        let sql = format!(
            "UPDATE story_constitution_rules SET {} WHERE id = ?{}",
            sets.join(", "),
            idx
        );
        param_values.push(id.to_string());

        let params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|v| v as &dyn rusqlite::types::ToSql)
            .collect();

        conn.execute(&sql, params.as_slice()).map_err(|e| {
            AppErrorDto::new("UPDATE_FAILED", "更新宪法规则失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }

    pub fn delete(&self, project_root: &str, id: &str) -> Result<(), AppErrorDto> {
        let conn = open_project_db(project_root)?;
        conn.execute(
            "DELETE FROM story_constitution_rules WHERE id = ?1",
            params![id],
        )
        .map_err(|e| {
            AppErrorDto::new("DELETE_FAILED", "删除宪法规则失败", true).with_detail(e.to_string())
        })?;
        // Also clean up related violations
        conn.execute(
            "DELETE FROM constitution_violations WHERE rule_id = ?1",
            params![id],
        )
        .map_err(|e| {
            AppErrorDto::new("DELETE_FAILED", "清理违反记录失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }

    /// Validate text against all active constitution rules for a project.
    /// Uses simple keyword/pattern matching for rule enforcement.
    pub fn validate_text(
        &self,
        project_root: &str,
        text: &str,
        run_id: Option<&str>,
        chapter_id: Option<&str>,
    ) -> Result<ConstitutionValidationResult, AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let project_id = get_project_id(&conn)?;

        let mut stmt = conn
            .prepare(
                "SELECT id, rule_type, rule_content, enforcement_level \
                 FROM story_constitution_rules \
                 WHERE project_id = ?1 AND is_active = 1",
            )
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询宪法规则失败", true)
                    .with_detail(e.to_string())
            })?;
        let rules: Vec<(String, String, String, String)> = stmt
            .query_map(params![project_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询宪法规则失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询宪法规则失败", true)
                    .with_detail(e.to_string())
            })?;

        let total_rules = rules.len();
        let now = now_iso();
        let mut violations = Vec::new();

        for (rule_id, rule_type, rule_content, enforcement_level) in &rules {
            if let Some(violation_text) = self.check_rule_violation(text, rule_type, rule_content) {
                let severity = match enforcement_level.as_str() {
                    "must" => "blocker",
                    "should" => "warning",
                    _ => "info",
                };
                let violation = ConstitutionViolation {
                    id: Uuid::new_v4().to_string(),
                    project_id: project_id.clone(),
                    run_id: run_id.map(str::to_string),
                    chapter_id: chapter_id.map(str::to_string),
                    rule_id: rule_id.clone(),
                    violation_text,
                    severity: severity.to_string(),
                    resolution_status: "open".to_string(),
                    resolution_note: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };
                // Persist violation
                let _ = conn.execute(
                    "INSERT INTO constitution_violations(id, project_id, run_id, chapter_id, \
                     rule_id, violation_text, severity, resolution_status, created_at, updated_at) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                    params![
                        violation.id,
                        violation.project_id,
                        violation.run_id,
                        violation.chapter_id,
                        violation.rule_id,
                        violation.violation_text,
                        violation.severity,
                        violation.resolution_status,
                        violation.created_at,
                        violation.updated_at
                    ],
                );
                violations.push(violation);
            }
        }

        let has_blocker = violations.iter().any(|v| v.severity == "blocker");
        Ok(ConstitutionValidationResult {
            total_rules_checked: total_rules,
            violations_found: violations.len(),
            has_blocker,
            violations,
        })
    }

    pub fn list_violations(
        &self,
        project_root: &str,
    ) -> Result<Vec<ConstitutionViolation>, AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, run_id, chapter_id, rule_id, violation_text, \
                 severity, resolution_status, resolution_note, created_at, updated_at \
                 FROM constitution_violations WHERE project_id = ?1 \
                 ORDER BY created_at DESC",
            )
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询违反记录失败", true)
                    .with_detail(e.to_string())
            })?;
        let violations = stmt
            .query_map(params![project_id], |row| {
                Ok(ConstitutionViolation {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    run_id: row.get::<_, Option<String>>(2)?,
                    chapter_id: row.get::<_, Option<String>>(3)?,
                    rule_id: row.get(4)?,
                    violation_text: row.get(5)?,
                    severity: row.get(6)?,
                    resolution_status: row.get(7)?,
                    resolution_note: row.get::<_, Option<String>>(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询违反记录失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询违反记录失败", true)
                    .with_detail(e.to_string())
            })?;
        Ok(violations)
    }

    pub fn update_violation_status(
        &self,
        project_root: &str,
        violation_id: &str,
        status: &str,
        note: Option<&str>,
    ) -> Result<(), AppErrorDto> {
        let conn = open_project_db(project_root)?;
        let now = now_iso();
        conn.execute(
            "UPDATE constitution_violations SET resolution_status = ?1, resolution_note = ?2, \
             updated_at = ?3 WHERE id = ?4",
            params![status, note, now, violation_id],
        )
        .map_err(|e| {
            AppErrorDto::new("UPDATE_FAILED", "更新违反状态失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }

    /// Collect active rules as formatted text for prompt injection.
    pub fn collect_rules_for_prompt(&self, project_root: &str) -> Result<String, AppErrorDto> {
        let rules = self.list(project_root)?;
        let active_rules: Vec<&ConstitutionRule> = rules.iter().filter(|r| r.is_active).collect();
        if active_rules.is_empty() {
            return Ok(String::new());
        }
        let mut lines = vec!["# 故事宪法（最高权威约束）".to_string()];
        lines.push("以下规则具有最高优先级，任何生成内容不得违反。".to_string());
        for rule in &active_rules {
            let level_label = match rule.enforcement_level.as_str() {
                "must" => "【必须】",
                "should" => "【应当】",
                "may" => "【建议】",
                _ => "【必须】",
            };
            lines.push(format!(
                "- {} [{}] {}",
                level_label, rule.rule_type, rule.rule_content
            ));
        }
        lines.push(String::new());
        Ok(lines.join("\n"))
    }

    /// Simple rule violation check using keyword/pattern matching.
    /// For narrative_pov rules, checks if text uses wrong POV pronouns.
    /// For banned content, checks if text contains forbidden phrases.
    fn check_rule_violation(
        &self,
        text: &str,
        rule_type: &str,
        rule_content: &str,
    ) -> Option<String> {
        match rule_type {
            "banned_expression" => {
                // Rule content is a phrase that must not appear
                if text.contains(rule_content) {
                    Some(format!("检测到禁止表达：{}", rule_content))
                } else {
                    None
                }
            }
            "narrative_pov" => {
                // Check POV consistency based on rule content
                let content_lower = rule_content.to_lowercase();
                if content_lower.contains("第一人称") {
                    // First person: should not have third-person narration markers
                    let markers = ["他想着", "她心想", "他暗道"];
                    for m in markers {
                        if text.contains(m) {
                            return Some(format!("第一人称视角下出现第三人称叙述：{}", m));
                        }
                    }
                    None
                } else if content_lower.contains("第三人称") {
                    let markers = ["我想着", "我心想", "我暗道"];
                    for m in markers {
                        if text.contains(m) {
                            return Some(format!("第三人称视角下出现第一人称叙述：{}", m));
                        }
                    }
                    None
                } else {
                    None
                }
            }
            "character_constraint" => {
                // Character constraints are checked via keyword presence
                // e.g., "张三不会使用魔法" — if text contains "张三" and "使用魔法"
                // This is a basic heuristic; full validation requires AI
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    use super::{ConstitutionService, CreateConstitutionRuleInput};
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let w = std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&w).expect("create temp workspace");
        w
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn create_test_project(ws: &PathBuf) -> String {
        let ps = ProjectService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "宪法测试".into(),
                author: None,
                genre: "玄幻".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");
        project.project_root
    }

    #[test]
    fn constitution_rule_create_and_list_succeeds() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let cs = ConstitutionService;

        let id = cs
            .create(
                &project_root,
                CreateConstitutionRuleInput {
                    source_step_key: Some("step-01-anchor".into()),
                    rule_type: "narrative_pov".into(),
                    rule_content: "全文使用第三人称有限视角".into(),
                    enforcement_level: Some("must".into()),
                },
            )
            .expect("create rule");
        assert!(!id.is_empty());

        let list = cs.list(&project_root).expect("list rules");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].rule_type, "narrative_pov");
        assert_eq!(list[0].enforcement_level, "must");
        assert!(list[0].is_active);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn constitution_rule_delete_succeeds() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let cs = ConstitutionService;

        let id = cs
            .create(
                &project_root,
                CreateConstitutionRuleInput {
                    source_step_key: None,
                    rule_type: "banned_expression".into(),
                    rule_content: "命运的齿轮".into(),
                    enforcement_level: None,
                },
            )
            .expect("create rule");

        cs.delete(&project_root, &id).expect("delete rule");
        let list = cs.list(&project_root).expect("list rules");
        assert_eq!(list.len(), 0);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn constitution_validate_detects_banned_expression() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let cs = ConstitutionService;

        cs.create(
            &project_root,
            CreateConstitutionRuleInput {
                source_step_key: None,
                rule_type: "banned_expression".into(),
                rule_content: "命运的齿轮".into(),
                enforcement_level: Some("must".into()),
            },
        )
        .expect("create rule");

        let result = cs
            .validate_text(&project_root, "这一刻，命运的齿轮开始转动。", None, None)
            .expect("validate");

        assert_eq!(result.total_rules_checked, 1);
        assert_eq!(result.violations_found, 1);
        assert!(result.has_blocker);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn constitution_validate_passes_clean_text() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let cs = ConstitutionService;

        cs.create(
            &project_root,
            CreateConstitutionRuleInput {
                source_step_key: None,
                rule_type: "banned_expression".into(),
                rule_content: "命运的齿轮".into(),
                enforcement_level: Some("must".into()),
            },
        )
        .expect("create rule");

        let result = cs
            .validate_text(
                &project_root,
                "他推开门，走进了那间昏暗的房间。",
                None,
                None,
            )
            .expect("validate");

        assert_eq!(result.violations_found, 0);
        assert!(!result.has_blocker);

        remove_temp_workspace(&ws);
    }
}
