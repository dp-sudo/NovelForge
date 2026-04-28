use std::collections::HashSet;
use std::fs;
use std::path::Path;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::import_service::{extract_asset_candidates, AssetExtractionCandidate};
use crate::services::project_service::{get_project_id, WritingStyle};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalContext {
    pub project_name: String,
    pub genre: String,
    pub narrative_pov: Option<String>,
    pub writing_style: Option<WritingStyle>,
    pub locked_terms: Vec<String>,
    pub banned_terms: Vec<String>,
    pub blueprint_summary: Vec<BlueprintStepSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintStepSummary {
    pub step_key: String,
    pub title: String,
    pub content: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedContext {
    pub chapter: Option<ChapterSummary>,
    pub characters: Vec<CharacterSummary>,
    pub world_rules: Vec<WorldRuleSummary>,
    pub plot_nodes: Vec<PlotNodeSummary>,
    pub previous_chapter_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSummary {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub chapter_index: i64,
    pub target_words: i64,
    pub current_words: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub role_type: String,
    pub aliases: Option<String>,
    pub motivation: Option<String>,
    pub desire: Option<String>,
    pub fear: Option<String>,
    pub flaw: Option<String>,
    pub arc_stage: Option<String>,
    pub identity_text: Option<String>,
    pub appearance: Option<String>,
    pub locked_fields: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldRuleSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub constraint_level: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotNodeSummary {
    pub id: String,
    pub title: String,
    pub node_type: String,
    pub goal: Option<String>,
    pub conflict: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectedContext {
    pub global_context: GlobalContext,
    pub related_context: RelatedContext,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryContextTerm {
    pub term: String,
    pub term_type: String,
    pub locked: bool,
    pub banned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintContextStep {
    pub step_key: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorChapterContext {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub target_words: i64,
    pub current_words: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorContextPanel {
    pub chapter: EditorChapterContext,
    pub characters: Vec<CharacterSummary>,
    pub world_rules: Vec<WorldRuleSummary>,
    pub plot_nodes: Vec<PlotNodeSummary>,
    pub glossary: Vec<GlossaryContextTerm>,
    pub blueprint: Vec<BlueprintContextStep>,
    pub asset_candidates: Vec<AssetExtractionCandidate>,
    pub relationship_drafts: Vec<RelationshipDraft>,
    pub involvement_drafts: Vec<InvolvementDraft>,
    pub scene_drafts: Vec<SceneDraft>,
    pub previous_chapter_summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyAssetCandidateInput {
    pub label: String,
    pub asset_type: String,
    pub evidence: Option<String>,
    pub target_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyAssetCandidateResult {
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub link_created: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDraft {
    pub source_label: String,
    pub target_label: String,
    pub relationship_type: String,
    pub confidence: f32,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvolvementDraft {
    pub character_label: String,
    pub involvement_type: String,
    pub occurrences: i64,
    pub confidence: f32,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneDraft {
    pub scene_label: String,
    pub scene_type: String,
    pub confidence: f32,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyStructuredDraftInput {
    pub draft_kind: String,
    pub source_label: String,
    pub target_label: Option<String>,
    pub relationship_type: Option<String>,
    pub involvement_type: Option<String>,
    pub scene_type: Option<String>,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyStructuredDraftResult {
    pub action: String,
    pub draft_kind: String,
    pub primary_target_id: String,
    pub secondary_target_id: Option<String>,
}

#[derive(Default)]
pub struct ContextService;

impl ContextService {
    /// Collect editor context panel payload that is consumed by renderer directly.
    pub fn collect_editor_context(
        &self,
        project_root: &str,
        chapter_id: &str,
    ) -> Result<EditorContextPanel, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let related = self.collect_related_context(&conn, &project_id, chapter_id)?;
        let chapter = related
            .chapter
            .clone()
            .ok_or_else(|| AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true))?;
        let glossary = self.collect_glossary_context(&conn, &project_id)?;
        let blueprint = self.collect_blueprint_context(&conn, &project_id)?;
        let chapter_content = conn
            .query_row(
                "SELECT content_path FROM chapters WHERE id = ?1 AND is_deleted = 0",
                params![chapter_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .and_then(|content_path| fs::read_to_string(project_root_path.join(content_path)).ok())
            .map(|content| strip_frontmatter(&content))
            .unwrap_or_default();
        let mut existing_labels: Vec<String> = Vec::new();
        existing_labels.extend(related.characters.iter().map(|item| item.name.clone()));
        existing_labels.extend(related.world_rules.iter().map(|item| item.title.clone()));
        existing_labels.extend(related.plot_nodes.iter().map(|item| item.title.clone()));
        existing_labels.extend(glossary.iter().map(|item| item.term.clone()));
        let asset_candidates = extract_asset_candidates(&chapter_content, &existing_labels, 12);
        let mut character_labels = self.collect_project_character_names(&conn, &project_id)?;
        character_labels.extend(
            asset_candidates
                .iter()
                .filter(|item| item.asset_type == "character")
                .map(|item| item.label.clone()),
        );
        dedupe_labels(&mut character_labels);
        let world_titles = self.collect_project_world_rule_titles(&conn, &project_id)?;
        let relationship_drafts =
            extract_relationship_drafts(&chapter_content, &character_labels, 10);
        let involvement_drafts =
            extract_involvement_drafts(&chapter_content, &character_labels, 10);
        let scene_drafts = extract_scene_drafts(&asset_candidates, &world_titles, 10);

        Ok(EditorContextPanel {
            chapter: EditorChapterContext {
                id: chapter.id,
                title: chapter.title,
                summary: chapter.summary,
                status: chapter.status,
                target_words: chapter.target_words,
                current_words: chapter.current_words,
            },
            characters: related.characters,
            world_rules: related.world_rules,
            plot_nodes: related.plot_nodes,
            glossary,
            blueprint,
            asset_candidates,
            relationship_drafts,
            involvement_drafts,
            scene_drafts,
            previous_chapter_summary: related.previous_chapter_summary,
        })
    }

    /// Collect only global context without requiring a chapter_id.
    pub fn collect_global_context_only(
        &self,
        project_root: &str,
    ) -> Result<CollectedContext, AppErrorDto> {
        let project_root_path = std::path::Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let global = self.collect_global_context(&conn, &project_id)?;
        Ok(CollectedContext {
            global_context: global,
            related_context: RelatedContext {
                chapter: None,
                characters: vec![],
                world_rules: vec![],
                plot_nodes: vec![],
                previous_chapter_summary: None,
            },
        })
    }

    /// Collect full chapter context from the project database.
    pub fn collect_chapter_context(
        &self,
        project_root: &str,
        chapter_id: &str,
    ) -> Result<CollectedContext, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;

        let project_id = get_project_id(&conn)?;

        let global = self.collect_global_context(&conn, &project_id)?;
        let related = self.collect_related_context(&conn, &project_id, chapter_id)?;

        Ok(CollectedContext {
            global_context: global,
            related_context: related,
        })
    }

    /// Apply an extracted candidate into structured assets with chapter linkage.
    /// This keeps ingestion user-reviewed instead of silently auto-writing.
    pub fn apply_asset_candidate(
        &self,
        project_root: &str,
        chapter_id: &str,
        input: ApplyAssetCandidateInput,
    ) -> Result<ApplyAssetCandidateResult, AppErrorDto> {
        let label = input.label.trim().to_string();
        if label.is_empty() {
            return Err(AppErrorDto::new(
                "CANDIDATE_INVALID",
                "候选标签不能为空",
                true,
            ));
        }
        let evidence = input.evidence.unwrap_or_default().trim().to_string();
        let target_type =
            resolve_candidate_target_type(input.target_kind.as_deref(), &input.asset_type)?;

        let project_root_path = Path::new(project_root);
        let mut conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;

        let chapter_exists = conn
            .query_row(
                "SELECT 1 FROM chapters WHERE id = ?1 AND project_id = ?2 AND is_deleted = 0",
                params![chapter_id, &project_id],
                |_row| Ok(()),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询章节失败", true)
                    .with_detail(err.to_string())
            })?
            .is_some();
        if !chapter_exists {
            return Err(AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true));
        }

        let tx = conn.transaction().map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "无法写入项目数据库", true)
                .with_detail(err.to_string())
        })?;
        let (target_id, action) = match target_type.as_str() {
            "character" => self.find_or_create_character(&tx, &project_id, &label, &evidence)?,
            "world_rule" => self.find_or_create_world_rule(
                &tx,
                &project_id,
                &label,
                &input.asset_type,
                &evidence,
            )?,
            "plot_node" => self.find_or_create_plot_node(&tx, &project_id, &label, &evidence)?,
            "glossary_term" => self.find_or_create_glossary_term(
                &tx,
                &project_id,
                &label,
                &input.asset_type,
                &evidence,
            )?,
            _ => {
                return Err(AppErrorDto::new(
                    "CANDIDATE_TARGET_INVALID",
                    "不支持的候选目标类型",
                    true,
                ))
            }
        };

        let link_created =
            self.ensure_chapter_link(&tx, &project_id, chapter_id, &target_type, &target_id)?;
        tx.commit().map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "保存候选失败", true).with_detail(err.to_string())
        })?;

        Ok(ApplyAssetCandidateResult {
            action,
            target_type,
            target_id,
            link_created,
            label,
        })
    }

    /// Apply one reviewed structured draft into database.
    /// Drafts are generated automatically but persisted only after user confirmation.
    pub fn apply_structured_draft(
        &self,
        project_root: &str,
        chapter_id: &str,
        input: ApplyStructuredDraftInput,
    ) -> Result<ApplyStructuredDraftResult, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let mut conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let chapter_exists = conn
            .query_row(
                "SELECT 1 FROM chapters WHERE id = ?1 AND project_id = ?2 AND is_deleted = 0",
                params![chapter_id, &project_id],
                |_row| Ok(()),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询章节失败", true)
                    .with_detail(err.to_string())
            })?
            .is_some();
        if !chapter_exists {
            return Err(AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true));
        }

        let draft_kind = input.draft_kind.trim().to_ascii_lowercase();
        let source_label = input.source_label.trim().to_string();
        if source_label.is_empty() {
            return Err(AppErrorDto::new("DRAFT_INVALID", "草案内容为空", true));
        }
        let evidence = input.evidence.unwrap_or_default().trim().to_string();

        let tx = conn.transaction().map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "无法写入项目数据库", true)
                .with_detail(err.to_string())
        })?;

        let result = match draft_kind.as_str() {
            "relationship" => {
                let target_label = input
                    .target_label
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if target_label.is_empty() {
                    return Err(AppErrorDto::new(
                        "DRAFT_INVALID",
                        "关系草案缺少目标角色",
                        true,
                    ));
                }
                if source_label == target_label {
                    return Err(AppErrorDto::new(
                        "DRAFT_INVALID",
                        "关系草案角色不能相同",
                        true,
                    ));
                }
                let relationship_type = input
                    .relationship_type
                    .unwrap_or_else(|| "互动".to_string())
                    .trim()
                    .to_string();

                let (source_id, _) =
                    self.find_or_create_character(&tx, &project_id, &source_label, &evidence)?;
                let (target_id, _) =
                    self.find_or_create_character(&tx, &project_id, &target_label, &evidence)?;
                let existing_relation_id = tx
                    .query_row(
                        "SELECT id FROM character_relationships WHERE project_id = ?1 AND relationship_type = ?2 AND ((source_character_id = ?3 AND target_character_id = ?4) OR (source_character_id = ?4 AND target_character_id = ?3)) LIMIT 1",
                        params![&project_id, &relationship_type, &source_id, &target_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|err| {
                        AppErrorDto::new("DB_QUERY_FAILED", "查询角色关系失败", true)
                            .with_detail(err.to_string())
                    })?;
                let action = if existing_relation_id.is_some() {
                    "reused".to_string()
                } else {
                    let relation_id = Uuid::new_v4().to_string();
                    let now = now_iso();
                    tx.execute(
                        "INSERT INTO character_relationships(id, project_id, source_character_id, target_character_id, relationship_type, description, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                        params![&relation_id, &project_id, &source_id, &target_id, &relationship_type, if evidence.is_empty() { None::<String> } else { Some(evidence.clone()) }, &now, &now],
                    )
                    .map_err(|err| {
                        AppErrorDto::new("DB_WRITE_FAILED", "创建角色关系失败", true)
                            .with_detail(err.to_string())
                    })?;
                    "created".to_string()
                };
                let _ = self.ensure_chapter_link_with_relation(
                    &tx,
                    &project_id,
                    chapter_id,
                    "character",
                    &source_id,
                    "relationship_context",
                )?;
                let _ = self.ensure_chapter_link_with_relation(
                    &tx,
                    &project_id,
                    chapter_id,
                    "character",
                    &target_id,
                    "relationship_context",
                )?;
                ApplyStructuredDraftResult {
                    action,
                    draft_kind: "relationship".to_string(),
                    primary_target_id: source_id,
                    secondary_target_id: Some(target_id),
                }
            }
            "involvement" => {
                let involvement_type = input
                    .involvement_type
                    .unwrap_or_else(|| "一般戏份".to_string())
                    .trim()
                    .to_string();
                let (character_id, _) =
                    self.find_or_create_character(&tx, &project_id, &source_label, &evidence)?;
                let relation_type = format!("involvement:{}", involvement_type);
                let link_created = self.ensure_chapter_link_with_relation(
                    &tx,
                    &project_id,
                    chapter_id,
                    "character",
                    &character_id,
                    &relation_type,
                )?;
                ApplyStructuredDraftResult {
                    action: if link_created { "created" } else { "reused" }.to_string(),
                    draft_kind: "involvement".to_string(),
                    primary_target_id: character_id,
                    secondary_target_id: None,
                }
            }
            "scene" => {
                let scene_type = input
                    .scene_type
                    .unwrap_or_else(|| "地点场景".to_string())
                    .trim()
                    .to_string();
                let existing_world_rule_id = tx
                    .query_row(
                        "SELECT id FROM world_rules WHERE project_id = ?1 AND title = ?2 AND is_deleted = 0 LIMIT 1",
                        params![&project_id, &source_label],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|err| {
                        AppErrorDto::new("DB_QUERY_FAILED", "查询场景设定失败", true)
                            .with_detail(err.to_string())
                    })?;
                let world_rule_id = if let Some(existing_id) = existing_world_rule_id {
                    existing_id
                } else {
                    let id = Uuid::new_v4().to_string();
                    let now = now_iso();
                    let description = if evidence.is_empty() {
                        format!("场景草案：{}", source_label)
                    } else {
                        format!("场景线索：{}", evidence)
                    };
                    tx.execute(
                        "INSERT INTO world_rules(id, project_id, title, category, description, constraint_level, related_entities, examples, contradiction_policy, is_deleted, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,NULL,NULL,0,?8,?9)",
                        params![&id, &project_id, &source_label, "场景", &description, "normal", "[]", &now, &now],
                    )
                    .map_err(|err| {
                        AppErrorDto::new("DB_WRITE_FAILED", "创建场景设定失败", true)
                            .with_detail(err.to_string())
                    })?;
                    id
                };
                let relation_type = format!("scene:{}", scene_type);
                let link_created = self.ensure_chapter_link_with_relation(
                    &tx,
                    &project_id,
                    chapter_id,
                    "world_rule",
                    &world_rule_id,
                    &relation_type,
                )?;
                ApplyStructuredDraftResult {
                    action: if link_created { "created" } else { "reused" }.to_string(),
                    draft_kind: "scene".to_string(),
                    primary_target_id: world_rule_id,
                    secondary_target_id: None,
                }
            }
            _ => {
                return Err(AppErrorDto::new(
                    "DRAFT_KIND_INVALID",
                    "不支持的结构化草案类型",
                    true,
                ))
            }
        };

        tx.commit().map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "保存结构化草案失败", true)
                .with_detail(err.to_string())
        })?;
        Ok(result)
    }

    fn find_or_create_character(
        &self,
        tx: &rusqlite::Transaction<'_>,
        project_id: &str,
        label: &str,
        evidence: &str,
    ) -> Result<(String, String), AppErrorDto> {
        if let Some(existing_id) = tx
            .query_row(
                "SELECT id FROM characters WHERE project_id = ?1 AND name = ?2 AND is_deleted = 0 LIMIT 1",
                params![project_id, label],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询角色失败", true).with_detail(err.to_string())
            })?
        {
            return Ok((existing_id, "reused".to_string()));
        }

        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let notes = if evidence.is_empty() {
            Some("来源线索：章节候选提取".to_string())
        } else {
            Some(format!("来源线索：{}", evidence))
        };
        tx.execute(
            "INSERT INTO characters(id, project_id, name, aliases, role_type, age, gender, identity_text, appearance, motivation, desire, fear, flaw, arc_stage, locked_fields, notes, is_deleted, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,?6,?7,0,?8,?9)",
            params![id, project_id, label, "[]", "配角", "[]", notes, now, now],
        )
        .map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "创建角色失败", true).with_detail(err.to_string())
        })?;
        Ok((id, "created".to_string()))
    }

    fn find_or_create_world_rule(
        &self,
        tx: &rusqlite::Transaction<'_>,
        project_id: &str,
        label: &str,
        asset_type: &str,
        evidence: &str,
    ) -> Result<(String, String), AppErrorDto> {
        if let Some(existing_id) = tx
            .query_row(
                "SELECT id FROM world_rules WHERE project_id = ?1 AND title = ?2 AND is_deleted = 0 LIMIT 1",
                params![project_id, label],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询设定失败", true).with_detail(err.to_string())
            })?
        {
            return Ok((existing_id, "reused".to_string()));
        }

        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let category = match asset_type {
            "location" => "地理场景",
            "organization" => "势力组织",
            "world_rule" => "规则体系",
            _ => "通用设定",
        };
        let description = if evidence.is_empty() {
            format!("章节新增线索：{}", label)
        } else {
            format!("章节线索：{}", evidence)
        };
        tx.execute(
            "INSERT INTO world_rules(id, project_id, title, category, description, constraint_level, related_entities, examples, contradiction_policy, is_deleted, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,NULL,NULL,0,?8,?9)",
            params![id, project_id, label, category, description, "normal", "[]", now, now],
        )
        .map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "创建设定失败", true).with_detail(err.to_string())
        })?;
        Ok((id, "created".to_string()))
    }

    fn find_or_create_plot_node(
        &self,
        tx: &rusqlite::Transaction<'_>,
        project_id: &str,
        label: &str,
        evidence: &str,
    ) -> Result<(String, String), AppErrorDto> {
        if let Some(existing_id) = tx
            .query_row(
                "SELECT id FROM plot_nodes WHERE project_id = ?1 AND title = ?2 LIMIT 1",
                params![project_id, label],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询剧情节点失败", true)
                    .with_detail(err.to_string())
            })?
        {
            return Ok((existing_id, "reused".to_string()));
        }

        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let next_sort_order = tx
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM plot_nodes WHERE project_id = ?1",
                params![project_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询剧情节点排序失败", true)
                    .with_detail(err.to_string())
            })?;
        let goal = if evidence.is_empty() {
            Some(format!("由章节线索补充：{}", label))
        } else {
            Some(evidence.to_string())
        };

        tx.execute(
            "INSERT INTO plot_nodes(id, project_id, title, node_type, sort_order, goal, conflict, emotional_curve, status, related_characters, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,NULL,NULL,?7,?8,?9,?10)",
            params![id, project_id, label, "支线", next_sort_order, goal, "planning", "[]", now, now],
        )
        .map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "创建剧情节点失败", true)
                .with_detail(err.to_string())
        })?;
        Ok((id, "created".to_string()))
    }

    fn find_or_create_glossary_term(
        &self,
        tx: &rusqlite::Transaction<'_>,
        project_id: &str,
        label: &str,
        asset_type: &str,
        evidence: &str,
    ) -> Result<(String, String), AppErrorDto> {
        if let Some(existing_id) = tx
            .query_row(
                "SELECT id FROM glossary_terms WHERE project_id = ?1 AND term = ?2 LIMIT 1",
                params![project_id, label],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询名词失败", true)
                    .with_detail(err.to_string())
            })?
        {
            return Ok((existing_id, "reused".to_string()));
        }

        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let term_type = match asset_type {
            "character" => "人名",
            "location" => "地名",
            "organization" => "组织",
            _ => "术语",
        };
        let description = if evidence.is_empty() {
            None
        } else {
            Some(format!("来源线索：{}", evidence))
        };
        tx.execute(
            "INSERT INTO glossary_terms(id, project_id, term, term_type, aliases, description, locked, banned, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,0,0,?7,?8)",
            params![id, project_id, label, term_type, "[]", description, now, now],
        )
        .map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "创建名词失败", true).with_detail(err.to_string())
        })?;
        Ok((id, "created".to_string()))
    }

    fn ensure_chapter_link(
        &self,
        tx: &rusqlite::Transaction<'_>,
        project_id: &str,
        chapter_id: &str,
        target_type: &str,
        target_id: &str,
    ) -> Result<bool, AppErrorDto> {
        self.ensure_chapter_link_with_relation(
            tx,
            project_id,
            chapter_id,
            target_type,
            target_id,
            "candidate_adopted",
        )
    }

    fn ensure_chapter_link_with_relation(
        &self,
        tx: &rusqlite::Transaction<'_>,
        project_id: &str,
        chapter_id: &str,
        target_type: &str,
        target_id: &str,
        relation_type: &str,
    ) -> Result<bool, AppErrorDto> {
        let exists = tx
            .query_row(
                "SELECT 1 FROM chapter_links WHERE chapter_id = ?1 AND target_type = ?2 AND target_id = ?3 LIMIT 1",
                params![chapter_id, target_type, target_id],
                |_row| Ok(()),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询章节关联失败", true)
                    .with_detail(err.to_string())
            })?
            .is_some();
        if exists {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO chapter_links(id, project_id, chapter_id, target_type, target_id, relation_type, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                Uuid::new_v4().to_string(),
                project_id,
                chapter_id,
                target_type,
                target_id,
                relation_type,
                now_iso()
            ],
        )
        .map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "写入章节关联失败", true).with_detail(err.to_string())
        })?;
        Ok(true)
    }

    fn collect_project_character_names(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<Vec<String>, AppErrorDto> {
        conn.prepare("SELECT name FROM characters WHERE project_id = ?1 AND is_deleted = 0")
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询角色失败", true))?
            .query_map(params![project_id], |row| row.get::<_, String>(0))
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询角色失败", true))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询角色失败", true))
    }

    fn collect_project_world_rule_titles(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<Vec<String>, AppErrorDto> {
        conn.prepare("SELECT title FROM world_rules WHERE project_id = ?1 AND is_deleted = 0")
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询设定失败", true))?
            .query_map(params![project_id], |row| row.get::<_, String>(0))
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询设定失败", true))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询设定失败", true))
    }

    fn collect_global_context(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<GlobalContext, AppErrorDto> {
        // Project info
        let project = conn
            .query_row(
                "SELECT name, genre, narrative_pov, writing_style FROM projects WHERE id = ?1",
                params![project_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .map_err(|err| {
                AppErrorDto::new("PROJECT_NOT_FOUND", "项目不存在", false)
                    .with_detail(err.to_string())
            })?;
        let writing_style = project
            .3
            .and_then(|json| serde_json::from_str::<WritingStyle>(&json).ok());

        // Locked & banned terms from glossary
        let locked_terms: Vec<String> = conn
            .prepare("SELECT term FROM glossary_terms WHERE project_id = ?1 AND locked = 1")
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询名词库失败", true))?
            .query_map(params![project_id], |row| row.get::<_, String>(0))
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询名词库失败", true))?
            .filter_map(|r| r.ok())
            .collect();

        let banned_terms: Vec<String> = conn
            .prepare("SELECT term FROM glossary_terms WHERE project_id = ?1 AND banned = 1")
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询禁用词失败", true))?
            .query_map(params![project_id], |row| row.get::<_, String>(0))
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询禁用词失败", true))?
            .filter_map(|r| r.ok())
            .collect();

        // Blueprint steps
        let blueprint_summary: Vec<BlueprintStepSummary> = conn
            .prepare(
                "SELECT step_key, title, content, status FROM blueprint_steps WHERE project_id = ?1 ORDER BY step_key",
            )
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询蓝图失败", true))?
            .query_map(params![project_id], |row| {
                Ok(BlueprintStepSummary {
                    step_key: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get::<_, Option<String>>(2)?,
                    status: row.get(3)?,
                })
            })
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询蓝图失败", true))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(GlobalContext {
            project_name: project.0,
            genre: project.1,
            narrative_pov: project.2,
            writing_style,
            locked_terms,
            banned_terms,
            blueprint_summary,
        })
    }

    fn collect_glossary_context(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<Vec<GlossaryContextTerm>, AppErrorDto> {
        conn.prepare(
            "SELECT term, term_type, locked, banned FROM glossary_terms WHERE project_id = ?1 ORDER BY term",
        )
        .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询名词库失败", true))?
        .query_map(params![project_id], |row| {
            Ok(GlossaryContextTerm {
                term: row.get(0)?,
                term_type: row.get(1)?,
                locked: row.get::<_, i64>(2)? != 0,
                banned: row.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询名词库失败", true))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询名词库失败", true))
    }

    fn collect_blueprint_context(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<Vec<BlueprintContextStep>, AppErrorDto> {
        conn.prepare(
            "SELECT step_key, content FROM blueprint_steps WHERE project_id = ?1 ORDER BY step_key",
        )
        .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询蓝图失败", true))?
        .query_map(params![project_id], |row| {
            Ok(BlueprintContextStep {
                step_key: row.get(0)?,
                content: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            })
        })
        .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询蓝图失败", true))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询蓝图失败", true))
    }

    fn collect_related_context(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
        chapter_id: &str,
    ) -> Result<RelatedContext, AppErrorDto> {
        // Current chapter info
        let chapter = conn
            .query_row(
                "SELECT id, title, summary, status, chapter_index, target_words, current_words FROM chapters WHERE id = ?1 AND is_deleted = 0",
                params![chapter_id],
                |row| {
                    Ok(ChapterSummary {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        summary: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        status: row.get(3)?,
                        chapter_index: row.get(4)?,
                        target_words: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                        current_words: row.get::<_, Option<i64>>(6)?.unwrap_or(0),
                    })
                },
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_QUERY_FAILED", "查询章节失败", true)
                    .with_detail(err.to_string())
            })?;

        // Characters linked to this chapter
        let characters: Vec<CharacterSummary> = conn
            .prepare(
                r#"
                SELECT c.id, c.name, c.role_type, c.aliases, c.motivation, c.desire,
                       c.fear, c.flaw, c.arc_stage, c.identity_text, c.appearance, c.locked_fields
                FROM characters c
                JOIN chapter_links cl ON cl.target_id = c.id
                WHERE cl.chapter_id = ?1 AND cl.target_type = 'character' AND c.is_deleted = 0
                "#,
            )
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询角色失败", true))?
            .query_map(params![chapter_id], |row| {
                Ok(CharacterSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    role_type: row.get(2)?,
                    aliases: row.get::<_, Option<String>>(3)?,
                    motivation: row.get::<_, Option<String>>(4)?,
                    desire: row.get::<_, Option<String>>(5)?,
                    fear: row.get::<_, Option<String>>(6)?,
                    flaw: row.get::<_, Option<String>>(7)?,
                    arc_stage: row.get::<_, Option<String>>(8)?,
                    identity_text: row.get::<_, Option<String>>(9)?,
                    appearance: row.get::<_, Option<String>>(10)?,
                    locked_fields: row.get::<_, Option<String>>(11)?,
                })
            })
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询角色失败", true))?
            .filter_map(|r| r.ok())
            .collect();

        // World rules linked to this chapter
        let world_rules: Vec<WorldRuleSummary> = conn
            .prepare(
                r#"
                SELECT w.id, w.title, w.category, w.description, w.constraint_level
                FROM world_rules w
                JOIN chapter_links cl ON cl.target_id = w.id
                WHERE cl.chapter_id = ?1 AND cl.target_type = 'world_rule' AND w.is_deleted = 0
                "#,
            )
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询世界规则失败", true))?
            .query_map(params![chapter_id], |row| {
                Ok(WorldRuleSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    category: row.get(2)?,
                    description: row.get(3)?,
                    constraint_level: row.get(4)?,
                })
            })
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询世界规则失败", true))?
            .filter_map(|r| r.ok())
            .collect();

        // Plot nodes linked to this chapter
        let plot_nodes: Vec<PlotNodeSummary> = conn
            .prepare(
                r#"
                SELECT p.id, p.title, p.node_type, p.goal, p.conflict, p.sort_order
                FROM plot_nodes p
                JOIN chapter_links cl ON cl.target_id = p.id
                WHERE cl.chapter_id = ?1 AND cl.target_type = 'plot_node'
                "#,
            )
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询主线节点失败", true))?
            .query_map(params![chapter_id], |row| {
                Ok(PlotNodeSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    node_type: row.get(2)?,
                    goal: row.get::<_, Option<String>>(3)?,
                    conflict: row.get::<_, Option<String>>(4)?,
                    sort_order: row.get(5)?,
                })
            })
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询主线节点失败", true))?
            .filter_map(|r| r.ok())
            .collect();

        // Previous chapter summary
        let previous_chapter_summary: Option<String> = chapter.as_ref().and_then(|ch| {
            let prev_index = ch.chapter_index - 1;
            if prev_index < 1 {
                return None;
            }
            conn.query_row(
                "SELECT summary FROM chapters WHERE project_id = ?1 AND chapter_index = ?2 AND is_deleted = 0",
                params![project_id, prev_index],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
        });

        Ok(RelatedContext {
            chapter,
            characters,
            world_rules,
            plot_nodes,
            previous_chapter_summary,
        })
    }
}

fn dedupe_labels(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|item| {
        let key = normalize_label_key(item);
        if key.is_empty() || seen.contains(&key) {
            return false;
        }
        seen.insert(key);
        true
    });
}

fn normalize_label_key(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn split_sentences(content: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for ch in content.chars() {
        if matches!(ch, '。' | '！' | '？' | '!' | '?' | '\n') {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                sentences.push(trimmed.to_string());
            }
            current.clear();
            continue;
        }
        current.push(ch);
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        sentences.push(trimmed.to_string());
    }
    sentences
}

fn infer_relationship_type(sentence: &str) -> (&'static str, f32) {
    if sentence.contains("师父")
        || sentence.contains("师尊")
        || sentence.contains("徒弟")
        || sentence.contains("弟子")
    {
        return ("师徒", 0.86);
    }
    if sentence.contains("父亲")
        || sentence.contains("母亲")
        || sentence.contains("哥哥")
        || sentence.contains("姐姐")
        || sentence.contains("弟弟")
        || sentence.contains("妹妹")
    {
        return ("亲属", 0.84);
    }
    if sentence.contains("恋人")
        || sentence.contains("夫妻")
        || sentence.contains("未婚妻")
        || sentence.contains("未婚夫")
    {
        return ("情感", 0.83);
    }
    if sentence.contains("朋友")
        || sentence.contains("同伴")
        || sentence.contains("伙伴")
        || sentence.contains("盟友")
    {
        return ("同盟", 0.78);
    }
    if sentence.contains("敌人")
        || sentence.contains("仇人")
        || sentence.contains("死敌")
        || sentence.contains("追杀")
    {
        return ("对立", 0.82);
    }
    if sentence.contains("上司")
        || sentence.contains("下属")
        || sentence.contains("部下")
        || sentence.contains("统领")
    {
        return ("上下级", 0.79);
    }
    ("互动", 0.62)
}

fn extract_sentence_evidence(content: &str, token: &str) -> String {
    let sentences = split_sentences(content);
    if let Some(sentence) = sentences.iter().find(|line| line.contains(token)) {
        return sentence.chars().take(80).collect();
    }
    token.to_string()
}

fn extract_relationship_drafts(
    content: &str,
    character_labels: &[String],
    limit: usize,
) -> Vec<RelationshipDraft> {
    if limit == 0 || content.trim().is_empty() {
        return Vec::new();
    }
    let mut names = character_labels
        .iter()
        .map(|name| name.trim().to_string())
        .filter(|name| {
            let len = name.chars().count();
            !name.is_empty() && len >= 2 && len <= 12
        })
        .collect::<Vec<_>>();
    dedupe_labels(&mut names);
    if names.len() < 2 {
        return Vec::new();
    }

    let mut drafts = Vec::new();
    let mut seen = HashSet::new();
    for sentence in split_sentences(content) {
        let present = names
            .iter()
            .filter(|name| sentence.contains(name.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if present.len() < 2 {
            continue;
        }
        let (relationship_type, confidence) = infer_relationship_type(&sentence);
        for i in 0..present.len() {
            for j in (i + 1)..present.len() {
                let a = &present[i];
                let b = &present[j];
                let mut pair = vec![normalize_label_key(a), normalize_label_key(b)];
                pair.sort();
                let key = format!("{}|{}|{}", pair[0], pair[1], relationship_type);
                if seen.contains(&key) {
                    continue;
                }
                seen.insert(key);
                drafts.push(RelationshipDraft {
                    source_label: a.clone(),
                    target_label: b.clone(),
                    relationship_type: relationship_type.to_string(),
                    confidence,
                    evidence: sentence.chars().take(100).collect(),
                });
                if drafts.len() >= limit {
                    return drafts;
                }
            }
        }
    }
    drafts
}

fn extract_involvement_drafts(
    content: &str,
    character_labels: &[String],
    limit: usize,
) -> Vec<InvolvementDraft> {
    if limit == 0 || content.trim().is_empty() {
        return Vec::new();
    }
    let mut names = character_labels
        .iter()
        .map(|name| name.trim().to_string())
        .filter(|name| {
            let len = name.chars().count();
            !name.is_empty() && len >= 2 && len <= 12
        })
        .collect::<Vec<_>>();
    dedupe_labels(&mut names);

    let mut drafts = Vec::new();
    for name in names {
        let occurrences = content.matches(&name).count() as i64;
        if occurrences < 2 {
            continue;
        }
        let (involvement_type, confidence) = if occurrences >= 6 {
            ("核心戏份", 0.88)
        } else if occurrences >= 4 {
            ("主要戏份", 0.78)
        } else {
            ("一般戏份", 0.66)
        };
        drafts.push(InvolvementDraft {
            character_label: name.clone(),
            involvement_type: involvement_type.to_string(),
            occurrences,
            confidence,
            evidence: extract_sentence_evidence(content, &name),
        });
    }
    drafts.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
    drafts.into_iter().take(limit).collect()
}

fn extract_scene_drafts(
    asset_candidates: &[AssetExtractionCandidate],
    existing_world_titles: &[String],
    limit: usize,
) -> Vec<SceneDraft> {
    if limit == 0 {
        return Vec::new();
    }
    let existing = existing_world_titles
        .iter()
        .map(|item| normalize_label_key(item))
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut drafts = Vec::new();
    for candidate in asset_candidates {
        let scene_type = match candidate.asset_type.as_str() {
            "location" => "地点场景",
            "organization" => "组织场景",
            "world_rule" => "规则场景",
            _ => continue,
        };
        let key = normalize_label_key(&candidate.label);
        if key.is_empty() || existing.contains(&key) || seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        drafts.push(SceneDraft {
            scene_label: candidate.label.clone(),
            scene_type: scene_type.to_string(),
            confidence: candidate.confidence,
            evidence: candidate.evidence.clone(),
        });
        if drafts.len() >= limit {
            break;
        }
    }
    drafts
}

fn resolve_candidate_target_type(
    target_kind: Option<&str>,
    asset_type: &str,
) -> Result<String, AppErrorDto> {
    if let Some(kind) = target_kind {
        let normalized = kind.trim().to_ascii_lowercase();
        let value = match normalized.as_str() {
            "character" => "character",
            "world" | "world_rule" => "world_rule",
            "plot" | "plot_node" => "plot_node",
            "glossary" | "glossary_term" | "term" => "glossary_term",
            _ => {
                return Err(AppErrorDto::new(
                    "CANDIDATE_TARGET_INVALID",
                    "目标类型不支持",
                    true,
                ))
            }
        };
        return Ok(value.to_string());
    }

    let value = match asset_type {
        "character" => "character",
        "location" | "organization" | "world_rule" => "world_rule",
        "term" => "glossary_term",
        _ => "glossary_term",
    };
    Ok(value.to_string())
}

fn strip_frontmatter(content: &str) -> String {
    if !content.starts_with("---\n") {
        return content.to_string();
    }
    if let Some(offset) = content[4..].find("\n---\n") {
        return content[(offset + 9)..].trim().to_string();
    }
    content.to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use rusqlite::params;
    use uuid::Uuid;

    use super::{ApplyAssetCandidateInput, ApplyStructuredDraftInput, ContextService};
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
    fn apply_asset_candidate_creates_character_and_reuses_it() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let context_service = ContextService;

        let project = project_service
            .create_project(CreateProjectInput {
                name: "候选采纳测试".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");
        let chapter = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: None,
                    target_words: None,
                    status: None,
                },
            )
            .expect("chapter created");

        let first = context_service
            .apply_asset_candidate(
                &project.project_root,
                &chapter.id,
                ApplyAssetCandidateInput {
                    label: "林夜".to_string(),
                    asset_type: "character".to_string(),
                    evidence: Some("林夜再次出现".to_string()),
                    target_kind: Some("character".to_string()),
                },
            )
            .expect("apply candidate first time");
        assert_eq!(first.action, "created");
        assert_eq!(first.target_type, "character");
        assert!(first.link_created);

        let second = context_service
            .apply_asset_candidate(
                &project.project_root,
                &chapter.id,
                ApplyAssetCandidateInput {
                    label: "林夜".to_string(),
                    asset_type: "character".to_string(),
                    evidence: Some("林夜再次出现".to_string()),
                    target_kind: Some("character".to_string()),
                },
            )
            .expect("apply candidate second time");
        assert_eq!(second.action, "reused");
        assert!(!second.link_created);

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let character_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM characters WHERE name = ?1 AND is_deleted = 0",
                params!["林夜"],
                |row| row.get(0),
            )
            .expect("character count");
        let link_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chapter_links WHERE chapter_id = ?1 AND target_type = 'character'",
                params![chapter.id],
                |row| row.get(0),
            )
            .expect("link count");
        assert_eq!(character_count, 1);
        assert_eq!(link_count, 1);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn apply_structured_relationship_creates_and_reuses_relationship() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let context_service = ContextService;

        let project = project_service
            .create_project(CreateProjectInput {
                name: "结构化关系测试".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");
        let chapter = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: None,
                    target_words: None,
                    status: None,
                },
            )
            .expect("chapter created");

        let first = context_service
            .apply_structured_draft(
                &project.project_root,
                &chapter.id,
                ApplyStructuredDraftInput {
                    draft_kind: "relationship".to_string(),
                    source_label: "林夜".to_string(),
                    target_label: Some("李伯".to_string()),
                    relationship_type: Some("同盟".to_string()),
                    involvement_type: None,
                    scene_type: None,
                    evidence: Some("林夜与李伯并肩迎敌".to_string()),
                },
            )
            .expect("apply structured relationship first time");
        assert_eq!(first.action, "created");

        let second = context_service
            .apply_structured_draft(
                &project.project_root,
                &chapter.id,
                ApplyStructuredDraftInput {
                    draft_kind: "relationship".to_string(),
                    source_label: "林夜".to_string(),
                    target_label: Some("李伯".to_string()),
                    relationship_type: Some("同盟".to_string()),
                    involvement_type: None,
                    scene_type: None,
                    evidence: Some("林夜与李伯并肩迎敌".to_string()),
                },
            )
            .expect("apply structured relationship second time");
        assert_eq!(second.action, "reused");

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let relation_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM character_relationships WHERE relationship_type = '同盟'",
                [],
                |row| row.get(0),
            )
            .expect("relation count");
        assert_eq!(relation_count, 1);

        remove_temp_workspace(&workspace);
    }
}
