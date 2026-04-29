use std::path::Path;

use rusqlite::params;
use serde_json::Value;
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::ai_pipeline_service::{PersistedRecord, RunAiTaskPipelineInput};
use crate::services::glossary_service::CreateGlossaryTermInput;
use crate::services::narrative_service::CreateObligationInput;
use crate::services::project_service::get_project_id;
use crate::services::{
    blueprint_service::{BlueprintService, SaveBlueprintStepInput},
    character_service::{CharacterService, CreateCharacterInput},
    glossary_service::GlossaryService,
    narrative_service::NarrativeService,
    plot_service::{CreatePlotNodeInput, PlotService},
    world_service::{CreateWorldRuleInput, WorldService},
};

#[derive(Clone, Default)]
pub struct TaskHandlers;

impl TaskHandlers {
    pub fn persist_task_output(
        &self,
        canonical_task: &str,
        project_root: &str,
        input: &RunAiTaskPipelineInput,
        normalized_output: &str,
        request_id: &str,
    ) -> Result<Vec<PersistedRecord>, AppErrorDto> {
        let mut records = Vec::new();
        match canonical_task {
            "character.create" => {
                let create_input = Self::build_character_create_input(
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = CharacterService::default().create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "character".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            "world.create_rule" => {
                let create_input = Self::build_world_rule_create_input(
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = WorldService::default().create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "world_rule".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            "plot.create_node" => {
                let create_input = Self::build_plot_node_create_input(
                    project_root,
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = PlotService::default().create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "plot_node".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            "blueprint.generate_step" => {
                let step_key = input
                    .blueprint_step_key
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        AppErrorDto::new(
                            "PIPELINE_BLUEPRINT_STEP_REQUIRED",
                            "蓝图持久化缺少 stepKey",
                            true,
                        )
                    })?;
                let saved = BlueprintService::default().save_step(
                    project_root,
                    SaveBlueprintStepInput {
                        step_key: step_key.to_string(),
                        content: Self::normalize_blueprint_content(normalized_output),
                        ai_generated: Some(true),
                    },
                )?;
                records.push(PersistedRecord {
                    entity_type: "blueprint_step".to_string(),
                    entity_id: saved.id,
                    action: "updated".to_string(),
                });
            }
            "consistency.scan" => {
                let chapter_id = input
                    .chapter_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        AppErrorDto::new(
                            "PIPELINE_CHAPTER_ID_REQUIRED",
                            "一致性持久化缺少 chapterId",
                            true,
                        )
                    })?;
                let batch_size =
                    self.persist_ai_consistency_issues(project_root, chapter_id, normalized_output)?;
                records.push(PersistedRecord {
                    entity_type: "consistency_issue_batch".to_string(),
                    entity_id: format!("{}:{}", chapter_id, request_id),
                    action: format!("inserted:{}", batch_size),
                });
            }
            "glossary.create_term" => {
                let create_input = Self::build_glossary_term_create_input(
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = GlossaryService::default().create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "glossary_term".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            "narrative.create_obligation" => {
                let create_input = Self::build_narrative_obligation_create_input(
                    normalized_output,
                    input.user_instruction.as_str(),
                )?;
                let id = NarrativeService::default().create(project_root, create_input)?;
                records.push(PersistedRecord {
                    entity_type: "narrative_obligation".to_string(),
                    entity_id: id,
                    action: "created".to_string(),
                });
            }
            _ => {}
        }
        Ok(records)
    }

    fn build_character_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateCharacterInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("character"))?;
        let name = Self::pick_string(
            &root,
            &["name", "characterName", "角色名", "title"],
            Some("未命名角色"),
        );
        let role_type = Self::pick_string(
            &root,
            &["roleType", "role_type", "type", "角色类型"],
            Some("配角"),
        );
        let aliases = Self::pick_string_array(&root, &["aliases", "alias", "别名"]);
        Ok(CreateCharacterInput {
            name,
            aliases: if aliases.is_empty() { None } else { Some(aliases) },
            role_type,
            age: None,
            gender: None,
            identity_text: Self::pick_optional_string(
                &root,
                &["identityText", "identity_text", "identity", "身份"],
            ),
            appearance: Self::pick_optional_string(&root, &["appearance", "looks", "外貌"]),
            motivation: Self::pick_optional_string(&root, &["motivation", "核心动机", "drive"]),
            desire: Self::pick_optional_string(&root, &["desire", "欲望"]),
            fear: Self::pick_optional_string(&root, &["fear", "恐惧"]),
            flaw: Self::pick_optional_string(&root, &["flaw", "缺陷"]),
            arc_stage: Self::pick_optional_string(&root, &["arcStage", "arc_stage", "成长弧线"]),
            locked_fields: None,
            notes: Self::pick_optional_string(&root, &["notes", "remark", "备注"]).or_else(|| {
                (!fallback_instruction.trim().is_empty()).then(|| fallback_instruction.to_string())
            }),
        })
    }

    fn build_world_rule_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateWorldRuleInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("worldRule"))?;
        let title = Self::pick_string(&root, &["title", "name", "设定名"], Some("未命名设定"));
        let category =
            Self::pick_string(&root, &["category", "type", "类别"], Some("世界规则"));
        let description = Self::pick_string(
            &root,
            &["description", "summary", "desc", "描述"],
            Some(fallback_instruction),
        );
        let constraint_level = Self::normalize_constraint_level(
            Self::pick_optional_string(
                &root,
                &["constraintLevel", "constraint_level", "strictness", "约束等级"],
            )
            .as_deref(),
        );
        let related_entities =
            Self::pick_string_array(&root, &["relatedEntities", "related_entities", "entities"]);
        Ok(CreateWorldRuleInput {
            title,
            category,
            description,
            constraint_level,
            related_entities: if related_entities.is_empty() {
                None
            } else {
                Some(related_entities)
            },
            examples: Self::pick_optional_string(&root, &["examples", "示例"]),
            contradiction_policy: Self::pick_optional_string(
                &root,
                &["contradictionPolicy", "contradiction_policy", "冲突策略"],
            ),
        })
    }

    fn build_plot_node_create_input(
        project_root: &str,
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreatePlotNodeInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("plotNode"))?;
        let sort_order = Self::next_plot_sort_order(project_root)?;
        Ok(CreatePlotNodeInput {
            title: Self::pick_string(&root, &["title", "name", "节点标题"], Some("未命名节点")),
            node_type: Self::pick_string(
                &root,
                &["nodeType", "node_type", "type", "节点类型"],
                Some("开端"),
            ),
            sort_order,
            goal: Self::pick_optional_string(&root, &["goal", "objective", "目标"]).or_else(|| {
                (!fallback_instruction.trim().is_empty()).then(|| fallback_instruction.to_string())
            }),
            conflict: Self::pick_optional_string(&root, &["conflict", "冲突"]),
            emotional_curve: Self::pick_optional_string(
                &root,
                &["emotionalCurve", "emotional_curve", "情绪曲线"],
            ),
            status: Self::pick_optional_string(&root, &["status", "状态"]),
            related_characters: {
                let related =
                    Self::pick_string_array(&root, &["relatedCharacters", "related_characters"]);
                if related.is_empty() {
                    None
                } else {
                    Some(related)
                }
            },
        })
    }

    fn build_glossary_term_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateGlossaryTermInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("glossaryTerm"))?;
        let term = Self::pick_string(&root, &["term", "name", "词条"], Some("未命名名词"));
        let term_type = Self::pick_string(&root, &["termType", "term_type", "type", "类型"], Some("术语"));
        let aliases = Self::pick_string_array(&root, &["aliases", "alias", "别名"]);
        Ok(CreateGlossaryTermInput {
            term,
            term_type,
            aliases: if aliases.is_empty() { None } else { Some(aliases) },
            description: Self::pick_optional_string(
                &root,
                &["description", "summary", "desc", "描述"],
            )
            .or_else(|| (!fallback_instruction.trim().is_empty()).then(|| fallback_instruction.to_string())),
            locked: Some(Self::pick_bool(&root, &["locked"], false)),
            banned: Some(Self::pick_bool(&root, &["banned"], false)),
        })
    }

    fn build_narrative_obligation_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateObligationInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("obligation"))?;
        let related_entities =
            Self::pick_string_array(&root, &["relatedEntities", "related_entities", "entities"]);
        Ok(CreateObligationInput {
            obligation_type: Self::pick_string(
                &root,
                &["obligationType", "obligation_type", "type"],
                Some("foreshadowing"),
            ),
            description: Self::pick_string(
                &root,
                &["description", "summary", "desc"],
                Some(fallback_instruction),
            ),
            planted_chapter_id: Self::pick_optional_string(
                &root,
                &["plantedChapterId", "planted_chapter_id"],
            ),
            expected_payoff_chapter_id: Self::pick_optional_string(
                &root,
                &["expectedPayoffChapterId", "expected_payoff_chapter_id"],
            ),
            actual_payoff_chapter_id: Self::pick_optional_string(
                &root,
                &["actualPayoffChapterId", "actual_payoff_chapter_id"],
            ),
            payoff_status: Self::pick_optional_string(
                &root,
                &["payoffStatus", "payoff_status", "status"],
            ),
            severity: Self::pick_optional_string(&root, &["severity", "priority"]),
            related_entities: if related_entities.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&related_entities).unwrap_or_default())
            },
        })
    }

    fn persist_ai_consistency_issues(
        &self,
        project_root: &str,
        chapter_id: &str,
        normalized_output: &str,
    ) -> Result<usize, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("PIPELINE_DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let value = Self::extract_output_value(normalized_output)?;
        let issues = value
            .get("issues")
            .and_then(|item| item.as_array())
            .cloned()
            .or_else(|| value.as_array().cloned())
            .unwrap_or_default();

        let _ = conn.execute(
            "DELETE FROM consistency_issues WHERE project_id = ?1 AND chapter_id = ?2 AND status = 'open'",
            params![project_id, chapter_id],
        );

        let now = now_iso();
        let mut inserted = 0usize;
        for issue in issues {
            let issue_obj = match issue.as_object() {
                Some(obj) => obj,
                None => continue,
            };
            let explanation = Self::pick_string(issue_obj, &["explanation", "message"], Some(""));
            if explanation.trim().is_empty() {
                continue;
            }
            let issue_type =
                Self::pick_string(issue_obj, &["issueType", "issue_type", "type"], Some("prose_style"));
            let severity =
                Self::normalize_consistency_severity(Self::pick_optional_string(issue_obj, &["severity", "level"]));
            let source_text =
                Self::pick_string(issue_obj, &["sourceText", "source_text", "snippet"], Some(""));
            let suggested_fix =
                Self::pick_optional_string(issue_obj, &["suggestedFix", "suggested_fix", "fix"]);

            conn.execute(
                "INSERT INTO consistency_issues(id, project_id, issue_type, severity, chapter_id, source_text, explanation, suggested_fix, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'open', ?9, ?10)",
                params![
                    Uuid::new_v4().to_string(),
                    project_id,
                    issue_type,
                    severity,
                    chapter_id,
                    source_text,
                    explanation,
                    suggested_fix,
                    now,
                    now
                ],
            )
            .map_err(|err| {
                AppErrorDto::new("PIPELINE_PERSIST_FAILED", "写入一致性问题失败", true)
                    .with_detail(err.to_string())
            })?;
            inserted += 1;
        }

        Ok(inserted)
    }

    fn extract_output_value(normalized_output: &str) -> Result<Value, AppErrorDto> {
        if let Ok(value) = serde_json::from_str::<Value>(normalized_output) {
            return Ok(value);
        }

        let brace_start = normalized_output.find('{');
        let brace_end = normalized_output.rfind('}');
        if let (Some(start), Some(end)) = (brace_start, brace_end) {
            if end > start {
                let json_text = &normalized_output[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(json_text) {
                    return Ok(value);
                }
            }
        }

        let bracket_start = normalized_output.find('[');
        let bracket_end = normalized_output.rfind(']');
        if let (Some(start), Some(end)) = (bracket_start, bracket_end) {
            if end > start {
                let json_text = &normalized_output[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(json_text) {
                    return Ok(value);
                }
            }
        }

        Err(AppErrorDto::new(
            "PIPELINE_PERSIST_PARSE_FAILED",
            "AI 返回结果无法解析为 JSON",
            true,
        ))
    }

    fn extract_output_object(
        normalized_output: &str,
        nested_key: Option<&str>,
    ) -> Result<serde_json::Map<String, Value>, AppErrorDto> {
        let value = Self::extract_output_value(normalized_output)?;
        let root_value = if let Some(key) = nested_key {
            value.get(key).cloned().unwrap_or(value)
        } else {
            value
        };
        root_value.as_object().cloned().ok_or_else(|| {
            AppErrorDto::new(
                "PIPELINE_PERSIST_PARSE_FAILED",
                "AI 返回 JSON 结构不是对象",
                true,
            )
        })
    }

    fn pick_optional_string(
        obj: &serde_json::Map<String, Value>,
        keys: &[&str],
    ) -> Option<String> {
        for key in keys {
            if let Some(value) = obj.get(*key) {
                match value {
                    Value::String(v) => {
                        let trimmed = v.trim();
                        if !trimmed.is_empty() {
                            return Some(trimmed.to_string());
                        }
                    }
                    Value::Number(v) => return Some(v.to_string()),
                    Value::Bool(v) => return Some(v.to_string()),
                    _ => {}
                }
            }
        }
        None
    }

    fn pick_string(
        obj: &serde_json::Map<String, Value>,
        keys: &[&str],
        fallback: Option<&str>,
    ) -> String {
        Self::pick_optional_string(obj, keys)
            .or_else(|| fallback.map(str::to_string))
            .unwrap_or_default()
    }

    fn pick_bool(obj: &serde_json::Map<String, Value>, keys: &[&str], fallback: bool) -> bool {
        for key in keys {
            if let Some(value) = obj.get(*key) {
                match value {
                    Value::Bool(v) => return *v,
                    Value::Number(v) => return v.as_i64().unwrap_or(0) != 0,
                    Value::String(v) => {
                        let normalized = v.trim().to_ascii_lowercase();
                        if matches!(normalized.as_str(), "true" | "1" | "yes" | "是") {
                            return true;
                        }
                        if matches!(normalized.as_str(), "false" | "0" | "no" | "否") {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
        }
        fallback
    }

    fn pick_string_array(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Vec<String> {
        for key in keys {
            if let Some(value) = obj.get(*key) {
                match value {
                    Value::Array(values) => {
                        let list = values
                            .iter()
                            .filter_map(|item| item.as_str())
                            .map(str::trim)
                            .filter(|item| !item.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>();
                        if !list.is_empty() {
                            return list;
                        }
                    }
                    Value::String(v) => {
                        let list = v
                            .split(&[',', '，', '、'][..])
                            .map(str::trim)
                            .filter(|item| !item.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>();
                        if !list.is_empty() {
                            return list;
                        }
                    }
                    _ => {}
                }
            }
        }
        Vec::new()
    }

    fn normalize_constraint_level(raw: Option<&str>) -> String {
        let value = raw.unwrap_or("").trim().to_ascii_lowercase();
        if value.contains("weak") || value.contains("low") || value.contains("弱") {
            return "weak".to_string();
        }
        if value.contains("absolute")
            || value.contains("must")
            || value.contains("不可")
            || value.contains("绝对")
        {
            return "absolute".to_string();
        }
        if value.contains("strong") || value.contains("high") || value.contains("强") {
            return "strong".to_string();
        }
        "normal".to_string()
    }

    fn normalize_consistency_severity(raw: Option<String>) -> String {
        let value = raw
            .unwrap_or_else(|| "medium".to_string())
            .trim()
            .to_ascii_lowercase();
        if matches!(value.as_str(), "blocker" | "high" | "medium" | "low" | "info") {
            value
        } else {
            "medium".to_string()
        }
    }

    fn normalize_blueprint_content(normalized_output: &str) -> String {
        if let Ok(value) = Self::extract_output_value(normalized_output) {
            if value.is_object() {
                if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                    return pretty;
                }
            }
        }
        normalized_output.to_string()
    }

    fn next_plot_sort_order(project_root: &str) -> Result<i64, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("PIPELINE_DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM plot_nodes WHERE project_id = ?1",
            params![project_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|err| {
            AppErrorDto::new("PIPELINE_DB_QUERY_FAILED", "读取剧情节点顺序失败", true)
                .with_detail(err.to_string())
        })
    }
}
