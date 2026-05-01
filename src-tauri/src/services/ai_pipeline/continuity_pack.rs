use serde::Serialize;
use std::collections::HashSet;

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
        affects_layers: &[String],
    ) -> ContinuityPack {
        let depth = ContinuityPackDepth::parse(depth);
        let mut pack = ContinuityPack {
            lexicon_policy_context: build_lexicon_policy_context(context),
            ..ContinuityPack::default()
        };

        if depth == ContinuityPackDepth::Minimal {
            return apply_layer_focus(pack, affects_layers);
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

        apply_layer_focus(pack, affects_layers)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ContinuityLayer {
    Constitution,
    Canon,
    LexiconPolicy,
    State,
    Promise,
    WindowPlan,
    RecentContinuity,
}

fn parse_continuity_layer(raw: &str) -> Option<ContinuityLayer> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "constitution" => Some(ContinuityLayer::Constitution),
        "canon" => Some(ContinuityLayer::Canon),
        "lexicon" | "lexicon_policy" | "policy" => Some(ContinuityLayer::LexiconPolicy),
        "state" => Some(ContinuityLayer::State),
        "promise" => Some(ContinuityLayer::Promise),
        "window" | "window_plan" => Some(ContinuityLayer::WindowPlan),
        "recent" | "recent_continuity" => Some(ContinuityLayer::RecentContinuity),
        _ => None,
    }
}

fn parse_layer_focus(affects_layers: &[String]) -> HashSet<ContinuityLayer> {
    affects_layers
        .iter()
        .filter_map(|raw| parse_continuity_layer(raw))
        .collect()
}

fn apply_layer_focus(mut pack: ContinuityPack, affects_layers: &[String]) -> ContinuityPack {
    let focus = parse_layer_focus(affects_layers);
    if focus.is_empty() {
        return pack;
    }

    if !focus.contains(&ContinuityLayer::Canon) {
        pack.canon_context.clear();
    }
    if !focus.contains(&ContinuityLayer::State) {
        pack.state_context.clear();
    }
    if !focus.contains(&ContinuityLayer::Promise) {
        pack.promise_context.clear();
    }
    if !focus.contains(&ContinuityLayer::WindowPlan) {
        pack.window_plan_context.clear();
    }
    if !focus.contains(&ContinuityLayer::RecentContinuity) {
        pack.recent_continuity_context.clear();
    }

    // 编排收敛策略：无论技能 focus 指向哪里，都保留宪法层与术语/文风约束。
    // 这两层是全局稳定性护栏，不随局部技能切换而关闭。
    pack
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
            "文风约束: 文风={}、描写密度={}、对话比例={}、句式节奏={}、氛围={}、心理深度={}",
            display_language_style(&style.language_style),
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

fn display_language_style(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "plain" => "平实".to_string(),
        "balanced" => "适中".to_string(),
        "ornate" => "华丽".to_string(),
        "colloquial" => "口语化".to_string(),
        "" => "适中".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_layer_focus, ContextService, ContinuityPack, ContinuityPackCompiler};
    use crate::services::context_service::{
        BlueprintStepSummary, ChapterSummary, CollectedContext, GlobalContext, RelatedContext,
    };
    use crate::services::project_service::WritingStyle;

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
            &[],
        );
        assert!(!pack.lexicon_policy_context.is_empty());
        assert!(pack.constitution_context.is_empty());
        assert!(pack.canon_context.is_empty());
        assert!(pack.state_context.is_empty());
        assert!(pack.promise_context.is_empty());
        assert!(pack.window_plan_context.is_empty());
        assert!(pack.recent_continuity_context.is_empty());
    }

    #[test]
    fn minimal_depth_lexicon_context_uses_tone_label() {
        let compiler = ContinuityPackCompiler;
        let mut context = sample_context();
        context.global_context.writing_style = Some(WritingStyle {
            language_style: "colloquial".to_string(),
            description_density: 4,
            dialogue_ratio: 6,
            sentence_rhythm: "mixed".to_string(),
            atmosphere: "warm".to_string(),
            psychological_depth: 5,
        });
        let service = ContextService;

        let pack = compiler.compile(
            "",
            "chapter.draft",
            "minimal",
            &context,
            &service,
            Some("ch-1"),
            &[],
        );
        let style_line = pack
            .lexicon_policy_context
            .iter()
            .find(|line| line.starts_with("文风约束:"))
            .expect("style line exists");
        assert!(style_line.contains("文风=口语化"));
        assert!(!style_line.contains("语言=colloquial"));
    }

    #[test]
    fn layer_focus_keeps_guardrails_and_selected_layers() {
        let pack = ContinuityPack {
            constitution_context: vec!["const".to_string()],
            canon_context: vec!["canon".to_string()],
            lexicon_policy_context: vec!["lexicon".to_string()],
            state_context: vec!["state".to_string()],
            promise_context: vec!["promise".to_string()],
            window_plan_context: vec!["window".to_string()],
            recent_continuity_context: vec!["recent".to_string()],
        };

        let filtered = apply_layer_focus(pack, &["state".to_string(), "window_plan".to_string()]);

        assert_eq!(filtered.constitution_context, vec!["const".to_string()]);
        assert_eq!(filtered.lexicon_policy_context, vec!["lexicon".to_string()]);
        assert!(filtered.canon_context.is_empty());
        assert_eq!(filtered.state_context, vec!["state".to_string()]);
        assert!(filtered.promise_context.is_empty());
        assert_eq!(filtered.window_plan_context, vec!["window".to_string()]);
        assert!(filtered.recent_continuity_context.is_empty());
    }
}
