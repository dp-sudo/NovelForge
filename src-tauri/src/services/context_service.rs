use std::collections::{hash_map::DefaultHasher, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::path_utils::resolve_project_relative_path;
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
    pub id: String,
    pub batch_id: String,
    pub status: String,
    pub source_label: String,
    pub target_label: String,
    pub relationship_type: String,
    pub confidence: f32,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvolvementDraft {
    pub id: String,
    pub batch_id: String,
    pub status: String,
    pub character_label: String,
    pub involvement_type: String,
    pub occurrences: i64,
    pub confidence: f32,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneDraft {
    pub id: String,
    pub batch_id: String,
    pub status: String,
    pub scene_label: String,
    pub scene_type: String,
    pub confidence: f32,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyStructuredDraftInput {
    pub draft_item_id: Option<String>,
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
    pub draft_item_id: Option<String>,
    pub draft_item_status: Option<String>,
    pub primary_target_id: String,
    pub secondary_target_id: Option<String>,
}

#[derive(Debug, Clone)]
struct ExtractedRelationshipDraft {
    source_label: String,
    target_label: String,
    relationship_type: String,
    confidence: f32,
    evidence: String,
}

#[derive(Debug, Clone)]
struct ExtractedInvolvementDraft {
    character_label: String,
    involvement_type: String,
    occurrences: i64,
    confidence: f32,
    evidence: String,
}

#[derive(Debug, Clone)]
struct ExtractedSceneDraft {
    scene_label: String,
    scene_type: String,
    confidence: f32,
    evidence: String,
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
        let mut conn = open_database(project_root_path).map_err(|err| {
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
        let chapter_content = match conn
            .query_row(
                "SELECT content_path FROM chapters WHERE id = ?1 AND is_deleted = 0",
                params![chapter_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("CONTEXT_COLLECT_FAILED", "无法读取章节路径", true)
                    .with_detail(err.to_string())
            })? {
            Some(content_path) => {
                let chapter_file = resolve_project_relative_path(project_root_path, &content_path)
                    .map_err(|detail| {
                        AppErrorDto::new("CONTEXT_COLLECT_FAILED", "章节路径无效", true)
                            .with_detail(detail)
                    })?;
                let content = fs::read_to_string(&chapter_file).map_err(|err| {
                    AppErrorDto::new("CONTEXT_COLLECT_FAILED", "无法读取章节正文", true)
                        .with_detail(err.to_string())
                })?;
                strip_frontmatter(&content)
            }
            None => String::new(),
        };
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
        self.persist_structured_draft_pool(
            &mut conn,
            &project_id,
            chapter_id,
            "editor.context.extract",
            &chapter_content,
            &relationship_drafts,
            &involvement_drafts,
            &scene_drafts,
        )?;
        let (relationship_drafts, involvement_drafts, scene_drafts) =
            self.load_structured_draft_pool(&conn, &project_id, chapter_id)?;

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

        let mut draft_kind = input.draft_kind.trim().to_ascii_lowercase();
        let mut source_label = input.source_label.trim().to_string();
        let mut target_label = input
            .target_label
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut relationship_type = input
            .relationship_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut involvement_type = input
            .involvement_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut scene_type = input
            .scene_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut evidence = input.evidence.unwrap_or_default().trim().to_string();
        let draft_item_id = input
            .draft_item_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let tx = conn.transaction().map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "无法写入项目数据库", true)
                .with_detail(err.to_string())
        })?;

        if let Some(ref item_id) = draft_item_id {
            let item = tx
                .query_row(
                    "SELECT draft_kind, source_label, target_label, evidence_text, payload_json, status
                     FROM structured_draft_items
                     WHERE id = ?1 AND project_id = ?2 AND chapter_id = ?3
                     LIMIT 1",
                    params![item_id, &project_id, chapter_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(|err| {
                    AppErrorDto::new("DB_QUERY_FAILED", "查询草案池失败", true)
                        .with_detail(err.to_string())
                })?
                .ok_or_else(|| AppErrorDto::new("DRAFT_ITEM_NOT_FOUND", "草案项不存在", true))?;

            if item.5 != "pending" {
                return Err(AppErrorDto::new(
                    "DRAFT_ITEM_ALREADY_PROCESSED",
                    "草案项已处理",
                    true,
                ));
            }

            let item_kind = item.0.trim().to_ascii_lowercase();
            if !draft_kind.is_empty() && draft_kind != item_kind {
                return Err(AppErrorDto::new(
                    "DRAFT_ITEM_KIND_MISMATCH",
                    "草案类型不匹配",
                    true,
                ));
            }
            draft_kind = item_kind;
            if source_label.is_empty() {
                source_label = item.1.trim().to_string();
            }
            if target_label.is_none() {
                target_label = item
                    .2
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string);
            }
            if evidence.is_empty() {
                evidence = item.3.unwrap_or_default().trim().to_string();
            }
            let payload: serde_json::Value =
                serde_json::from_str(&item.4).unwrap_or_else(|_| serde_json::Value::Null);
            if relationship_type.is_none() {
                relationship_type =
                    payload_lookup_string(&payload, &["relationshipType", "relationship_type"]);
            }
            if involvement_type.is_none() {
                involvement_type =
                    payload_lookup_string(&payload, &["involvementType", "involvement_type"]);
            }
            if scene_type.is_none() {
                scene_type = payload_lookup_string(&payload, &["sceneType", "scene_type"]);
            }
        }

        if draft_kind.is_empty() || source_label.is_empty() {
            return Err(AppErrorDto::new("DRAFT_INVALID", "草案内容为空", true));
        }

        let result = match draft_kind.as_str() {
            "relationship" => {
                let target_label = target_label.clone().unwrap_or_default();
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
                let relationship_type = relationship_type
                    .clone()
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
                let (relation_id, action) = if let Some(existing_id) = existing_relation_id {
                    (existing_id, "reused".to_string())
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
                    (relation_id, "created".to_string())
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
                    draft_item_id: draft_item_id.clone(),
                    draft_item_status: Some("applied".to_string()),
                    primary_target_id: relation_id.clone(),
                    secondary_target_id: Some(target_id),
                }
            }
            "involvement" => {
                let involvement_type = involvement_type
                    .clone()
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
                    draft_item_id: draft_item_id.clone(),
                    draft_item_status: Some("applied".to_string()),
                    primary_target_id: character_id,
                    secondary_target_id: None,
                }
            }
            "scene" => {
                let scene_type = scene_type
                    .clone()
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
                    draft_item_id: draft_item_id.clone(),
                    draft_item_status: Some("applied".to_string()),
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

        if let Some(ref item_id) = draft_item_id {
            let (target_type, target_field) = match result.draft_kind.as_str() {
                "relationship" => (
                    "character_relationship",
                    relationship_type.as_deref().unwrap_or("互动"),
                ),
                "involvement" => (
                    "character",
                    involvement_type.as_deref().unwrap_or("一般戏份"),
                ),
                "scene" => ("world_rule", scene_type.as_deref().unwrap_or("地点场景")),
                _ => ("unknown", ""),
            };
            let now = now_iso();
            tx.execute(
                "UPDATE structured_draft_items
                 SET status = 'applied',
                     applied_target_type = ?1,
                     applied_target_id = ?2,
                     applied_target_field = ?3,
                     applied_at = ?4,
                     updated_at = ?4
                 WHERE id = ?5",
                params![
                    target_type,
                    &result.primary_target_id,
                    target_field,
                    &now,
                    item_id
                ],
            )
            .map_err(|err| {
                AppErrorDto::new("DB_WRITE_FAILED", "回写草案项状态失败", true)
                    .with_detail(err.to_string())
            })?;
            let batch_id = tx
                .query_row(
                    "SELECT batch_id FROM structured_draft_items WHERE id = ?1",
                    params![item_id],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|err| {
                    AppErrorDto::new("DB_QUERY_FAILED", "查询草案批次失败", true)
                        .with_detail(err.to_string())
                })?;
            let pending_count: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM structured_draft_items WHERE batch_id = ?1 AND status = 'pending'",
                    params![&batch_id],
                    |row| row.get(0),
                )
                .map_err(|err| {
                    AppErrorDto::new("DB_QUERY_FAILED", "查询批次状态失败", true)
                        .with_detail(err.to_string())
                })?;
            tx.execute(
                "UPDATE structured_draft_batches
                 SET status = ?1,
                     updated_at = ?2
                 WHERE id = ?3",
                params![
                    if pending_count == 0 {
                        "applied"
                    } else {
                        "pending"
                    },
                    &now,
                    &batch_id
                ],
            )
            .map_err(|err| {
                AppErrorDto::new("DB_WRITE_FAILED", "回写草案批次失败", true)
                    .with_detail(err.to_string())
            })?;
        }

        tx.commit().map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "保存结构化草案失败", true)
                .with_detail(err.to_string())
        })?;
        Ok(result)
    }

    fn persist_structured_draft_pool(
        &self,
        conn: &mut rusqlite::Connection,
        project_id: &str,
        chapter_id: &str,
        source_task_type: &str,
        chapter_content: &str,
        relationship_drafts: &[ExtractedRelationshipDraft],
        involvement_drafts: &[ExtractedInvolvementDraft],
        scene_drafts: &[ExtractedSceneDraft],
    ) -> Result<(), AppErrorDto> {
        #[derive(Debug)]
        struct DraftRow {
            draft_kind: &'static str,
            source_label: String,
            target_label: Option<String>,
            normalized_key: String,
            confidence: f32,
            occurrences: i64,
            evidence_text: String,
            payload_json: String,
        }

        let mut rows: Vec<DraftRow> = Vec::new();
        for draft in relationship_drafts {
            rows.push(DraftRow {
                draft_kind: "relationship",
                source_label: draft.source_label.clone(),
                target_label: Some(draft.target_label.clone()),
                normalized_key: normalized_relationship_key(
                    &draft.source_label,
                    &draft.target_label,
                    &draft.relationship_type,
                ),
                confidence: draft.confidence,
                occurrences: 1,
                evidence_text: draft.evidence.clone(),
                payload_json: serde_json::json!({
                    "relationshipType": draft.relationship_type
                })
                .to_string(),
            });
        }
        for draft in involvement_drafts {
            rows.push(DraftRow {
                draft_kind: "involvement",
                source_label: draft.character_label.clone(),
                target_label: None,
                normalized_key: normalized_involvement_key(
                    chapter_id,
                    &draft.character_label,
                    &draft.involvement_type,
                ),
                confidence: draft.confidence,
                occurrences: draft.occurrences.max(1),
                evidence_text: draft.evidence.clone(),
                payload_json: serde_json::json!({
                    "involvementType": draft.involvement_type
                })
                .to_string(),
            });
        }
        for draft in scene_drafts {
            rows.push(DraftRow {
                draft_kind: "scene",
                source_label: draft.scene_label.clone(),
                target_label: None,
                normalized_key: normalized_scene_key(&draft.scene_label, &draft.scene_type),
                confidence: draft.confidence,
                occurrences: 1,
                evidence_text: draft.evidence.clone(),
                payload_json: serde_json::json!({
                    "sceneType": draft.scene_type
                })
                .to_string(),
            });
        }
        if rows.is_empty() {
            return Ok(());
        }

        let content_hash = content_hash(chapter_content);
        let existing_batch = conn
            .query_row(
                "SELECT id, run_id
                 FROM structured_draft_batches
                 WHERE project_id = ?1
                   AND chapter_id = ?2
                   AND content_hash = ?3
                   AND status = 'pending'
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![project_id, chapter_id, &content_hash],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询草案批次失败", true)
                    .with_detail(err.to_string())
            })?;

        let now = now_iso();
        let (run_id, batch_id) = if let Some((existing_batch_id, existing_run_id)) = existing_batch
        {
            (existing_run_id, existing_batch_id)
        } else {
            let run_id = Uuid::new_v4().to_string();
            let batch_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO ai_pipeline_runs(
                    id, project_id, chapter_id, task_type, ui_action, status, phase, duration_ms, created_at, completed_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'succeeded', 'persist', 0, ?6, ?6)",
                params![
                    &run_id,
                    project_id,
                    chapter_id,
                    source_task_type,
                    "editor.context.extract",
                    &now
                ],
            )
            .map_err(|err| {
                AppErrorDto::new("DB_WRITE_FAILED", "记录草案运行失败", true)
                    .with_detail(err.to_string())
            })?;
            conn.execute(
                "INSERT INTO structured_draft_batches(
                    id, run_id, project_id, chapter_id, source_task_type, content_hash, status, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?7)",
                params![
                    &batch_id,
                    &run_id,
                    project_id,
                    chapter_id,
                    source_task_type,
                    &content_hash,
                    &now
                ],
            )
            .map_err(|err| {
                AppErrorDto::new("DB_WRITE_FAILED", "创建草案批次失败", true)
                    .with_detail(err.to_string())
            })?;
            (run_id, batch_id)
        };

        let tx = conn.transaction().map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "无法写入草案池", true).with_detail(err.to_string())
        })?;
        for row in rows {
            let existing_pending = tx
                .query_row(
                    "SELECT id, occurrences, confidence, evidence_text
                     FROM structured_draft_items
                     WHERE project_id = ?1
                       AND draft_kind = ?2
                       AND normalized_key = ?3
                       AND status = 'pending'
                     LIMIT 1",
                    params![project_id, row.draft_kind, &row.normalized_key],
                    |db_row| {
                        Ok((
                            db_row.get::<_, String>(0)?,
                            db_row.get::<_, i64>(1)?,
                            db_row.get::<_, Option<f64>>(2)?,
                            db_row.get::<_, Option<String>>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(|err| {
                    AppErrorDto::new("DB_QUERY_FAILED", "查询草案项失败", true)
                        .with_detail(err.to_string())
                })?;

            if let Some((
                existing_id,
                existing_occurrences,
                existing_confidence,
                existing_evidence,
            )) = existing_pending
            {
                let merged_evidence =
                    merge_draft_evidence(existing_evidence.as_deref(), &row.evidence_text);
                tx.execute(
                    "UPDATE structured_draft_items
                     SET batch_id = ?1,
                         run_id = ?2,
                         chapter_id = ?3,
                         target_label = ?4,
                         confidence = ?5,
                         occurrences = ?6,
                         evidence_text = ?7,
                         payload_json = ?8,
                         updated_at = ?9
                     WHERE id = ?10",
                    params![
                        &batch_id,
                        &run_id,
                        chapter_id,
                        row.target_label.as_deref(),
                        (existing_confidence
                            .unwrap_or(0.0_f64)
                            .max(row.confidence as f64)),
                        existing_occurrences.max(row.occurrences),
                        merged_evidence,
                        &row.payload_json,
                        &now,
                        &existing_id
                    ],
                )
                .map_err(|err| {
                    AppErrorDto::new("DB_WRITE_FAILED", "更新草案项失败", true)
                        .with_detail(err.to_string())
                })?;
                continue;
            }

            tx.execute(
                "INSERT INTO structured_draft_items(
                    id, batch_id, run_id, project_id, chapter_id, draft_kind, source_label, target_label,
                    normalized_key, confidence, occurrences, evidence_text, payload_json, status, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'pending', ?14, ?14)",
                params![
                    Uuid::new_v4().to_string(),
                    &batch_id,
                    &run_id,
                    project_id,
                    chapter_id,
                    row.draft_kind,
                    &row.source_label,
                    row.target_label.as_deref(),
                    &row.normalized_key,
                    row.confidence as f64,
                    row.occurrences,
                    &row.evidence_text,
                    &row.payload_json,
                    &now
                ],
            )
            .map_err(|err| {
                AppErrorDto::new("DB_WRITE_FAILED", "写入草案项失败", true)
                    .with_detail(err.to_string())
            })?;
        }

        tx.execute(
            "UPDATE structured_draft_batches
             SET updated_at = ?1
             WHERE id = ?2",
            params![&now, &batch_id],
        )
        .map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "更新草案批次失败", true)
                .with_detail(err.to_string())
        })?;
        tx.commit().map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "保存草案池失败", true).with_detail(err.to_string())
        })?;
        Ok(())
    }

    fn load_structured_draft_pool(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
        chapter_id: &str,
    ) -> Result<
        (
            Vec<RelationshipDraft>,
            Vec<InvolvementDraft>,
            Vec<SceneDraft>,
        ),
        AppErrorDto,
    > {
        let mut relationship_drafts: Vec<RelationshipDraft> = Vec::new();
        let mut involvement_drafts: Vec<InvolvementDraft> = Vec::new();
        let mut scene_drafts: Vec<SceneDraft> = Vec::new();

        let mut stmt = conn
            .prepare(
                "SELECT id, batch_id, draft_kind, source_label, target_label, confidence, occurrences, evidence_text, payload_json, status
                 FROM structured_draft_items
                 WHERE project_id = ?1 AND chapter_id = ?2 AND draft_kind IN ('relationship', 'involvement', 'scene')
                 ORDER BY CASE status WHEN 'pending' THEN 0 ELSE 1 END, updated_at DESC",
            )
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询草案池失败", true).with_detail(err.to_string())
            })?;

        let rows = stmt
            .query_map(params![project_id, chapter_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<f64>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "查询草案池失败", true)
                    .with_detail(err.to_string())
            })?;

        for row in rows {
            let (
                id,
                batch_id,
                draft_kind,
                source_label,
                target_label,
                confidence,
                occurrences,
                evidence_text,
                payload_json,
                status,
            ) = row.map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "解析草案池失败", true)
                    .with_detail(err.to_string())
            })?;
            let payload: serde_json::Value =
                serde_json::from_str(&payload_json).unwrap_or_else(|_| serde_json::Value::Null);
            let confidence = confidence.unwrap_or(0.0) as f32;
            let evidence = evidence_text.unwrap_or_default();

            match draft_kind.as_str() {
                "relationship" => {
                    relationship_drafts.push(RelationshipDraft {
                        id,
                        batch_id,
                        status,
                        source_label,
                        target_label: target_label.unwrap_or_default(),
                        relationship_type: payload_lookup_string(
                            &payload,
                            &["relationshipType", "relationship_type"],
                        )
                        .unwrap_or_else(|| "互动".to_string()),
                        confidence,
                        evidence,
                    });
                }
                "involvement" => {
                    involvement_drafts.push(InvolvementDraft {
                        id,
                        batch_id,
                        status,
                        character_label: source_label,
                        involvement_type: payload_lookup_string(
                            &payload,
                            &["involvementType", "involvement_type"],
                        )
                        .unwrap_or_else(|| "一般戏份".to_string()),
                        occurrences: occurrences.max(1),
                        confidence,
                        evidence,
                    });
                }
                "scene" => {
                    scene_drafts.push(SceneDraft {
                        id,
                        batch_id,
                        status,
                        scene_label: source_label,
                        scene_type: payload_lookup_string(&payload, &["sceneType", "scene_type"])
                            .unwrap_or_else(|| "地点场景".to_string()),
                        confidence,
                        evidence,
                    });
                }
                _ => {}
            }
        }

        Ok((relationship_drafts, involvement_drafts, scene_drafts))
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

        // Linked assets first, then project-level fallback assets.
        // This avoids context starvation when a newly created asset has not been linked yet.
        let characters: Vec<CharacterSummary> = conn
            .prepare(
                r#"
                SELECT c.id, c.name, c.role_type, c.aliases, c.motivation, c.desire,
                       c.fear, c.flaw, c.arc_stage, c.identity_text, c.appearance, c.locked_fields
                FROM characters c
                LEFT JOIN chapter_links cl
                  ON cl.target_id = c.id
                 AND cl.target_type = 'character'
                 AND cl.chapter_id = ?1
                WHERE c.project_id = ?2 AND c.is_deleted = 0
                ORDER BY
                  CASE WHEN cl.chapter_id IS NULL THEN 1 ELSE 0 END,
                  c.updated_at DESC,
                  c.created_at DESC
                LIMIT 24
                "#,
            )
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询角色失败", true))?
            .query_map(params![chapter_id, project_id], |row| {
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
                LEFT JOIN chapter_links cl
                  ON cl.target_id = w.id
                 AND cl.target_type = 'world_rule'
                 AND cl.chapter_id = ?1
                WHERE w.project_id = ?2 AND w.is_deleted = 0
                ORDER BY
                  CASE WHEN cl.chapter_id IS NULL THEN 1 ELSE 0 END,
                  w.updated_at DESC,
                  w.created_at DESC
                LIMIT 24
                "#,
            )
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询世界规则失败", true))?
            .query_map(params![chapter_id, project_id], |row| {
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
                LEFT JOIN chapter_links cl
                  ON cl.target_id = p.id
                 AND cl.target_type = 'plot_node'
                 AND cl.chapter_id = ?1
                WHERE p.project_id = ?2
                ORDER BY
                  CASE WHEN cl.chapter_id IS NULL THEN 1 ELSE 0 END,
                  p.sort_order ASC,
                  p.updated_at DESC
                LIMIT 24
                "#,
            )
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询主线节点失败", true))?
            .query_map(params![chapter_id, project_id], |row| {
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

fn normalized_relationship_key(
    source_label: &str,
    target_label: &str,
    relationship_type: &str,
) -> String {
    let mut pair = [
        normalize_label_key(source_label),
        normalize_label_key(target_label),
    ];
    pair.sort();
    format!(
        "rel:{}|{}|{}",
        pair[0],
        pair[1],
        normalize_label_key(relationship_type)
    )
}

fn normalized_involvement_key(
    chapter_id: &str,
    character_label: &str,
    involvement_type: &str,
) -> String {
    format!(
        "inv:{}|{}|{}",
        normalize_label_key(chapter_id),
        normalize_label_key(character_label),
        normalize_label_key(involvement_type)
    )
}

fn normalized_scene_key(scene_label: &str, scene_type: &str) -> String {
    format!(
        "scene:{}|{}",
        normalize_label_key(scene_label),
        normalize_label_key(scene_type)
    )
}

fn content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn merge_draft_evidence(existing: Option<&str>, incoming: &str) -> String {
    let existing = existing.unwrap_or_default().trim();
    let incoming = incoming.trim();
    if existing.is_empty() {
        return incoming.to_string();
    }
    if incoming.is_empty() || existing.contains(incoming) {
        return existing.to_string();
    }
    format!("{}\n{}", existing, incoming)
}

fn payload_lookup_string(payload: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = payload.get(*key).and_then(|item| item.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
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
) -> Vec<ExtractedRelationshipDraft> {
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
                drafts.push(ExtractedRelationshipDraft {
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
) -> Vec<ExtractedInvolvementDraft> {
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
        drafts.push(ExtractedInvolvementDraft {
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
) -> Vec<ExtractedSceneDraft> {
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
        drafts.push(ExtractedSceneDraft {
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
                    draft_item_id: None,
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
                    draft_item_id: None,
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

    #[test]
    fn collect_context_persists_structured_draft_pool_and_apply_updates_item_status() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let context_service = ContextService;

        let project = project_service
            .create_project(CreateProjectInput {
                name: "草案池闭环测试".to_string(),
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

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let content_path: String = conn
            .query_row(
                "SELECT content_path FROM chapters WHERE id = ?1",
                params![&chapter.id],
                |row| row.get(0),
            )
            .expect("query content path");
        drop(conn);

        let chapter_file = std::path::Path::new(&project.project_root).join(content_path);
        fs::write(
            &chapter_file,
            "林夜与李伯并肩迎敌。林夜提醒李伯小心。林夜回望青石镇，青石镇夜色沉沉。",
        )
        .expect("write chapter content");

        let panel = context_service
            .collect_editor_context(&project.project_root, &chapter.id)
            .expect("collect context");
        assert!(!panel.relationship_drafts.is_empty());
        let relationship = panel
            .relationship_drafts
            .iter()
            .find(|item| item.status == "pending")
            .expect("pending relationship draft")
            .clone();

        let applied = context_service
            .apply_structured_draft(
                &project.project_root,
                &chapter.id,
                ApplyStructuredDraftInput {
                    draft_item_id: Some(relationship.id.clone()),
                    draft_kind: "relationship".to_string(),
                    source_label: relationship.source_label.clone(),
                    target_label: Some(relationship.target_label.clone()),
                    relationship_type: Some(relationship.relationship_type.clone()),
                    involvement_type: None,
                    scene_type: None,
                    evidence: Some(relationship.evidence.clone()),
                },
            )
            .expect("apply structured item");
        assert_eq!(applied.action, "created");
        assert_eq!(
            applied.draft_item_id.as_deref(),
            Some(relationship.id.as_str())
        );

        let conn = open_database(std::path::Path::new(&project.project_root)).expect("open db");
        let (status, target_id): (String, Option<String>) = conn
            .query_row(
                "SELECT status, applied_target_id FROM structured_draft_items WHERE id = ?1",
                params![&relationship.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("query applied item");
        assert_eq!(status, "applied");
        assert!(target_id.is_some());

        remove_temp_workspace(&workspace);
    }
}
