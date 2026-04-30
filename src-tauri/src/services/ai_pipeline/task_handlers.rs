use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::{params, Connection};
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
    character_service::{
        CharacterService, CreateCharacterInput, CreateRelationshipInput, RelationshipService,
    },
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
                        content: Self::normalize_blueprint_step_content(
                            step_key,
                            normalized_output,
                        ),
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
                let batch_size = self.persist_ai_consistency_issues(
                    project_root,
                    chapter_id,
                    normalized_output,
                )?;
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
            "chapter.plan" => {
                let updated_chapter_id =
                    Self::persist_chapter_plan_output(project_root, input, normalized_output)?;
                records.push(PersistedRecord {
                    entity_type: "chapter".to_string(),
                    entity_id: updated_chapter_id,
                    action: "updated".to_string(),
                });
            }
            "timeline.review" => {
                let updated_count =
                    Self::persist_timeline_review_output(project_root, normalized_output)?;
                records.push(PersistedRecord {
                    entity_type: "timeline_entry_batch".to_string(),
                    entity_id: request_id.to_string(),
                    action: format!("updated:{}", updated_count),
                });
            }
            "relationship.review" => {
                let inserted_count =
                    Self::persist_relationship_review_output(project_root, normalized_output)?;
                records.push(PersistedRecord {
                    entity_type: "character_relationship_batch".to_string(),
                    entity_id: request_id.to_string(),
                    action: format!("inserted:{}", inserted_count),
                });
            }
            _ => {}
        }
        self.record_entity_provenance_for_records(
            canonical_task,
            project_root,
            input,
            request_id,
            &records,
        )?;
        Ok(records)
    }

    fn record_entity_provenance_for_records(
        &self,
        canonical_task: &str,
        project_root: &str,
        input: &RunAiTaskPipelineInput,
        request_id: &str,
        records: &[PersistedRecord],
    ) -> Result<(), AppErrorDto> {
        if records.is_empty() {
            return Ok(());
        }
        let source_kind = match Self::resolve_provenance_source_kind(canonical_task, input) {
            Some(kind) => kind,
            None => return Ok(()),
        };
        let source_ref = Self::resolve_provenance_source_ref(canonical_task, input);

        let conn = open_database(Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        for record in records {
            if !Self::should_record_provenance(canonical_task, record) {
                continue;
            }
            Self::insert_entity_provenance(
                &conn,
                &project_id,
                &record.entity_type,
                &record.entity_id,
                source_kind,
                source_ref.as_deref(),
                request_id,
            )?;
        }
        Ok(())
    }

    fn should_record_provenance(canonical_task: &str, record: &PersistedRecord) -> bool {
        match canonical_task {
            "blueprint.generate_step" => record.entity_type == "blueprint_step",
            "character.create"
            | "world.create_rule"
            | "plot.create_node"
            | "glossary.create_term"
            | "narrative.create_obligation"
            | "chapter.plan" => true,
            _ => false,
        }
    }

    fn resolve_provenance_source_kind(
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> Option<&'static str> {
        match canonical_task {
            "blueprint.generate_step" => Some("blueprint_draft"),
            "character.create"
            | "world.create_rule"
            | "plot.create_node"
            | "glossary.create_term"
            | "narrative.create_obligation"
            | "chapter.plan" => {
                if Self::is_promotion_action(input.ui_action.as_deref()) {
                    let tier = input
                        .automation_tier
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or("");
                    if tier.eq_ignore_ascii_case("auto") || tier.eq_ignore_ascii_case("supervised")
                    {
                        Some("auto_promotion")
                    } else {
                        Some("manual_promotion")
                    }
                } else {
                    Some("ai_generation")
                }
            }
            _ => None,
        }
    }

    fn resolve_provenance_source_ref(
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> Option<String> {
        if canonical_task == "blueprint.generate_step" {
            return input
                .blueprint_step_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
        }
        input
            .ui_action
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn is_promotion_action(ui_action: Option<&str>) -> bool {
        ui_action
            .map(str::trim)
            .map(|value| value.to_ascii_lowercase().contains("promote"))
            .unwrap_or(false)
    }

    fn insert_entity_provenance(
        conn: &Connection,
        project_id: &str,
        entity_type: &str,
        entity_id: &str,
        source_kind: &str,
        source_ref: Option<&str>,
        request_id: &str,
    ) -> Result<(), AppErrorDto> {
        conn.execute(
            "INSERT INTO entity_provenance (id, project_id, entity_type, entity_id, source_kind, source_ref, request_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Uuid::new_v4().to_string(),
                project_id,
                entity_type,
                entity_id,
                source_kind,
                source_ref,
                request_id,
                now_iso(),
            ],
        )
        .map_err(|err| {
            AppErrorDto::new("PIPELINE_PROVENANCE_WRITE_FAILED", "写入来源轨迹失败", true)
                .with_detail(err.to_string())
        })?;
        Ok(())
    }

    fn build_character_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateCharacterInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("character"))?;
        let basic_info = Self::pick_object(&root, &["basicInfo", "basic_info", "basic"]);
        let appearance_obj = Self::pick_object(&root, &["appearance", "looks_detail"]);
        let personality_obj = Self::pick_object(
            &root,
            &["personality", "personalityProfile", "personality_profile"],
        );
        let arc_obj = Self::pick_object(&root, &["arc", "growthArc", "growth_arc"]);
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
            aliases: if aliases.is_empty() {
                None
            } else {
                Some(aliases)
            },
            role_type,
            age: Self::pick_optional_text(&root, &["age", "年龄"]).or_else(|| {
                basic_info.and_then(|obj| Self::pick_optional_text(obj, &["age", "年龄"]))
            }),
            gender: Self::pick_optional_text(&root, &["gender", "性别"]).or_else(|| {
                basic_info.and_then(|obj| Self::pick_optional_text(obj, &["gender", "性别"]))
            }),
            identity_text: Self::pick_optional_string(
                &root,
                &["identityText", "identity_text", "identity", "身份"],
            )
            .or_else(|| {
                basic_info.and_then(|obj| {
                    Self::compose_identity_text(
                        Self::pick_optional_text(obj, &["occupation", "职业"]),
                        Self::pick_optional_text(obj, &["status", "身份", "状态"]),
                    )
                })
            }),
            appearance: Self::pick_optional_string(&root, &["appearance", "looks", "外貌"])
                .or_else(|| Self::compose_appearance_text(appearance_obj)),
            motivation: Self::pick_optional_string(&root, &["motivation", "核心动机", "drive"])
                .or_else(|| {
                    personality_obj.and_then(|obj| {
                        Self::pick_optional_text(obj, &["desires", "desire", "愿望", "诉求"])
                    })
                }),
            desire: Self::pick_optional_string(&root, &["desire", "欲望"]).or_else(|| {
                personality_obj.and_then(|obj| {
                    Self::pick_optional_text(obj, &["desires", "desire", "愿望", "诉求"])
                })
            }),
            fear: Self::pick_optional_string(&root, &["fear", "恐惧"]).or_else(|| {
                personality_obj
                    .and_then(|obj| Self::pick_optional_text(obj, &["fears", "fear", "恐惧"]))
            }),
            flaw: Self::pick_optional_string(&root, &["flaw", "缺陷"]).or_else(|| {
                personality_obj
                    .and_then(|obj| Self::pick_optional_text(obj, &["flaws", "flaw", "缺陷"]))
            }),
            arc_stage: Self::pick_optional_string(&root, &["arcStage", "arc_stage", "成长弧线"])
                .or_else(|| {
                    arc_obj.and_then(|obj| {
                        Self::pick_optional_text(
                            obj,
                            &["potentialGrowth", "potential_growth", "成长", "成长方向"],
                        )
                    })
                }),
            locked_fields: None,
            notes: Self::pick_optional_string(&root, &["notes", "remark", "备注"])
                .or_else(|| Self::pick_optional_text(&root, &["background", "背景", "经历"]))
                .or_else(|| {
                    personality_obj.and_then(|obj| {
                        Self::pick_optional_text(
                            obj,
                            &["contradictions", "contradiction", "矛盾", "内在矛盾"],
                        )
                    })
                })
                .or_else(|| {
                    (!fallback_instruction.trim().is_empty())
                        .then(|| fallback_instruction.to_string())
                }),
        })
    }

    fn build_world_rule_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateWorldRuleInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("worldRule"))?;
        let title = Self::pick_string(&root, &["title", "name", "设定名"], Some("未命名设定"));
        let category = Self::pick_string(&root, &["category", "type", "类别"], Some("世界规则"));
        let mut description_parts = Vec::new();
        if let Some(value) =
            Self::pick_optional_text(&root, &["description", "summary", "desc", "描述"])
        {
            description_parts.push(value);
        }
        if let Some(value) = Self::pick_optional_text(
            &root,
            &["boundary", "scopeBoundary", "hardBoundary", "边界"],
        ) {
            description_parts.push(format!("边界：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(&root, &["cost", "代价"]) {
            description_parts.push(format!("代价：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(
            &root,
            &["failureConditions", "failure_conditions", "失效条件"],
        ) {
            description_parts.push(format!("失效条件：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(&root, &["pitfalls", "riskHints", "风险提示"])
        {
            description_parts.push(format!("风险提示：{value}"));
        }
        let description = if description_parts.is_empty() {
            fallback_instruction.to_string()
        } else {
            description_parts.join("\n")
        };
        let constraint_level = Self::normalize_constraint_level(
            Self::pick_optional_string(
                &root,
                &[
                    "constraintLevel",
                    "constraint_level",
                    "strictness",
                    "约束等级",
                    "constraintLevelHint",
                ],
            )
            .as_deref(),
        );
        let related_entities = Self::pick_string_array(
            &root,
            &[
                "relatedEntities",
                "related_entities",
                "entities",
                "conflictMechanisms",
                "conflict_mechanisms",
                "interactions",
            ],
        );
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
            examples: Self::pick_optional_text(
                &root,
                &["examples", "示例", "narrativeUsage", "narrative_usage"],
            ),
            contradiction_policy: Self::pick_optional_string(
                &root,
                &[
                    "contradictionPolicy",
                    "contradiction_policy",
                    "冲突策略",
                    "interactionRule",
                    "interaction_rule",
                    "priorityRule",
                    "priority_rule",
                ],
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
            title: Self::pick_string(
                &root,
                &[
                    "title",
                    "name",
                    "nodeTitle",
                    "eventTitle",
                    "keyEvent",
                    "节点标题",
                    "核心事件",
                ],
                Some("未命名节点"),
            ),
            node_type: Self::pick_string(
                &root,
                &[
                    "nodeType",
                    "node_type",
                    "type",
                    "layer",
                    "conflictType",
                    "节点类型",
                    "冲突类型",
                ],
                Some("开端"),
            ),
            sort_order,
            goal: Self::pick_optional_text(
                &root,
                &[
                    "goal",
                    "objective",
                    "目标",
                    "keyEvent",
                    "关键事件",
                    "triggerCondition",
                    "payoffWindow",
                ],
            )
            .or_else(|| {
                (!fallback_instruction.trim().is_empty()).then(|| fallback_instruction.to_string())
            }),
            conflict: Self::pick_optional_text(
                &root,
                &[
                    "conflict",
                    "冲突",
                    "conflictType",
                    "networkRisk",
                    "riskHints",
                    "风险提示",
                ],
            ),
            emotional_curve: Self::pick_optional_text(
                &root,
                &[
                    "emotionalCurve",
                    "emotional_curve",
                    "emotionalTone",
                    "tone",
                    "情绪曲线",
                    "情绪基调",
                ],
            ),
            status: Self::pick_optional_string(&root, &["status", "状态"]),
            related_characters: {
                let related = Self::pick_string_array(
                    &root,
                    &[
                        "relatedCharacters",
                        "related_characters",
                        "characters",
                        "linkedCharacterArc",
                        "linked_character_arc",
                    ],
                );
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
        let term = Self::pick_string(
            &root,
            &[
                "term",
                "name",
                "canonicalName",
                "canonical_name",
                "词条",
                "规范名",
            ],
            Some("未命名名词"),
        );
        let term_type = Self::pick_string(
            &root,
            &["termType", "term_type", "type", "category", "类型", "分类"],
            Some("术语"),
        );
        let aliases = Self::pick_string_array(&root, &["aliases", "alias", "aliasMap", "别名"]);
        let mut description_parts = Vec::new();
        if let Some(value) = Self::pick_optional_text(
            &root,
            &[
                "description",
                "summary",
                "desc",
                "描述",
                "oneLineDefinition",
                "one_line_definition",
            ],
        ) {
            description_parts.push(value);
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["scopeBoundary", "scope_boundary", "适用边界"])
        {
            description_parts.push(format!("适用边界：{value}"));
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["firstUseContext", "first_use_context", "首次语境"])
        {
            description_parts.push(format!("首次语境：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(
            &root,
            &[
                "forbiddenMisuse",
                "forbidden_misuse",
                "常见误用",
                "禁用用法",
            ],
        ) {
            description_parts.push(format!("禁用用法：{value}"));
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["usageGuidelines", "usage_guidelines", "用法建议"])
        {
            description_parts.push(format!("用法建议：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(
            &root,
            &[
                "conflictCheck",
                "conflict_check",
                "resolution",
                "冲突检测",
                "整合建议",
            ],
        ) {
            description_parts.push(format!("冲突与整合：{value}"));
        }
        Ok(CreateGlossaryTermInput {
            term,
            term_type,
            aliases: if aliases.is_empty() {
                None
            } else {
                Some(aliases)
            },
            description: if description_parts.is_empty() {
                (!fallback_instruction.trim().is_empty()).then(|| fallback_instruction.to_string())
            } else {
                Some(description_parts.join("\n"))
            },
            locked: Some(Self::pick_bool(&root, &["locked"], false)),
            banned: Some(Self::pick_bool(&root, &["banned"], false)),
        })
    }

    fn build_narrative_obligation_create_input(
        normalized_output: &str,
        fallback_instruction: &str,
    ) -> Result<CreateObligationInput, AppErrorDto> {
        let root = Self::extract_output_object(normalized_output, Some("obligation"))?;
        let related_entities = Self::pick_string_array(
            &root,
            &[
                "relatedEntities",
                "related_entities",
                "entities",
                "linkedPlotNode",
                "linked_plot_node",
                "linkedCharacterArc",
                "linked_character_arc",
                "relations",
            ],
        );
        let mut description_parts = Vec::new();
        if let Some(value) =
            Self::pick_optional_text(&root, &["description", "summary", "desc", "notes", "说明"])
        {
            description_parts.push(value);
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["seedSignal", "seed_signal", "伏笔埋点"])
        {
            description_parts.push(format!("埋点信号：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(
            &root,
            &["triggerCondition", "trigger_condition", "触发条件"],
        ) {
            description_parts.push(format!("触发条件：{value}"));
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["payoffWindow", "payoff_window", "回收窗口"])
        {
            description_parts.push(format!("回收窗口：{value}"));
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["fallbackPlan", "fallback_plan", "补救方案"])
        {
            description_parts.push(format!("延期补救：{value}"));
        }
        Ok(CreateObligationInput {
            obligation_type: Self::pick_string(
                &root,
                &[
                    "obligationType",
                    "obligation_type",
                    "type",
                    "obligationKind",
                    "伏笔类型",
                ],
                Some("foreshadowing"),
            ),
            description: if description_parts.is_empty() {
                fallback_instruction.to_string()
            } else {
                description_parts.join("\n")
            },
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
            severity: Self::pick_optional_string(
                &root,
                &["severity", "priority", "riskLevel", "risk_level"],
            ),
            related_entities: if related_entities.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&related_entities).unwrap_or_default())
            },
        })
    }

    fn persist_chapter_plan_output(
        project_root: &str,
        input: &RunAiTaskPipelineInput,
        normalized_output: &str,
    ) -> Result<String, AppErrorDto> {
        let chapter_id = input
            .chapter_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppErrorDto::new(
                    "PIPELINE_CHAPTER_ID_REQUIRED",
                    "章节规划持久化缺少 chapterId",
                    true,
                )
            })?;

        let root = Self::extract_output_object(normalized_output, Some("chapterPlan"))?;
        let mut summary_parts = Vec::new();

        if let Some(value) = Self::pick_optional_text(
            &root,
            &["chapterFunction", "chapter_function", "章节功能定位"],
        ) {
            summary_parts.push(format!("章节功能：{value}"));
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["successCriteria", "success_criteria", "完成标准"])
        {
            summary_parts.push(format!("完成标准：{value}"));
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["emotionalArc", "emotional_arc", "节奏曲线"])
        {
            summary_parts.push(format!("节奏曲线：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(&root, &["scenes", "场景拆分"]) {
            summary_parts.push(format!("场景拆分：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(
            &root,
            &["foreshadowingPlan", "foreshadowing_plan", "伏笔处理"],
        ) {
            summary_parts.push(format!("伏笔处理：{value}"));
        }
        if let Some(value) =
            Self::pick_optional_text(&root, &["cliffhanger", "章节钩子", "结尾钩子"])
        {
            summary_parts.push(format!("章节钩子：{value}"));
        }
        if let Some(value) = Self::pick_optional_text(&root, &["notes", "备注"]) {
            summary_parts.push(format!("备注：{value}"));
        }

        let summary = if summary_parts.is_empty() {
            input.user_instruction.trim().to_string()
        } else {
            summary_parts.join("\n")
        };
        let target_words = Self::pick_optional_i64(
            &root,
            &["totalWords", "total_words", "targetWords", "target_words"],
        );

        let status = Self::pick_optional_string(&root, &["status", "chapterStatus", "章节状态"])
            .unwrap_or_else(|| "planned".to_string());

        let conn = open_database(Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("PIPELINE_DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let updated_at = now_iso();

        let changed = conn
            .execute(
                "
                UPDATE chapters
                SET summary = ?1,
                    status = ?2,
                    target_words = COALESCE(?3, target_words),
                    updated_at = ?4
                WHERE id = ?5 AND project_id = ?6 AND is_deleted = 0
                ",
                params![
                    summary,
                    status,
                    target_words,
                    updated_at,
                    chapter_id,
                    project_id
                ],
            )
            .map_err(|err| {
                AppErrorDto::new("PIPELINE_PERSIST_FAILED", "写入章节规划失败", true)
                    .with_detail(err.to_string())
            })?;

        if changed == 0 {
            return Err(AppErrorDto::new(
                "CHAPTER_NOT_FOUND",
                "章节不存在或不可写入",
                true,
            ));
        }

        Ok(chapter_id.to_string())
    }

    fn persist_timeline_review_output(
        project_root: &str,
        normalized_output: &str,
    ) -> Result<usize, AppErrorDto> {
        let value = Self::extract_output_value(normalized_output)?;
        let root = value.as_object().cloned().ok_or_else(|| {
            AppErrorDto::new(
                "PIPELINE_PERSIST_PARSE_FAILED",
                "时间线审阅结果不是 JSON 对象",
                true,
            )
        })?;

        let entries = root
            .get("timelineEntries")
            .or_else(|| root.get("timeline_entries"))
            .or_else(|| root.get("eventTimeline"))
            .or_else(|| root.get("event_timeline"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if entries.is_empty() {
            return Ok(0);
        }

        let conn = open_database(Path::new(project_root)).map_err(|err| {
            AppErrorDto::new("PIPELINE_DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let updated_at = now_iso();
        let mut updated_count = 0usize;

        for item in entries {
            let obj = match item.as_object() {
                Some(obj) => obj,
                None => continue,
            };
            let chapter_id = Self::pick_optional_string(obj, &["chapterId", "chapter_id", "id"]);
            let chapter_index =
                Self::pick_optional_i64(obj, &["chapterIndex", "chapter_index", "index"]);
            let title = Self::pick_optional_string(
                obj,
                &["title", "chapterTitle", "chapter_title", "章节标题"],
            );
            let summary =
                Self::pick_optional_text(obj, &["summary", "event", "description", "事件摘要"]);
            let status =
                Self::pick_optional_string(obj, &["status", "chapterStatus", "chapter_status"]);
            let target_words =
                Self::pick_optional_i64(obj, &["targetWords", "target_words", "words"]);

            let changed = if let Some(chapter_id) = chapter_id.as_deref() {
                conn.execute(
                    "
                    UPDATE chapters
                    SET title = COALESCE(?1, title),
                        summary = COALESCE(?2, summary),
                        status = COALESCE(?3, status),
                        target_words = COALESCE(?4, target_words),
                        updated_at = ?5
                    WHERE id = ?6 AND project_id = ?7 AND is_deleted = 0
                    ",
                    params![
                        title,
                        summary,
                        status,
                        target_words,
                        updated_at,
                        chapter_id,
                        project_id
                    ],
                )
                .map_err(|err| {
                    AppErrorDto::new("PIPELINE_PERSIST_FAILED", "写入时间线失败", true)
                        .with_detail(err.to_string())
                })?
            } else if let Some(chapter_index) = chapter_index {
                conn.execute(
                    "
                    UPDATE chapters
                    SET title = COALESCE(?1, title),
                        summary = COALESCE(?2, summary),
                        status = COALESCE(?3, status),
                        target_words = COALESCE(?4, target_words),
                        updated_at = ?5
                    WHERE chapter_index = ?6 AND project_id = ?7 AND is_deleted = 0
                    ",
                    params![
                        title,
                        summary,
                        status,
                        target_words,
                        updated_at,
                        chapter_index,
                        project_id
                    ],
                )
                .map_err(|err| {
                    AppErrorDto::new("PIPELINE_PERSIST_FAILED", "写入时间线失败", true)
                        .with_detail(err.to_string())
                })?
            } else {
                0
            };

            if changed > 0 {
                updated_count += changed as usize;
            }
        }

        Ok(updated_count)
    }

    fn persist_relationship_review_output(
        project_root: &str,
        normalized_output: &str,
    ) -> Result<usize, AppErrorDto> {
        let value = Self::extract_output_value(normalized_output)?;
        let root = value.as_object().cloned().ok_or_else(|| {
            AppErrorDto::new(
                "PIPELINE_PERSIST_PARSE_FAILED",
                "关系审阅结果不是 JSON 对象",
                true,
            )
        })?;

        let mut edges = root
            .get("edges")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if edges.is_empty() {
            edges = root
                .get("relationGraph")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("edges"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
        if edges.is_empty() {
            return Ok(0);
        }

        let characters = CharacterService::default().list(project_root)?;
        let mut name_index = HashMap::<String, String>::new();
        for item in &characters {
            let key = Self::normalize_lookup_label(&item.name);
            if !key.is_empty() {
                name_index.insert(key, item.id.clone());
            }
            if let Ok(aliases) = serde_json::from_str::<Vec<String>>(&item.aliases) {
                for alias in aliases {
                    let alias_key = Self::normalize_lookup_label(&alias);
                    if !alias_key.is_empty() {
                        name_index.insert(alias_key, item.id.clone());
                    }
                }
            }
        }

        let existing_links = RelationshipService::default().list(project_root, None)?;
        let mut dedupe = HashSet::<String>::new();
        for item in existing_links {
            let key = format!(
                "{}|{}|{}",
                item.source_character_id,
                item.target_character_id,
                Self::normalize_lookup_label(&item.relationship_type)
            );
            dedupe.insert(key);
        }

        let mut inserted_count = 0usize;
        for edge in edges {
            let obj = match edge.as_object() {
                Some(obj) => obj,
                None => continue,
            };

            let source_name = Self::pick_optional_string(
                obj,
                &[
                    "sourceName",
                    "source_name",
                    "source",
                    "from",
                    "sourceCharacter",
                ],
            )
            .unwrap_or_default();
            let target_name = Self::pick_optional_string(
                obj,
                &[
                    "targetName",
                    "target_name",
                    "target",
                    "to",
                    "targetCharacter",
                ],
            )
            .unwrap_or_default();
            let relation_type = Self::pick_optional_string(
                obj,
                &[
                    "relationshipType",
                    "relationship_type",
                    "type",
                    "relationType",
                ],
            )
            .unwrap_or_else(|| "未命名关系".to_string());
            if source_name.trim().is_empty() || target_name.trim().is_empty() {
                continue;
            }

            let source_id = name_index
                .get(&Self::normalize_lookup_label(&source_name))
                .cloned();
            let target_id = name_index
                .get(&Self::normalize_lookup_label(&target_name))
                .cloned();
            let (source_id, target_id) = match (source_id, target_id) {
                (Some(source_id), Some(target_id)) => (source_id, target_id),
                _ => continue,
            };
            if source_id == target_id {
                continue;
            }

            let dedupe_key = format!(
                "{}|{}|{}",
                source_id,
                target_id,
                Self::normalize_lookup_label(&relation_type)
            );
            if dedupe.contains(&dedupe_key) {
                continue;
            }

            let description = Self::pick_optional_text(
                obj,
                &[
                    "description",
                    "reason",
                    "evidence",
                    "latestTriggerEvent",
                    "latest_trigger_event",
                ],
            );
            RelationshipService::default().create(
                project_root,
                CreateRelationshipInput {
                    source_character_id: source_id.clone(),
                    target_character_id: target_id.clone(),
                    relationship_type: relation_type,
                    description,
                },
            )?;
            dedupe.insert(dedupe_key);
            inserted_count += 1;
        }

        Ok(inserted_count)
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
            let issue_type = Self::pick_string(
                issue_obj,
                &["issueType", "issue_type", "type"],
                Some("prose_style"),
            );
            let severity = Self::normalize_consistency_severity(Self::pick_optional_string(
                issue_obj,
                &["severity", "level"],
            ));
            let source_text = Self::pick_string(
                issue_obj,
                &["sourceText", "source_text", "snippet"],
                Some(""),
            );
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
        let trimmed = normalized_output.trim();
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Ok(value);
        }

        if let Some(value) = Self::extract_value_from_code_fences(trimmed) {
            return Ok(value);
        }

        if let Some(value) = Self::extract_first_balanced_json_value(trimmed) {
            return Ok(value);
        }

        let brace_start = trimmed.find('{');
        let brace_end = trimmed.rfind('}');
        if let (Some(start), Some(end)) = (brace_start, brace_end) {
            if end > start {
                let json_text = &trimmed[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(json_text) {
                    return Ok(value);
                }
            }
        }

        let bracket_start = trimmed.find('[');
        let bracket_end = trimmed.rfind(']');
        if let (Some(start), Some(end)) = (bracket_start, bracket_end) {
            if end > start {
                let json_text = &trimmed[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(json_text) {
                    return Ok(value);
                }
            }
        }

        Err(AppErrorDto::new(
            "PIPELINE_PERSIST_PARSE_FAILED",
            "AI 返回结果无法解析为 JSON",
            true,
        )
        .with_detail(format!(
            "normalized_output_preview={}",
            Self::preview_output_for_error(normalized_output, 320)
        )))
    }

    fn extract_value_from_code_fences(raw: &str) -> Option<Value> {
        for (idx, segment) in raw.split("```").enumerate() {
            if idx % 2 == 0 {
                continue;
            }
            let body = if let Some(line_break) = segment.find('\n') {
                &segment[line_break + 1..]
            } else {
                segment
            };
            let candidate = body.trim();
            if candidate.is_empty() {
                continue;
            }

            if let Ok(value) = serde_json::from_str::<Value>(candidate) {
                return Some(value);
            }
            if let Some(value) = Self::extract_first_balanced_json_value(candidate) {
                return Some(value);
            }
        }
        None
    }

    fn extract_first_balanced_json_value(raw: &str) -> Option<Value> {
        let bytes = raw.as_bytes();
        for start in 0..bytes.len() {
            if !matches!(bytes[start], b'{' | b'[') {
                continue;
            }
            let end = match Self::find_balanced_json_end(raw, start) {
                Some(end) => end,
                None => continue,
            };
            let candidate = &raw[start..=end];
            if let Ok(value) = serde_json::from_str::<Value>(candidate) {
                return Some(value);
            }
        }
        None
    }

    fn find_balanced_json_end(raw: &str, start: usize) -> Option<usize> {
        let bytes = raw.as_bytes();
        let mut stack: Vec<u8> = Vec::new();
        let mut in_string = false;
        let mut escaped = false;

        for idx in start..bytes.len() {
            let byte = bytes[idx];
            if in_string {
                if escaped {
                    escaped = false;
                    continue;
                }
                match byte {
                    b'\\' => escaped = true,
                    b'"' => in_string = false,
                    _ => {}
                }
                continue;
            }

            match byte {
                b'"' => in_string = true,
                b'{' => stack.push(b'}'),
                b'[' => stack.push(b']'),
                b'}' | b']' => {
                    let expected = stack.pop()?;
                    if byte != expected {
                        return None;
                    }
                    if stack.is_empty() {
                        return Some(idx);
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn extract_output_object(
        normalized_output: &str,
        nested_key: Option<&str>,
    ) -> Result<serde_json::Map<String, Value>, AppErrorDto> {
        let value = Self::extract_output_value(normalized_output)?;
        let root_obj = value.as_object().cloned().ok_or_else(|| {
            AppErrorDto::new(
                "PIPELINE_PERSIST_PARSE_FAILED",
                "AI 返回 JSON 结构不是对象",
                true,
            )
            .with_detail(format!(
                "normalized_output_preview={}",
                Self::preview_output_for_error(normalized_output, 320)
            ))
        })?;

        if let Some(key) = nested_key {
            if let Some(nested) = Self::pick_value(&root_obj, key).and_then(Value::as_object) {
                return Ok(nested.clone());
            }
        }

        for fallback_key in ["data", "content", "fields", "payload", "result"] {
            if let Some(nested) =
                Self::pick_value(&root_obj, fallback_key).and_then(Value::as_object)
            {
                return Ok(nested.clone());
            }
        }

        Ok(root_obj)
    }

    fn preview_output_for_error(raw: &str, max_chars: usize) -> String {
        let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.is_empty() {
            return "<empty>".to_string();
        }
        let chars = normalized.chars().collect::<Vec<_>>();
        if chars.len() <= max_chars {
            return normalized;
        }
        let preview = chars[..max_chars].iter().collect::<String>();
        format!("{preview}...(truncated,total_chars={})", chars.len())
    }

    fn pick_optional_string(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(value) = Self::pick_value(obj, key) {
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

    fn pick_optional_text(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(value) = Self::pick_value(obj, key) {
                if let Some(text) = Self::json_value_to_text(value) {
                    return Some(text);
                }
            }
        }
        None
    }

    fn pick_object<'a>(
        obj: &'a serde_json::Map<String, Value>,
        keys: &[&str],
    ) -> Option<&'a serde_json::Map<String, Value>> {
        for key in keys {
            if let Some(value) = Self::pick_value(obj, key).and_then(Value::as_object) {
                return Some(value);
            }
        }
        None
    }

    fn normalize_key(key: &str) -> String {
        key.to_ascii_lowercase()
            .chars()
            .filter(|ch| !matches!(ch, '_' | '-' | ' '))
            .collect()
    }

    fn normalize_lookup_label(value: &str) -> String {
        value
            .trim()
            .to_ascii_lowercase()
            .chars()
            .filter(|ch| !matches!(ch, ' ' | '\n' | '\r' | '\t'))
            .collect()
    }

    fn pick_value<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a Value> {
        if let Some(value) = obj.get(key) {
            return Some(value);
        }
        let normalized_key = Self::normalize_key(key);
        obj.iter()
            .find(|(candidate, _)| Self::normalize_key(candidate) == normalized_key)
            .map(|(_, value)| value)
    }

    fn pick_optional_i64(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<i64> {
        for key in keys {
            if let Some(value) = Self::pick_value(obj, key) {
                match value {
                    Value::Number(v) => {
                        if let Some(parsed) = v.as_i64() {
                            return Some(parsed);
                        }
                    }
                    Value::String(v) => {
                        if let Ok(parsed) = v.trim().parse::<i64>() {
                            return Some(parsed);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    fn json_value_to_text(value: &Value) -> Option<String> {
        match value {
            Value::String(v) => {
                let trimmed = v.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            Value::Number(v) => Some(v.to_string()),
            Value::Bool(v) => Some(v.to_string()),
            Value::Array(values) => {
                let list = values
                    .iter()
                    .filter_map(Self::json_value_to_text)
                    .collect::<Vec<_>>();
                if list.is_empty() {
                    None
                } else {
                    Some(list.join("；"))
                }
            }
            _ => None,
        }
    }

    fn compose_identity_text(occupation: Option<String>, status: Option<String>) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(value) = occupation {
            if !value.trim().is_empty() {
                parts.push(value);
            }
        }
        if let Some(value) = status {
            if !value.trim().is_empty() {
                parts.push(value);
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("；"))
        }
    }

    fn compose_appearance_text(
        appearance_obj: Option<&serde_json::Map<String, Value>>,
    ) -> Option<String> {
        let obj = appearance_obj?;
        let overview = Self::pick_optional_text(obj, &["overview", "概述"]);
        let distinctive = Self::pick_optional_text(
            obj,
            &[
                "distinctiveFeatures",
                "distinctive_features",
                "特征",
                "细节",
            ],
        );
        let style = Self::pick_optional_text(obj, &["style", "风格", "穿着"]);
        let mut parts = Vec::new();
        if let Some(value) = overview {
            parts.push(value);
        }
        if let Some(value) = distinctive {
            parts.push(format!("特征：{value}"));
        }
        if let Some(value) = style {
            parts.push(format!("风格：{value}"));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
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
            if let Some(value) = Self::pick_value(obj, key) {
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
            if let Some(value) = Self::pick_value(obj, key) {
                match value {
                    Value::Array(values) => {
                        let list = values
                            .iter()
                            .filter_map(|item| match item {
                                Value::String(v) => {
                                    let trimmed = v.trim();
                                    (!trimmed.is_empty()).then(|| trimmed.to_string())
                                }
                                Value::Number(v) => Some(v.to_string()),
                                Value::Bool(v) => Some(v.to_string()),
                                Value::Object(obj) => Self::pick_optional_text(
                                    obj,
                                    &[
                                        "name",
                                        "label",
                                        "title",
                                        "id",
                                        "value",
                                        "target",
                                        "node",
                                        "character",
                                    ],
                                ),
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        if !list.is_empty() {
                            return list;
                        }
                    }
                    Value::Object(obj) => {
                        if let Some(text) = Self::pick_optional_text(
                            obj,
                            &[
                                "name",
                                "label",
                                "title",
                                "id",
                                "value",
                                "target",
                                "node",
                                "character",
                            ],
                        ) {
                            return vec![text];
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
        if matches!(
            value.as_str(),
            "blocker" | "high" | "medium" | "low" | "info"
        ) {
            value
        } else {
            "medium".to_string()
        }
    }

    fn blueprint_step_fields(step_key: &str) -> &'static [&'static str] {
        match step_key {
            "step-01-anchor" => &[
                "coreInspiration",
                "coreProposition",
                "coreEmotion",
                "targetReader",
                "sellingPoint",
                "readerExpectation",
            ],
            "step-02-genre" => &[
                "mainGenre",
                "subGenre",
                "narrativePov",
                "styleKeywords",
                "rhythmType",
                "bannedStyle",
            ],
            "step-03-premise" => &[
                "oneLineLogline",
                "threeParagraphSummary",
                "beginning",
                "middle",
                "climax",
                "ending",
            ],
            "step-04-characters" => &[
                "protagonist",
                "antagonist",
                "supportingCharacters",
                "relationshipSummary",
                "growthArc",
            ],
            "step-05-world" => &[
                "worldBackground",
                "rules",
                "locations",
                "organizations",
                "inviolableRules",
            ],
            "step-06-glossary" => &[
                "personNames",
                "placeNames",
                "organizationNames",
                "terms",
                "aliases",
                "bannedTerms",
            ],
            "step-07-plot" => &[
                "mainGoal",
                "stages",
                "keyConflicts",
                "twists",
                "climax",
                "ending",
            ],
            "step-08-chapters" => &[
                "volumeStructure",
                "chapterList",
                "chapterGoals",
                "characters",
                "plotNodes",
            ],
            _ => &[],
        }
    }

    fn blueprint_field_aliases(field: &str) -> &'static [&'static str] {
        match field {
            "coreInspiration" => &["inspiration", "core_inspiration", "核心灵感", "灵感来源"],
            "coreProposition" => &["proposition", "core_proposition", "核心命题", "主题命题"],
            "coreEmotion" => &["emotion", "core_emotion", "核心情绪", "情绪基调"],
            "targetReader" => &["reader", "target_reader", "目标读者"],
            "sellingPoint" => &["selling_point", "商业卖点", "卖点"],
            "readerExpectation" => &["reader_expectation", "读者期待", "预期"],
            "mainGenre" => &["genre", "main_genre", "主类型", "主题材"],
            "subGenre" => &["sub_genre", "子类型", "子题材"],
            "narrativePov" => &["pov", "narrative_pov", "叙事视角", "视角"],
            "styleKeywords" => &["style", "style_keywords", "文风关键词", "风格关键词"],
            "rhythmType" => &["rhythm", "rhythm_type", "节奏类型", "节奏"],
            "bannedStyle" => &["banned_style", "禁用风格", "避免风格"],
            "oneLineLogline" => &["logline", "one_line_logline", "一句话梗概"],
            "threeParagraphSummary" => &["summary", "three_paragraph_summary", "三段式梗概"],
            "beginning" => &["start", "opening", "开端"],
            "middle" => &["mid", "中段"],
            "climax" => &["高潮"],
            "ending" => &["结局", "ending_direction"],
            "protagonist" => &["mainCharacter", "main_character", "主角"],
            "antagonist" => &["villain", "反派"],
            "supportingCharacters" => &["supporting_characters", "配角", "关键配角"],
            "relationshipSummary" => &["relationship_summary", "角色关系", "角色关系摘要"],
            "growthArc" => &["arc", "growth_arc", "成长弧线", "角色成长"],
            "worldBackground" => &["background", "world_background", "世界背景"],
            "rules" => &["rule", "world_rules", "规则", "规则体系"],
            "locations" => &["places", "地点"],
            "organizations" => &["factions", "组织", "势力"],
            "inviolableRules" => &["hard_rules", "inviolable_rules", "不可违反规则", "铁律"],
            "personNames" => &["person_names", "characters", "人名"],
            "placeNames" => &["place_names", "地名"],
            "organizationNames" => &["organization_names", "组织名"],
            "terms" => &["术语", "glossary_terms"],
            "aliases" => &["别名", "alias_map"],
            "bannedTerms" => &["banned_terms", "禁用名词", "禁词"],
            "mainGoal" => &["goal", "main_goal", "主线目标"],
            "stages" => &["stage_nodes", "阶段节点"],
            "keyConflicts" => &["conflicts", "key_conflicts", "关键冲突"],
            "twists" => &["reversals", "反转"],
            "volumeStructure" => &["volume_structure", "卷结构"],
            "chapterList" => &["chapters", "chapter_list", "章节列表"],
            "chapterGoals" => &["chapter_goals", "章节目标"],
            "characters" => &["cast", "出场人物"],
            "plotNodes" => &["plot_nodes", "关联主线节点"],
            _ => &[],
        }
    }

    fn normalize_blueprint_step_content(step_key: &str, normalized_output: &str) -> String {
        let fields = Self::blueprint_step_fields(step_key);
        if fields.is_empty() {
            return normalized_output.to_string();
        }

        let mut merged = serde_json::Map::new();
        for field in fields {
            merged.insert((*field).to_string(), Value::String(String::new()));
        }

        let value = match Self::extract_output_value(normalized_output) {
            Ok(value) => value,
            Err(_) => {
                let first_key = fields[0];
                merged.insert(
                    first_key.to_string(),
                    Value::String(normalized_output.trim().to_string()),
                );
                return serde_json::to_string_pretty(&merged)
                    .unwrap_or_else(|_| normalized_output.to_string());
            }
        };

        let mut candidate_objects = Vec::<serde_json::Map<String, Value>>::new();
        if let Some(obj) = value.as_object() {
            candidate_objects.push(obj.clone());
            for nested_key in [
                "blueprintStep",
                "content",
                "fields",
                "data",
                "payload",
                "result",
            ] {
                if let Some(nested_obj) = obj.get(nested_key).and_then(Value::as_object) {
                    candidate_objects.push(nested_obj.clone());
                }
            }
        }

        let mut filled_count = 0usize;
        for field in fields {
            let mut aliases = vec![*field];
            aliases.extend(Self::blueprint_field_aliases(field));
            let mut picked: Option<String> = None;
            for candidate in &candidate_objects {
                picked = Self::pick_optional_text(candidate, &aliases);
                if picked.is_some() {
                    break;
                }
            }
            if let Some(text) = picked {
                merged.insert((*field).to_string(), Value::String(text));
                filled_count += 1;
            }
        }

        if filled_count == 0 {
            let suggestion = value
                .as_object()
                .and_then(|obj| obj.get("suggestion"))
                .and_then(Self::json_value_to_text)
                .unwrap_or_else(|| normalized_output.trim().to_string());
            let first_key = fields[0];
            merged.insert(first_key.to_string(), Value::String(suggestion));
        }

        serde_json::to_string_pretty(&merged).unwrap_or_else(|_| normalized_output.to_string())
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use rusqlite::params;

    use super::TaskHandlers;
    use crate::infra::database::open_database;
    use crate::services::ai_pipeline_service::RunAiTaskPipelineInput;
    use crate::services::chapter_service::{ChapterInput, ChapterService};
    use crate::services::character_service::CharacterService;
    use crate::services::project_service::{CreateProjectInput, ProjectService};
    use serde_json::Value;
    use uuid::Uuid;

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-task-handlers-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn build_pipeline_input(
        project_root: &str,
        task_type: &str,
        chapter_id: Option<String>,
    ) -> RunAiTaskPipelineInput {
        RunAiTaskPipelineInput {
            project_root: project_root.to_string(),
            task_type: task_type.to_string(),
            chapter_id,
            ui_action: None,
            user_instruction: "测试输入".to_string(),
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: true,
            persist_mode: None,
            automation_tier: None,
        }
    }

    #[test]
    fn build_character_create_input_maps_nested_character_json() {
        let normalized_output = r#"
        {
          "name": "沈惊寒",
          "aliases": ["寒剑", "冷面修罗"],
          "basicInfo": {
            "age": "二十七岁",
            "gender": "男",
            "occupation": "游历剑客",
            "status": "背负灭门之仇的孤行侠客"
          },
          "appearance": {
            "overview": "身形修长而精瘦",
            "distinctiveFeatures": ["左手小指少一截", "右眼眼尾有淡痣"],
            "style": "终年一身灰黑色劲装"
          },
          "personality": {
            "desires": ["手刃仇人", "渴望被人理解"],
            "fears": ["永远找不到仇人"],
            "flaws": ["被仇恨吞噬"]
          },
          "background": "十年前师门覆灭，他背负血仇。",
          "arc": {
            "potentialGrowth": "复仇之后学会活下去。"
          }
        }
        "#;

        let input = TaskHandlers::build_character_create_input(normalized_output, "fallback")
            .expect("build_character_create_input should parse nested json");

        assert_eq!(input.name, "沈惊寒");
        assert_eq!(input.age.as_deref(), Some("二十七岁"));
        assert_eq!(input.gender.as_deref(), Some("男"));
        assert_eq!(
            input.identity_text.as_deref(),
            Some("游历剑客；背负灭门之仇的孤行侠客")
        );
        assert!(input
            .appearance
            .as_deref()
            .is_some_and(|value| value.contains("特征：左手小指少一截；右眼眼尾有淡痣")));
        assert_eq!(input.desire.as_deref(), Some("手刃仇人；渴望被人理解"));
        assert_eq!(input.fear.as_deref(), Some("永远找不到仇人"));
        assert_eq!(input.flaw.as_deref(), Some("被仇恨吞噬"));
        assert_eq!(input.arc_stage.as_deref(), Some("复仇之后学会活下去。"));
        assert_eq!(input.notes.as_deref(), Some("十年前师门覆灭，他背负血仇。"));
    }

    #[test]
    fn normalize_blueprint_step_content_maps_nested_step_fields() {
        let normalized_output = r#"
        {
          "data": {
            "核心灵感": "被废墟文明打动",
            "核心命题": "秩序与自由的代价",
            "emotion": "压抑中带希望",
            "target_reader": "青年幻想读者",
            "商业卖点": "反乌托邦+修真混搭",
            "reader_expectation": "高冲突与强反转"
          }
        }
        "#;

        let normalized =
            TaskHandlers::normalize_blueprint_step_content("step-01-anchor", normalized_output);
        let parsed: Value =
            serde_json::from_str(&normalized).expect("normalized blueprint content should be json");
        let obj = parsed
            .as_object()
            .expect("normalized blueprint content should be object");

        assert_eq!(
            obj.get("coreInspiration").and_then(Value::as_str),
            Some("被废墟文明打动")
        );
        assert_eq!(
            obj.get("coreProposition").and_then(Value::as_str),
            Some("秩序与自由的代价")
        );
        assert_eq!(
            obj.get("coreEmotion").and_then(Value::as_str),
            Some("压抑中带希望")
        );
        assert_eq!(
            obj.get("targetReader").and_then(Value::as_str),
            Some("青年幻想读者")
        );
        assert_eq!(
            obj.get("sellingPoint").and_then(Value::as_str),
            Some("反乌托邦+修真混搭")
        );
        assert_eq!(
            obj.get("readerExpectation").and_then(Value::as_str),
            Some("高冲突与强反转")
        );
    }

    #[test]
    fn normalize_blueprint_step_content_falls_back_to_first_field_on_non_json() {
        let normalized = TaskHandlers::normalize_blueprint_step_content(
            "step-03-premise",
            "这是一段无法解析为 JSON 的建议文本",
        );
        let parsed: Value =
            serde_json::from_str(&normalized).expect("fallback blueprint content should be json");
        let obj = parsed
            .as_object()
            .expect("fallback blueprint content should be object");

        assert_eq!(
            obj.get("oneLineLogline").and_then(Value::as_str),
            Some("这是一段无法解析为 JSON 的建议文本")
        );
    }

    #[test]
    fn extract_output_object_prefers_common_nested_payload() {
        let raw = r#"
        {
          "meta": { "traceId": "abc" },
          "payload": {
            "title": "灵脉反噬",
            "desc": "每次越阶施法都会损寿。"
          }
        }
        "#;

        let obj = TaskHandlers::extract_output_object(raw, Some("worldRule"))
            .expect("should resolve nested payload object");
        assert_eq!(obj.get("title").and_then(Value::as_str), Some("灵脉反噬"));
        assert_eq!(
            obj.get("desc").and_then(Value::as_str),
            Some("每次越阶施法都会损寿。")
        );
    }

    #[test]
    fn extract_output_value_accepts_json_with_markdown_wrapping_and_prefix_text() {
        let raw = r#"
        这是结果：
        ```json
        {
          "name": "沈惊寒",
          "roleType": "主角",
          "motivation": "复仇并守护遗孤"
        }
        ```
        请直接入库。
        "#;

        let value = TaskHandlers::extract_output_value(raw)
            .expect("extract_output_value should parse markdown wrapped json");
        let obj = value.as_object().expect("parsed value should be object");

        assert_eq!(obj.get("name").and_then(Value::as_str), Some("沈惊寒"));
        assert_eq!(obj.get("roleType").and_then(Value::as_str), Some("主角"));
    }

    #[test]
    fn pick_optional_string_supports_normalized_keys() {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "constraint-level".to_string(),
            Value::String("absolute".to_string()),
        );

        let picked = TaskHandlers::pick_optional_string(&obj, &["constraint_level"]);
        assert_eq!(picked.as_deref(), Some("absolute"));
    }

    #[test]
    fn build_world_rule_create_input_maps_nested_world_rule_json() {
        let normalized_output = r#"
        {
          "payload": {
            "设定名": "血契铁律",
            "类别": "世界规则",
            "description": "施术者必须支付等价代价。",
            "constraint-level": "strong",
            "related_entities": ["血契印", "宗门法典"],
            "examples": "祭火阵反噬",
            "contradiction_policy": "冲突时以铁律优先"
          }
        }
        "#;

        let input = TaskHandlers::build_world_rule_create_input(normalized_output, "fallback")
            .expect("build_world_rule_create_input should parse nested json");
        assert_eq!(input.title, "血契铁律");
        assert_eq!(input.category, "世界规则");
        assert_eq!(input.description, "施术者必须支付等价代价。");
        assert_eq!(input.constraint_level, "strong");
        assert_eq!(
            input.related_entities,
            Some(vec!["血契印".to_string(), "宗门法典".to_string()])
        );
        assert_eq!(input.examples.as_deref(), Some("祭火阵反噬"));
        assert_eq!(
            input.contradiction_policy.as_deref(),
            Some("冲突时以铁律优先")
        );
    }

    #[test]
    fn build_glossary_term_create_input_maps_canonical_schema_json() {
        let normalized_output = r#"
        {
          "content": {
            "canonicalName": "逆命印",
            "category": "法则名",
            "aliases": ["逆印", "命印"],
            "oneLineDefinition": "以寿元换取逆转瞬间因果的禁术标记。",
            "scopeBoundary": "仅用于生死决断场景，日常不可用。",
            "firstUseContext": "第12章祭典审判前夜。",
            "forbiddenMisuse": "不可与治疗术混用。",
            "resolution": "与既有术语无冲突，可直接入库。"
          }
        }
        "#;

        let input = TaskHandlers::build_glossary_term_create_input(normalized_output, "fallback")
            .expect("build_glossary_term_create_input should parse canonical schema");
        assert_eq!(input.term, "逆命印");
        assert_eq!(input.term_type, "法则名");
        assert_eq!(
            input.aliases,
            Some(vec!["逆印".to_string(), "命印".to_string()])
        );
        let description = input.description.unwrap_or_default();
        assert!(description.contains("以寿元换取逆转瞬间因果"));
        assert!(description.contains("适用边界"));
        assert!(description.contains("首次语境"));
        assert!(description.contains("禁用用法"));
        assert!(description.contains("冲突与整合"));
    }

    #[test]
    fn build_narrative_obligation_create_input_maps_tracking_schema_json() {
        let normalized_output = r#"
        {
          "data": {
            "obligationType": "明线伏笔",
            "seedSignal": "她拇指在怀表裂痕上停了一秒。",
            "triggerCondition": "旧案卷宗重启调查",
            "payoffWindow": "第12-14章",
            "fallbackPlan": "若延迟回收，则在第10章追加线索重提。",
            "payoffStatus": "open",
            "riskLevel": "high",
            "relatedEntities": [
              {"target": "旧案真凶浮出"},
              {"name": "沈惊寒角色弧-抉择"}
            ]
          }
        }
        "#;

        let input =
            TaskHandlers::build_narrative_obligation_create_input(normalized_output, "fallback")
                .expect("build_narrative_obligation_create_input should parse tracking schema");

        assert_eq!(input.obligation_type, "明线伏笔");
        assert_eq!(input.payoff_status.as_deref(), Some("open"));
        assert_eq!(input.severity.as_deref(), Some("high"));
        assert!(input.description.contains("埋点信号"));
        assert!(input.description.contains("触发条件"));
        assert!(input.description.contains("回收窗口"));
        assert!(input.description.contains("延期补救"));
        let related = input
            .related_entities
            .as_deref()
            .expect("related_entities should be serialized");
        let related_items: Vec<String> =
            serde_json::from_str(related).expect("related_entities should be json array");
        assert!(related_items.contains(&"旧案真凶浮出".to_string()));
        assert!(related_items.contains(&"沈惊寒角色弧-抉择".to_string()));
    }

    #[test]
    fn build_plot_node_create_input_maps_network_schema_json() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "剧情映射测试".to_string(),
                author: None,
                genre: "玄幻".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project should be created");

        let normalized_output = r#"
        {
          "payload": {
            "nodeTitle": "祭典审判",
            "layer": "A",
            "keyEvent": "主角在祭典上公开指证仇首。",
            "conflictType": "人物 vs 社会",
            "emotionalTone": "压抑转爆发",
            "status": "planning",
            "relatedCharacters": [
              {"name": "沈惊寒"},
              {"name": "玄霄宗主"}
            ]
          }
        }
        "#;

        let input = TaskHandlers::build_plot_node_create_input(
            &project.project_root,
            normalized_output,
            "fallback",
        )
        .expect("build_plot_node_create_input should parse network schema");
        assert_eq!(input.title, "祭典审判");
        assert_eq!(input.node_type, "A");
        assert_eq!(input.goal.as_deref(), Some("主角在祭典上公开指证仇首。"));
        assert_eq!(input.conflict.as_deref(), Some("人物 vs 社会"));
        assert_eq!(input.emotional_curve.as_deref(), Some("压抑转爆发"));
        assert_eq!(input.status.as_deref(), Some("planning"));
        assert_eq!(
            input.related_characters,
            Some(vec!["沈惊寒".to_string(), "玄霄宗主".to_string()])
        );

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn persist_task_output_chapter_plan_updates_chapter_summary_and_target_words() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "章节规划回填测试".to_string(),
                author: None,
                genre: "玄幻".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project should be created");

        let chapter = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: Some("旧摘要".to_string()),
                    target_words: Some(1200),
                    status: Some("drafting".to_string()),
                },
            )
            .expect("chapter should be created");

        let input = build_pipeline_input(
            &project.project_root,
            "chapter.plan",
            Some(chapter.id.clone()),
        );
        let output = r#"
        {
          "chapterFunction": "推进主线并建立对立",
          "successCriteria": "主角完成线索确认并触发下一冲突",
          "emotionalArc": "压抑 -> 爆发",
          "scenes": [
            {"purpose":"线索确认"},
            {"purpose":"正面冲突"}
          ],
          "foreshadowingPlan": "埋下逆命印线索",
          "totalWords": 3600,
          "cliffhanger": "反派现身",
          "notes": "需保持冷峻文风"
        }
        "#;

        let records = TaskHandlers::default()
            .persist_task_output(
                "chapter.plan",
                &project.project_root,
                &input,
                output,
                "req-chapter-plan",
            )
            .expect("chapter plan persist should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].entity_type, "chapter");
        assert_eq!(records[0].entity_id, chapter.id);

        let chapters = chapter_service
            .list_chapters(&project.project_root)
            .expect("list chapters should succeed");
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].target_words, 3600);
        assert_eq!(chapters[0].status, "planned");
        assert!(chapters[0].summary.contains("章节功能"));
        assert!(chapters[0].summary.contains("伏笔处理"));

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn persist_task_output_timeline_review_updates_chapter_rows() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "时间线回填测试".to_string(),
                author: None,
                genre: "玄幻".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project should be created");

        chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: Some("旧摘要".to_string()),
                    target_words: Some(1000),
                    status: Some("drafting".to_string()),
                },
            )
            .expect("chapter should be created");

        let input = build_pipeline_input(&project.project_root, "timeline.review", None);
        let output = r#"
        {
          "timelineEntries": [
            {
              "chapterIndex": 1,
              "title": "第一章 风起",
              "summary": "主角在雪夜确认灭门线索。",
              "status": "planned",
              "targetWords": 2800
            }
          ]
        }
        "#;

        let records = TaskHandlers::default()
            .persist_task_output(
                "timeline.review",
                &project.project_root,
                &input,
                output,
                "req-timeline",
            )
            .expect("timeline persist should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].entity_type, "timeline_entry_batch");
        assert_eq!(records[0].action, "updated:1");

        let chapters = chapter_service
            .list_chapters(&project.project_root)
            .expect("list chapters should succeed");
        assert_eq!(chapters[0].title, "第一章 风起");
        assert_eq!(chapters[0].summary, "主角在雪夜确认灭门线索。");
        assert_eq!(chapters[0].status, "planned");
        assert_eq!(chapters[0].target_words, 2800);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn persist_task_output_records_manual_promotion_provenance() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "来源轨迹记录测试".to_string(),
                author: None,
                genre: "玄幻".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project should be created");

        let mut input = build_pipeline_input(&project.project_root, "character.create", None);
        input.ui_action = Some("book.pipeline.promote.manual".to_string());
        input.automation_tier = Some("confirm".to_string());
        let output = r#"
        {
          "name": "林夜",
          "roleType": "主角",
          "motivation": "守住故土"
        }
        "#;

        let records = TaskHandlers::default()
            .persist_task_output(
                "character.create",
                &project.project_root,
                &input,
                output,
                "req-promo-manual",
            )
            .expect("character create persist should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].entity_type, "character");

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let (source_kind, source_ref, provenance_request_id): (
            String,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT source_kind, source_ref, request_id
                 FROM entity_provenance
                 WHERE project_id = ?1 AND entity_type = 'character' AND entity_id = ?2
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![&project.project.project_id, &records[0].entity_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("provenance row");

        assert_eq!(source_kind, "manual_promotion");
        assert_eq!(source_ref.as_deref(), Some("book.pipeline.promote.manual"));
        assert_eq!(provenance_request_id.as_deref(), Some("req-promo-manual"));

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn persist_task_output_relationship_review_creates_relationship_edges() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let character_service = CharacterService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "关系回填测试".to_string(),
                author: None,
                genre: "玄幻".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project should be created");

        character_service
            .create(
                &project.project_root,
                crate::services::character_service::CreateCharacterInput {
                    name: "沈惊寒".to_string(),
                    role_type: "主角".to_string(),
                    aliases: Some(vec!["寒剑".to_string()]),
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
            .expect("character should be created");
        character_service
            .create(
                &project.project_root,
                crate::services::character_service::CreateCharacterInput {
                    name: "苏晚棠".to_string(),
                    role_type: "配角".to_string(),
                    aliases: None,
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
            .expect("character should be created");

        let input = build_pipeline_input(&project.project_root, "relationship.review", None);
        let output = r#"
        {
          "relationGraph": {
            "edges": [
              {
                "sourceName": "寒剑",
                "targetName": "苏晚棠",
                "relationshipType": "守护与试探",
                "description": "两人在共同追查中形成脆弱同盟。"
              }
            ]
          }
        }
        "#;

        let records = TaskHandlers::default()
            .persist_task_output(
                "relationship.review",
                &project.project_root,
                &input,
                output,
                "req-relationship",
            )
            .expect("relationship persist should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].entity_type, "character_relationship_batch");
        assert_eq!(records[0].action, "inserted:1");

        let relations = crate::services::character_service::RelationshipService::default()
            .list(&project.project_root, None)
            .expect("list relationships should succeed");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].relationship_type, "守护与试探");

        remove_temp_workspace(&workspace);
    }
}
