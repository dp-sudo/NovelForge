use serde::Serialize;

use crate::services::context_service::{CollectedContext, ContextService, StoryStateSummary};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityPack {
    pub constitution_context: Vec<String>,
    pub canon_context: Vec<String>,
    pub lexicon_policy_context: Vec<String>,
    pub state_context: Vec<String>,
    pub promise_context: Vec<String>,
    pub window_plan_context: Vec<String>,
    pub recent_continuity_context: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContinuityPackDepth {
    Minimal,
    Standard,
    Deep,
}

impl ContinuityPackDepth {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "minimal" => Self::Minimal,
            "standard" => Self::Standard,
            "deep" => Self::Deep,
            _ => Self::Standard,
        }
    }
}

#[derive(Default)]
pub struct ContinuityPackCompiler;

impl ContinuityPackCompiler {
    pub fn compile(
        &self,
        project_root: &str,
        canonical_task: &str,
        depth: &str,
        context: &CollectedContext,
        context_service: &ContextService,
        chapter_id: Option<&str>,
    ) -> ContinuityPack {
        let depth = ContinuityPackDepth::parse(depth);
        let mut pack = ContinuityPack {
            lexicon_policy_context: build_lexicon_policy_context(context),
            ..ContinuityPack::default()
        };

        if depth == ContinuityPackDepth::Minimal {
            return pack;
        }

        pack.constitution_context = context_service.get_constitution_context(context);
        pack.canon_context = context_service.get_canon_context(context);

        pack.state_context = match context_service.get_state_summary(project_root) {
            Ok(items) => items.into_iter().map(format_state_line).collect(),
            Err(err) => {
                log::warn!(
                    "[CONTINUITY_PACK] state summary unavailable for task {}: {} {}",
                    canonical_task,
                    err.code,
                    err.message
                );
                Vec::new()
            }
        };

        pack.promise_context = match context_service.get_promise_context(project_root) {
            Ok(lines) => lines,
            Err(err) => {
                log::warn!(
                    "[CONTINUITY_PACK] promise context unavailable for task {}: {} {}",
                    canonical_task,
                    err.code,
                    err.message
                );
                Vec::new()
            }
        };

        if depth == ContinuityPackDepth::Deep {
            pack.window_plan_context =
                match context_service.get_window_plan(project_root, chapter_id, context) {
                    Ok(lines) => lines,
                    Err(err) => {
                        log::warn!(
                            "[CONTINUITY_PACK] window plan unavailable for task {}: {} {}",
                            canonical_task,
                            err.code,
                            err.message
                        );
                        Vec::new()
                    }
                };

            pack.recent_continuity_context =
                match context_service.get_recent_continuity(project_root, chapter_id) {
                    Ok(lines) => lines,
                    Err(err) => {
                        log::warn!(
                            "[CONTINUITY_PACK] recent continuity unavailable for task {}: {} {}",
                            canonical_task,
                            err.code,
                            err.message
                        );
                        Vec::new()
                    }
                };
        }

        pack
    }
}

fn build_lexicon_policy_context(context: &CollectedContext) -> Vec<String> {
    let global = &context.global_context;
    let mut lines = Vec::new();

    if !global.locked_terms.is_empty() {
        lines.push(format!("锁定术语: {}", global.locked_terms.join("、")));
    }
    if !global.banned_terms.is_empty() {
        lines.push(format!("禁用词: {}", global.banned_terms.join("、")));
    }
    if let Some(style) = &global.writing_style {
        lines.push(format!(
            "文风约束: 语言={}、描写密度={}、对话比例={}、句式节奏={}、氛围={}、心理深度={}",
            style.language_style,
            style.description_density,
            style.dialogue_ratio,
            style.sentence_rhythm,
            style.atmosphere,
            style.psychological_depth
        ));
    }

    lines
}

fn format_state_line(item: StoryStateSummary) -> String {
    let payload = preview_text(&json_value_to_inline(&item.payload), 180);
    format!(
        "{} / {} / {}: {}",
        item.subject_type, item.subject_id, item.state_kind, payload
    )
}

fn json_value_to_inline(value: &serde_json::Value) -> String {
    if value.is_null() {
        return String::new();
    }
    match value {
        serde_json::Value::String(raw) => raw.trim().to_string(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn preview_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    let chars = trimmed.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return trimmed.to_string();
    }
    format!("{}...", chars[..max_chars].iter().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::{ContextService, ContinuityPackCompiler};
    use crate::services::context_service::{
        BlueprintStepSummary, ChapterSummary, CollectedContext, GlobalContext, RelatedContext,
    };

    fn sample_context() -> CollectedContext {
        CollectedContext {
            global_context: GlobalContext {
                project_name: "长夜行舟".to_string(),
                genre: "玄幻".to_string(),
                narrative_pov: Some("third_limited".to_string()),
                writing_style: None,
                locked_terms: vec!["灵火".to_string()],
                banned_terms: vec!["然而".to_string()],
                blueprint_summary: vec![BlueprintStepSummary {
                    step_key: "step-03-premise".to_string(),
                    title: "故事母题".to_string(),
                    content: Some("少年误入禁地".to_string()),
                    status: "completed".to_string(),
                }],
            },
            related_context: RelatedContext {
                chapter: Some(ChapterSummary {
                    id: "ch-1".to_string(),
                    title: "第一章".to_string(),
                    summary: "主角初入宗门".to_string(),
                    status: "drafting".to_string(),
                    chapter_index: 1,
                    target_words: 2200,
                    current_words: 1200,
                }),
                characters: Vec::new(),
                world_rules: Vec::new(),
                plot_nodes: Vec::new(),
                relationship_edges: Vec::new(),
                previous_chapter_summary: Some("前章埋下宗门内斗线索".to_string()),
            },
        }
    }

    #[test]
    fn minimal_depth_only_emits_lexicon_context() {
        let compiler = ContinuityPackCompiler;
        let context = sample_context();
        let service = ContextService;

        let pack = compiler.compile(
            "",
            "chapter.draft",
            "minimal",
            &context,
            &service,
            Some("ch-1"),
        );
        assert!(!pack.lexicon_policy_context.is_empty());
        assert!(pack.constitution_context.is_empty());
        assert!(pack.canon_context.is_empty());
        assert!(pack.state_context.is_empty());
        assert!(pack.promise_context.is_empty());
        assert!(pack.window_plan_context.is_empty());
        assert!(pack.recent_continuity_context.is_empty());
    }
}
