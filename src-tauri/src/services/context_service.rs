use std::fs;
use std::path::Path;

use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::services::import_service::{extract_asset_candidates, AssetExtractionCandidate};
use crate::services::project_service::get_project_id;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalContext {
    pub project_name: String,
    pub genre: String,
    pub narrative_pov: Option<String>,
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
    pub previous_chapter_summary: Option<String>,
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

    fn collect_global_context(
        &self,
        conn: &rusqlite::Connection,
        project_id: &str,
    ) -> Result<GlobalContext, AppErrorDto> {
        // Project info
        let project = conn
            .query_row(
                "SELECT name, genre, narrative_pov FROM projects WHERE id = ?1",
                params![project_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .map_err(|err| {
                AppErrorDto::new("PROJECT_NOT_FOUND", "项目不存在", false)
                    .with_detail(err.to_string())
            })?;

        // Locked & banned terms from glossary
        let locked_terms: Vec<String> = conn
            .prepare(
                "SELECT term FROM glossary_terms WHERE project_id = ?1 AND locked = 1",
            )
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询名词库失败", true))?
            .query_map(params![project_id], |row| row.get::<_, String>(0))
            .map_err(|_| AppErrorDto::new("DB_QUERY_FAILED", "查询名词库失败", true))?
            .filter_map(|r| r.ok())
            .collect();

        let banned_terms: Vec<String> = conn
            .prepare(
                "SELECT term FROM glossary_terms WHERE project_id = ?1 AND banned = 1",
            )
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

fn strip_frontmatter(content: &str) -> String {
    if !content.starts_with("---\n") {
        return content.to_string();
    }
    if let Some(offset) = content[4..].find("\n---\n") {
        return content[(offset + 9)..].trim().to_string();
    }
    content.to_string()
}
