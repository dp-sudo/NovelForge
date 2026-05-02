use serde::Serialize;
use serde_json::Value;

use crate::errors::AppErrorDto;
use crate::services::blueprint_service::{BlueprintService, BlueprintStep};
use crate::services::chapter_service::{ChapterRecord, ChapterService, VolumeRecord, VolumeService};
use crate::services::character_service::{CharacterRecord, CharacterService};
use crate::services::consistency_service::{ConsistencyIssue, ConsistencyService};
use crate::services::context_service::{ContextService, EditorContextPanel};
use crate::services::dashboard_service::{DashboardService, DashboardStats};
use crate::services::feedback_service::{FeedbackEventRecord, FeedbackService};
use crate::services::glossary_service::{GlossaryService, GlossaryTermRecord};
use crate::services::narrative_service::{NarrativeObligation, NarrativeService};
use crate::services::plot_service::{PlotNodeRecord, PlotService};
use crate::services::project_service::{AiStrategyProfile, ProjectService};
use crate::services::world_service::{WorldRuleRecord, WorldService};

const CHAPTER_BLUEPRINT_STEP_KEY: &str = "step-08-chapters";
const PREVIEW_LIMIT: usize = 5;
const REVIEW_LIST_LIMIT: usize = 12;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCenterStats {
    pub total_words: i64,
    pub chapter_count: i64,
    pub character_count: i64,
    pub world_rule_count: i64,
    pub plot_node_count: i64,
    pub open_issue_count: i64,
    pub completed_chapter_count: i64,
    pub completed_blueprint_count: i64,
    pub total_blueprint_steps: i64,
    pub blueprint_progress: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCenterConstitution {
    pub blueprint_steps: Vec<BlueprintStep>,
    pub obligations: Vec<NarrativeObligation>,
    pub locked_terms: Vec<GlossaryTermRecord>,
    pub banned_terms: Vec<GlossaryTermRecord>,
    pub strong_rules: Vec<WorldRuleRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCenterWindowPlanning {
    pub volume_structure: String,
    pub chapter_goals: Vec<String>,
    pub current_volume_progress: i64,
    pub planned_chapter_count: i64,
    pub window_planning_horizon: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCenterProductionQueue {
    pub active_chapter_id: Option<String>,
    pub chapters: Vec<ChapterRecord>,
    pub volumes: Vec<VolumeRecord>,
    pub window_planning: CommandCenterWindowPlanning,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCenterReviewQueue {
    pub feedback_events: Vec<FeedbackEventRecord>,
    pub consistency_issues: Vec<ConsistencyIssue>,
    pub drift_warnings: Vec<String>,
    pub open_feedback_count: usize,
    pub acknowledged_feedback_count: usize,
    pub open_issue_count: usize,
    pub high_severity_issue_count: usize,
    pub state_update_count: usize,
    pub asset_promotion_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCenterAssetAuthority {
    pub character_count: usize,
    pub world_rule_count: usize,
    pub glossary_count: usize,
    pub plot_node_count: usize,
    pub preview_characters: Vec<CharacterRecord>,
    pub preview_world_rules: Vec<WorldRuleRecord>,
    pub preview_glossary: Vec<GlossaryTermRecord>,
    pub preview_plot_nodes: Vec<PlotNodeRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCenterWorkspace {
    pub chapter_id: String,
    pub chapter_index: i64,
    pub chapter_title: String,
    pub chapter_status: String,
    pub target_words: i64,
    pub current_words: i64,
    pub context: EditorContextPanel,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCenterSnapshot {
    pub stats: CommandCenterStats,
    pub constitution: CommandCenterConstitution,
    pub production_queue: CommandCenterProductionQueue,
    pub review_queue: CommandCenterReviewQueue,
    pub asset_authority: CommandCenterAssetAuthority,
    pub workspace: Option<CommandCenterWorkspace>,
}

#[derive(Default)]
pub struct CommandCenterService;

impl CommandCenterService {
    pub fn get_snapshot(
        &self,
        project_root: &str,
        preferred_chapter_id: Option<&str>,
    ) -> Result<CommandCenterSnapshot, AppErrorDto> {
        let dashboard_stats = DashboardService.get_stats(project_root)?;
        let blueprint_steps = BlueprintService.list_steps(project_root)?;
        let obligations = NarrativeService.list(project_root)?;
        let glossary_terms = GlossaryService.list(project_root)?;
        let world_rules = WorldService.list(project_root)?;
        let plot_nodes = PlotService.list(project_root)?;
        let chapters = ChapterService.list_chapters(project_root)?;
        let volumes = VolumeService.list(project_root)?;
        let characters = CharacterService.list(project_root)?;
        let consistency_issues = ConsistencyService.list_issues(project_root)?;
        let feedback_events = FeedbackService.get_feedback_events(project_root)?;
        let strategy = ProjectService.get_ai_strategy_profile(project_root)?;

        let active_chapter = select_active_chapter(&chapters, preferred_chapter_id);
        let workspace = match active_chapter.as_ref() {
            Some(chapter) => Some(self.build_workspace(project_root, chapter)?),
            None => None,
        };

        let window_planning = build_window_planning(
            blueprint_steps.iter().find(|step| step.step_key == CHAPTER_BLUEPRINT_STEP_KEY),
            &chapters,
            &strategy,
        );
        let stats = build_command_center_stats(&dashboard_stats, &chapters);
        let constitution = build_constitution(
            blueprint_steps,
            obligations,
            glossary_terms.as_slice(),
            world_rules.as_slice(),
        );
        let asset_authority =
            build_asset_authority(characters, world_rules, glossary_terms, plot_nodes);
        let review_queue = build_review_queue(
            feedback_events,
            consistency_issues,
            &chapters,
            window_planning.planned_chapter_count,
            workspace.as_ref(),
        );
        let next_actions = build_next_actions(active_chapter.as_ref(), &review_queue);

        Ok(CommandCenterSnapshot {
            stats,
            constitution,
            production_queue: CommandCenterProductionQueue {
                active_chapter_id: active_chapter.map(|chapter| chapter.id),
                chapters,
                volumes,
                window_planning,
                next_actions,
            },
            review_queue,
            asset_authority,
            workspace,
        })
    }

    fn build_workspace(
        &self,
        project_root: &str,
        chapter: &ChapterRecord,
    ) -> Result<CommandCenterWorkspace, AppErrorDto> {
        let context = ContextService.collect_editor_context(project_root, &chapter.id)?;
        Ok(CommandCenterWorkspace {
            chapter_id: chapter.id.clone(),
            chapter_index: chapter.chapter_index,
            chapter_title: chapter.title.clone(),
            chapter_status: chapter.status.clone(),
            target_words: chapter.target_words,
            current_words: chapter.current_words,
            context,
        })
    }
}

fn build_command_center_stats(
    dashboard_stats: &DashboardStats,
    chapters: &[ChapterRecord],
) -> CommandCenterStats {
    let total_steps = if dashboard_stats.total_blueprint_steps > 0 {
        dashboard_stats.total_blueprint_steps
    } else {
        8
    };
    let blueprint_progress =
        ((dashboard_stats.completed_blueprint_count * 100) / total_steps).clamp(0, 100);
    let completed_chapter_count = chapters
        .iter()
        .filter(|chapter| chapter.status == "completed")
        .count() as i64;

    CommandCenterStats {
        total_words: dashboard_stats.total_words,
        chapter_count: dashboard_stats.chapter_count,
        character_count: dashboard_stats.character_count,
        world_rule_count: dashboard_stats.world_rule_count,
        plot_node_count: dashboard_stats.plot_node_count,
        open_issue_count: dashboard_stats.open_issue_count,
        completed_chapter_count,
        completed_blueprint_count: dashboard_stats.completed_blueprint_count,
        total_blueprint_steps: total_steps,
        blueprint_progress,
    }
}

fn build_constitution(
    blueprint_steps: Vec<BlueprintStep>,
    obligations: Vec<NarrativeObligation>,
    glossary_terms: &[GlossaryTermRecord],
    world_rules: &[WorldRuleRecord],
) -> CommandCenterConstitution {
    CommandCenterConstitution {
        blueprint_steps,
        obligations,
        locked_terms: glossary_terms
            .iter()
            .filter(|item| item.locked)
            .cloned()
            .collect(),
        banned_terms: glossary_terms
            .iter()
            .filter(|item| item.banned)
            .cloned()
            .collect(),
        strong_rules: world_rules
            .iter()
            .filter(|item| item.constraint_level == "strong" || item.constraint_level == "absolute")
            .cloned()
            .collect(),
    }
}

fn build_asset_authority(
    characters: Vec<CharacterRecord>,
    world_rules: Vec<WorldRuleRecord>,
    glossary_terms: Vec<GlossaryTermRecord>,
    plot_nodes: Vec<PlotNodeRecord>,
) -> CommandCenterAssetAuthority {
    CommandCenterAssetAuthority {
        character_count: characters.len(),
        world_rule_count: world_rules.len(),
        glossary_count: glossary_terms.len(),
        plot_node_count: plot_nodes.len(),
        preview_characters: characters.into_iter().take(PREVIEW_LIMIT).collect(),
        preview_world_rules: world_rules.into_iter().take(PREVIEW_LIMIT).collect(),
        preview_glossary: glossary_terms.into_iter().take(PREVIEW_LIMIT).collect(),
        preview_plot_nodes: plot_nodes.into_iter().take(PREVIEW_LIMIT).collect(),
    }
}

fn build_review_queue(
    feedback_events: Vec<FeedbackEventRecord>,
    consistency_issues: Vec<ConsistencyIssue>,
    chapters: &[ChapterRecord],
    planned_chapter_count: i64,
    workspace: Option<&CommandCenterWorkspace>,
) -> CommandCenterReviewQueue {
    let open_feedback_count = feedback_events
        .iter()
        .filter(|event| event.status == "open")
        .count();
    let acknowledged_feedback_count = feedback_events
        .iter()
        .filter(|event| event.status == "acknowledged")
        .count();
    let open_issue_count = consistency_issues
        .iter()
        .filter(|issue| issue.status == "open")
        .count();
    let high_severity_issue_count = consistency_issues
        .iter()
        .filter(|issue| issue.severity == "high" || issue.severity == "blocker")
        .count();
    let state_update_count = workspace
        .map(|item| item.context.state_summary.len())
        .unwrap_or_default();
    let asset_promotion_count = workspace.map(count_promoted_assets).unwrap_or_default();

    CommandCenterReviewQueue {
        feedback_events: feedback_events
            .into_iter()
            .filter(|event| event.status == "open" || event.status == "acknowledged")
            .take(REVIEW_LIST_LIMIT)
            .collect(),
        consistency_issues: consistency_issues
            .into_iter()
            .filter(|issue| issue.status == "open")
            .take(REVIEW_LIST_LIMIT)
            .collect(),
        drift_warnings: collect_drift_warnings(chapters, planned_chapter_count),
        open_feedback_count,
        acknowledged_feedback_count,
        open_issue_count,
        high_severity_issue_count,
        state_update_count,
        asset_promotion_count,
    }
}

fn count_promoted_assets(workspace: &CommandCenterWorkspace) -> usize {
    workspace
        .context
        .characters
        .iter()
        .filter(|item| item.source_kind != "user_input")
        .count()
        + workspace
            .context
            .world_rules
            .iter()
            .filter(|item| item.source_kind != "user_input")
            .count()
        + workspace
            .context
            .plot_nodes
            .iter()
            .filter(|item| item.source_kind != "user_input")
            .count()
        + workspace
            .context
            .glossary
            .iter()
            .filter(|item| item.source_kind != "user_input")
            .count()
}

fn collect_drift_warnings(chapters: &[ChapterRecord], planned_chapter_count: i64) -> Vec<String> {
    let mut warnings = Vec::new();

    for chapter in chapters {
        if chapter.target_words <= 0 || chapter.current_words <= 0 {
            continue;
        }
        let delta = ((chapter.current_words - chapter.target_words).abs() as f64)
            / chapter.target_words as f64;
        if delta >= 0.35 {
            warnings.push(format!(
                "第 {} 章字数偏差 {}%（目标 {}，当前 {}）",
                chapter.chapter_index,
                (delta * 100.0).round() as i64,
                chapter.target_words,
                chapter.current_words
            ));
        }
    }

    if planned_chapter_count > 0 && chapters.len() as i64 > planned_chapter_count {
        warnings.push(format!(
            "实际章节数 {} 已超过蓝图计划 {}",
            chapters.len(),
            planned_chapter_count
        ));
    } else if planned_chapter_count > 0 && planned_chapter_count - chapters.len() as i64 >= 3 {
        warnings.push(format!(
            "当前章节数 {} 低于蓝图计划 {}，窗口执行存在滞后",
            chapters.len(),
            planned_chapter_count
        ));
    }

    warnings
}

fn build_next_actions(
    active_chapter: Option<&ChapterRecord>,
    review_queue: &CommandCenterReviewQueue,
) -> Vec<String> {
    let mut actions = Vec::new();

    if review_queue.high_severity_issue_count > 0 {
        actions.push("先处理高优先级风险与回报事件".to_string());
    } else if review_queue.open_issue_count > 0 || review_queue.open_feedback_count > 0 {
        actions.push("检查待处理风险，确认是否影响继续推进".to_string());
    }

    match active_chapter {
        Some(chapter) if chapter.status == "planned" => {
            actions.push(format!("为第 {} 章生成章节计划", chapter.chapter_index));
        }
        Some(chapter) if chapter.current_words == 0 => {
            actions.push(format!("开始起草第 {} 章正文", chapter.chapter_index));
        }
        Some(chapter) if chapter.status == "revising" => {
            actions.push(format!("继续精修第 {} 章并清理草案状态", chapter.chapter_index));
        }
        Some(chapter) => {
            actions.push(format!("继续推进第 {} 章正文生产", chapter.chapter_index));
        }
        None => actions.push("创建第一章并建立当前生产窗口".to_string()),
    }

    actions.push("必要时从资产抽屉补录或核对正式资产".to_string());
    actions
}

fn select_active_chapter(
    chapters: &[ChapterRecord],
    preferred_chapter_id: Option<&str>,
) -> Option<ChapterRecord> {
    if chapters.is_empty() {
        return None;
    }

    if let Some(preferred) = preferred_chapter_id.map(str::trim).filter(|value| !value.is_empty()) {
        if let Some(chapter) = chapters.iter().find(|chapter| chapter.id == preferred) {
            return Some(chapter.clone());
        }
    }

    let mut sorted = chapters.to_vec();
    sorted.sort_by_key(|chapter| chapter.chapter_index);

    sorted
        .iter()
        .find(|chapter| chapter.status == "drafting")
        .cloned()
        .or_else(|| {
            sorted
                .iter()
                .find(|chapter| chapter.status == "revising")
                .cloned()
        })
        .or_else(|| {
            sorted
                .iter()
                .find(|chapter| chapter.status == "planned")
                .cloned()
        })
        .or_else(|| {
            sorted
                .iter()
                .find(|chapter| chapter.status != "archived")
                .cloned()
        })
        .or_else(|| sorted.into_iter().next())
}

fn build_window_planning(
    chapter_step: Option<&BlueprintStep>,
    chapters: &[ChapterRecord],
    strategy: &AiStrategyProfile,
) -> CommandCenterWindowPlanning {
    let fields = extract_chapter_blueprint_fields(
        chapter_step.map(|step| step.content.as_str()).unwrap_or_default(),
    );
    let chapter_goals = split_text_list(fields.chapter_goals.as_deref().unwrap_or_default());
    let chapter_list = split_text_list(fields.chapter_list.as_deref().unwrap_or_default());
    let planned_chapter_count = if !chapter_list.is_empty() {
        chapter_list.len() as i64
    } else {
        chapter_goals.len() as i64
    };
    let completed_chapter_count = chapters
        .iter()
        .filter(|chapter| chapter.status == "completed")
        .count() as i64;
    let progress_base = if planned_chapter_count > 0 {
        planned_chapter_count
    } else {
        chapters.len() as i64
    };
    let current_volume_progress = if progress_base > 0 {
        ((completed_chapter_count * 100) / progress_base).clamp(0, 100)
    } else {
        0
    };
    let horizon = normalize_window_horizon(strategy.window_planning_horizon);
    let truncated_goals = chapter_goals.into_iter().take(horizon as usize).collect();

    CommandCenterWindowPlanning {
        volume_structure: fields.volume_structure.unwrap_or_default(),
        chapter_goals: truncated_goals,
        current_volume_progress,
        planned_chapter_count,
        window_planning_horizon: horizon,
    }
}

struct ChapterBlueprintFields {
    volume_structure: Option<String>,
    chapter_list: Option<String>,
    chapter_goals: Option<String>,
}

fn extract_chapter_blueprint_fields(content: &str) -> ChapterBlueprintFields {
    let parsed = serde_json::from_str::<Value>(content).ok();
    let mut fields = ChapterBlueprintFields {
        volume_structure: None,
        chapter_list: None,
        chapter_goals: None,
    };

    if let Some(value) = parsed.as_ref() {
        if let Some(object) = value.as_object() {
            fields.volume_structure = find_blueprint_field_text(object, &["volumeStructure", "卷结构"]);
            fields.chapter_list = find_blueprint_field_text(object, &["chapterList", "章节列表", "chapters"]);
            fields.chapter_goals = find_blueprint_field_text(object, &["chapterGoals", "章节目标"]);

            for nested_key in ["fields", "data", "payload", "content", "result"] {
                if let Some(nested_object) = object.get(nested_key).and_then(Value::as_object) {
                    if fields.volume_structure.is_none() {
                        fields.volume_structure =
                            find_blueprint_field_text(nested_object, &["volumeStructure", "卷结构"]);
                    }
                    if fields.chapter_list.is_none() {
                        fields.chapter_list = find_blueprint_field_text(
                            nested_object,
                            &["chapterList", "章节列表", "chapters"],
                        );
                    }
                    if fields.chapter_goals.is_none() {
                        fields.chapter_goals =
                            find_blueprint_field_text(nested_object, &["chapterGoals", "章节目标"]);
                    }
                }
            }
        }
    } else if !content.trim().is_empty() {
        fields.chapter_goals = Some(content.trim().to_string());
    }

    fields
}

fn find_blueprint_field_text(
    object: &serde_json::Map<String, Value>,
    aliases: &[&str],
) -> Option<String> {
    for alias in aliases {
        if let Some(value) = object.get(*alias).and_then(json_value_to_text) {
            return Some(value);
        }

        let normalized_alias = normalize_blueprint_key(alias);
        if let Some((_, value)) = object
            .iter()
            .find(|(key, _)| normalize_blueprint_key(key) == normalized_alias)
        {
            if let Some(text) = json_value_to_text(value) {
                return Some(text);
            }
        }
    }
    None
}

fn normalize_blueprint_key(raw: &str) -> String {
    raw.to_ascii_lowercase()
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-'))
        .collect()
}

fn json_value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Array(items) => {
            let joined = items
                .iter()
                .filter_map(json_value_to_text)
                .collect::<Vec<_>>()
                .join("；");
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        _ => None,
    }
}

fn split_text_list(raw: &str) -> Vec<String> {
    raw.split(['\n', ';', '；'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_window_horizon(raw: i64) -> i64 {
    raw.clamp(1, 50)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use super::CommandCenterService;
    use crate::services::blueprint_service::{BlueprintService, SaveBlueprintStepInput};
    use crate::services::chapter_service::{ChapterInput, ChapterService};
    use crate::services::glossary_service::{CreateGlossaryTermInput, GlossaryService};
    use crate::services::project_service::{CreateProjectInput, ProjectService};
    use crate::services::world_service::{CreateWorldRuleInput, WorldService};

    fn create_temp_workspace() -> PathBuf {
        let workspace = std::env::temp_dir()
            .join(format!("novelforge-command-center-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn snapshot_prefers_drafting_chapter_and_builds_window_plan() {
        let workspace = create_temp_workspace();
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: "指挥台聚合测试".into(),
                author: None,
                genre: "奇幻".into(),
                target_words: Some(120_000),
                save_directory: workspace.to_string_lossy().into_owned(),
            })
            .expect("create project");

        let chapter_one = ChapterService
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".into(),
                    summary: Some("开场".into()),
                    target_words: Some(1800),
                    status: Some("planned".into()),
                },
            )
            .expect("create chapter one");
        let chapter_two = ChapterService
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第二章".into(),
                    summary: Some("推进".into()),
                    target_words: Some(2200),
                    status: Some("drafting".into()),
                },
            )
            .expect("create chapter two");

        BlueprintService
            .save_step(
                &project.project_root,
                SaveBlueprintStepInput {
                    step_key: "step-08-chapters".into(),
                    content: serde_json::json!({
                        "volumeStructure": "第一卷：序章到失控",
                        "chapterList": "第一章：开场\n第二章：推进",
                        "chapterGoals": "建立世界\n引入主角\n扩大冲突",
                    })
                    .to_string(),
                    ai_generated: Some(false),
                    certainty_zones: None,
                },
            )
            .expect("save blueprint step");

        let snapshot = CommandCenterService
            .get_snapshot(&project.project_root, None)
            .expect("get command center snapshot");

        assert_eq!(
            snapshot.production_queue.active_chapter_id.as_deref(),
            Some(chapter_two.id.as_str())
        );
        assert_eq!(
            snapshot.workspace.as_ref().map(|item| item.chapter_id.as_str()),
            Some(chapter_two.id.as_str())
        );
        assert_eq!(snapshot.production_queue.chapters.len(), 2);
        assert_eq!(
            snapshot.production_queue.window_planning.planned_chapter_count,
            2
        );
        assert_eq!(
            snapshot.production_queue.window_planning.chapter_goals.len(),
            3
        );
        assert_eq!(
            snapshot.production_queue.window_planning.volume_structure,
            "第一卷：序章到失控"
        );
        assert!(
            snapshot
                .production_queue
                .next_actions
                .iter()
                .any(|item| item.contains("第 2 章") || item.contains("第 2"))
        );
        assert_eq!(chapter_one.status, "planned");

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn snapshot_collects_locked_terms_and_strong_rules() {
        let workspace = create_temp_workspace();
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: "宪法聚合测试".into(),
                author: None,
                genre: "科幻".into(),
                target_words: Some(80_000),
                save_directory: workspace.to_string_lossy().into_owned(),
            })
            .expect("create project");

        GlossaryService
            .create(
                &project.project_root,
                CreateGlossaryTermInput {
                    term: "火种协议".into(),
                    term_type: "术语".into(),
                    aliases: None,
                    description: Some("不可变更的协议术语".into()),
                    locked: Some(true),
                    banned: Some(false),
                },
            )
            .expect("create locked term");
        WorldService
            .create(
                &project.project_root,
                CreateWorldRuleInput {
                    title: "跃迁需要燃料".into(),
                    category: "世界规则".into(),
                    description: "任何远距离跃迁都必须消耗燃料".into(),
                    constraint_level: "strong".into(),
                    related_entities: None,
                    examples: None,
                    contradiction_policy: None,
                },
            )
            .expect("create strong world rule");

        let snapshot = CommandCenterService
            .get_snapshot(&project.project_root, None)
            .expect("get command center snapshot");

        assert_eq!(snapshot.constitution.locked_terms.len(), 1);
        assert_eq!(snapshot.constitution.strong_rules.len(), 1);
        assert_eq!(snapshot.constitution.locked_terms[0].term, "火种协议");
        assert_eq!(snapshot.constitution.strong_rules[0].title, "跃迁需要燃料");

        remove_temp_workspace(&workspace);
    }
}
