use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

use rusqlite::params;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::path_utils::resolve_project_relative_path;
use crate::services::ai_pipeline_service::RunAiTaskPipelineInput;
use crate::services::context_service::CollectedContext;
use crate::services::skill_registry::SkillRegistry;

#[derive(Clone, Default)]
pub struct PromptResolver;

#[derive(Debug, Clone, Default)]
struct PromptRenderContext {
    // 问题1修复: 统一渲染上下文字典（input/context/task_meta）并提供扁平键访问。
    input: BTreeMap<String, String>,
    context: BTreeMap<String, String>,
    task_meta: BTreeMap<String, String>,
    merged: BTreeMap<String, String>,
}

impl PromptRenderContext {
    fn set_input(&mut self, key: &str, value: String) {
        self.input.insert(key.to_string(), value.clone());
        self.merged.insert(key.to_string(), value.clone());
        self.merged.insert(format!("input.{key}"), value);
    }

    fn set_context(&mut self, key: &str, value: String) {
        self.context.insert(key.to_string(), value.clone());
        self.merged.insert(key.to_string(), value.clone());
        self.merged.insert(format!("context.{key}"), value);
    }

    fn set_task_meta(&mut self, key: &str, value: String) {
        self.task_meta.insert(key.to_string(), value.clone());
        self.merged.insert(key.to_string(), value.clone());
        self.merged.insert(format!("task_meta.{key}"), value);
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.merged.get(key).map(String::as_str)
    }
}

impl PromptResolver {
    pub fn resolve_or_build_prompt(
        &self,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        context: &CollectedContext,
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> Result<String, AppErrorDto> {
        let guard = skill_registry.read().map_err(|err| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                .with_detail(err.to_string())
        })?;

        // 问题2修复: Prompt 真相源收敛到 skills markdown，缺失模板时显式报错。
        let template = guard
            .read_skill_prompt_template(canonical_task)?
            .ok_or_else(|| Self::template_not_found_error(canonical_task))?;

        let render_context = self.build_render_context(context, canonical_task, input);
        Self::validate_required_inputs(canonical_task, &render_context)?;
        Self::render_template(canonical_task, &template, &render_context)
    }

    fn template_not_found_error(task_type: &str) -> AppErrorDto {
        AppErrorDto::new(
            "PROMPT_TEMPLATE_NOT_FOUND",
            &format!("Task '{}' has no prompt template", task_type),
            true,
        )
        .with_suggested_action("请为该任务创建对应的 Skill 模板")
    }

    fn build_render_context(
        &self,
        context: &CollectedContext,
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> PromptRenderContext {
        let mut render = PromptRenderContext::default();
        let user_instruction = input.user_instruction.trim().to_string();
        let selected_text = normalize_optional(input.selected_text.as_deref());
        let chapter_content = normalize_optional(input.chapter_content.as_deref());
        let preceding_text = if canonical_task == "chapter.continue" {
            self.resolve_preceding_text(input, context, &selected_text, &chapter_content)
        } else {
            selected_text.clone()
        };
        let chapter_context = Self::chapter_context_to_string(context);
        let project_context = Self::context_to_string(context);
        let target_words = context
            .related_context
            .chapter
            .as_ref()
            .map(|chapter| chapter.target_words.to_string())
            .unwrap_or_default();

        render.set_input("userInstruction", user_instruction.clone());
        render.set_input("userDescription", user_instruction.clone());
        render.set_input("selectedText", selected_text);
        render.set_input("precedingText", preceding_text);
        render.set_input("chapterContent", chapter_content.clone());
        render.set_input("content", chapter_content);
        render.set_input(
            "chapterId",
            normalize_optional(input.chapter_id.as_deref()),
        );
        render.set_input(
            "stepTitle",
            normalize_optional(input.blueprint_step_title.as_deref()),
        );
        render.set_input(
            "stepKey",
            normalize_optional(input.blueprint_step_key.as_deref()),
        );
        render.set_input("targetWords", target_words);

        render.set_context("projectContext", project_context);
        render.set_context("chapterContext", chapter_context);

        render.set_task_meta("taskType", canonical_task.to_string());
        render.set_task_meta("uiAction", normalize_optional(input.ui_action.as_deref()));

        render
    }

    // 问题1修复: 缺参硬校验，避免关键输入为空仍静默放行。
    fn validate_required_inputs(
        task_type: &str,
        render_context: &PromptRenderContext,
    ) -> Result<(), AppErrorDto> {
        let missing = required_input_keys(task_type)
            .iter()
            .filter_map(|key| {
                let value = render_context
                    .input
                    .get(*key)
                    .or_else(|| render_context.context.get(*key))
                    .or_else(|| render_context.task_meta.get(*key))
                    .map(String::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if value.is_empty() {
                    Some((*key).to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();
        if missing.is_empty() {
            return Ok(());
        }
        Err(AppErrorDto::new(
            "MISSING_REQUIRED_INPUT",
            &format!(
                "Task '{}' missing required prompt inputs: {}",
                task_type,
                missing.join(", ")
            ),
            true,
        )
        .with_suggested_action("请补齐必填输入后重试"))
    }

    fn render_template(
        task_type: &str,
        template: &str,
        render_context: &PromptRenderContext,
    ) -> Result<String, AppErrorDto> {
        let placeholders = extract_placeholders(template);
        let mut rendered = template.to_string();
        let mut missing_keys = Vec::new();

        for key in placeholders {
            let token = format!("{{{key}}}");
            if let Some(value) = render_context.get(&key) {
                rendered = rendered.replace(&token, value);
            } else {
                missing_keys.push(key);
            }
        }

        if !missing_keys.is_empty() {
            return Err(AppErrorDto::new(
                "MISSING_REQUIRED_INPUT",
                &format!(
                    "Task '{}' missing prompt variables: {}",
                    task_type,
                    missing_keys.join(", ")
                ),
                true,
            ));
        }

        // 问题1修复: 渲染后未解析占位符硬校验，禁止静默透传到 LLM。
        let unresolved = extract_placeholders(&rendered);
        if !unresolved.is_empty() {
            return Err(AppErrorDto::new(
                "PROMPT_UNRESOLVED_PLACEHOLDER",
                &format!(
                    "Task '{}' has unresolved placeholders after render: {}",
                    task_type,
                    unresolved.join(", ")
                ),
                true,
            ));
        }

        Ok(rendered)
    }

    fn resolve_preceding_text(
        &self,
        input: &RunAiTaskPipelineInput,
        context: &CollectedContext,
        selected_text: &str,
        chapter_content: &str,
    ) -> String {
        if !selected_text.is_empty() {
            return selected_text.to_string();
        }
        if !chapter_content.is_empty() {
            return tail_chars(chapter_content, 1200);
        }
        if let Some(chapter_id) = input.chapter_id.as_deref() {
            if let Ok(text) = load_chapter_tail(&input.project_root, chapter_id, 1200) {
                if !text.trim().is_empty() {
                    return text;
                }
            }
        }
        context
            .related_context
            .chapter
            .as_ref()
            .map(|chapter| chapter.summary.trim().to_string())
            .unwrap_or_default()
    }

    pub fn context_to_string(context: &CollectedContext) -> String {
        let global = &context.global_context;
        let related = &context.related_context;
        let mut parts = vec![];

        parts.push(format!("作品名称：{}", global.project_name));
        parts.push(format!("题材：{}", global.genre));
        if let Some(ref pov) = global.narrative_pov {
            parts.push(format!("叙事视角：{}", pov));
        }
        if let Some(ref style) = global.writing_style {
            parts.push(format!(
                "写作风格：语言={}、描写密度={}、对话比例={}、句式节奏={}、氛围={}、心理深度={}",
                style.language_style,
                style.description_density,
                style.dialogue_ratio,
                style.sentence_rhythm,
                style.atmosphere,
                style.psychological_depth
            ));
        }
        for step in &global.blueprint_summary {
            if step.status == "completed" {
                if let Some(ref content) = step.content {
                    let preview: String = content.chars().take(200).collect();
                    parts.push(format!("[蓝图] {}: {}", step.title, preview));
                }
            }
        }
        if let Some(ref ch) = related.chapter {
            parts.push(format!(
                "当前章节：{}（序号{}，目标{}字，当前{}字）",
                ch.title, ch.chapter_index, ch.target_words, ch.current_words
            ));
            if !ch.summary.is_empty() {
                parts.push(format!("当前章节摘要：{}", ch.summary));
            }
        }
        if let Some(ref prev) = related.previous_chapter_summary {
            if !prev.trim().is_empty() {
                parts.push(format!("前章摘要：{}", prev.trim()));
            }
        }
        for node in &related.plot_nodes {
            parts.push(format!("剧情节点：{}", node.title));
        }
        for ch in &related.characters {
            parts.push(format!("角色：{}", ch.name));
        }
        for edge in &related.relationship_edges {
            let mut line = format!(
                "关系：{} -> {} [{}]",
                edge.source_name, edge.target_name, edge.relationship_type
            );
            if let Some(ref description) = edge.description {
                if !description.trim().is_empty() {
                    line.push_str(&format!("：{}", description.trim()));
                }
            }
            parts.push(line);
        }
        for rule in &related.world_rules {
            let preview: String = rule.description.chars().take(120).collect();
            parts.push(format!("世界规则：{}", preview));
        }
        parts.join("\n")
    }

    fn chapter_context_to_string(context: &CollectedContext) -> String {
        let related = &context.related_context;
        let mut parts = Vec::new();

        if let Some(ref chapter) = related.chapter {
            parts.push(format!("标题：{}", chapter.title));
            parts.push(format!("章节序号：{}", chapter.chapter_index));
            parts.push(format!("目标字数：{}", chapter.target_words));
            parts.push(format!("当前字数：{}", chapter.current_words));
            if !chapter.summary.trim().is_empty() {
                parts.push(format!("摘要：{}", chapter.summary.trim()));
            }
        }

        if let Some(ref prev) = related.previous_chapter_summary {
            if !prev.trim().is_empty() {
                parts.push(format!("前章摘要：{}", prev.trim()));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n")
        }
    }

    pub fn requires_global_only_context(task_type: &str) -> bool {
        matches!(
            task_type,
            "character.create"
                | "world.create_rule"
                | "plot.create_node"
                | "blueprint.generate_step"
                | "glossary.create_term"
                | "narrative.create_obligation"
                | "timeline.review"
                | "relationship.review"
                | "dashboard.review"
                | "export.review"
        )
    }

    pub fn generate_user_message(task_type: &str) -> &'static str {
        match task_type {
            "character.create" => "请根据用户设想生成角色卡 JSON。",
            "world.create_rule" => "请根据用户设想生成世界设定 JSON。",
            "plot.create_node" => "请根据用户设想生成剧情节点 JSON。",
            "glossary.create_term" => "请根据用户设想生成名词条目 JSON。",
            "narrative.create_obligation" => "请根据用户设想生成叙事义务 JSON。",
            "consistency.scan" => "请检查章节一致性并输出 JSON。",
            _ => "请根据上述要求生成内容。",
        }
    }
}

fn required_input_keys(task_type: &str) -> &'static [&'static str] {
    match task_type {
        "chapter.continue" => &["precedingText"],
        "chapter.rewrite" => &["selectedText"],
        "prose.naturalize" => &["selectedText"],
        "character.create"
        | "world.create_rule"
        | "plot.create_node"
        | "glossary.create_term"
        | "narrative.create_obligation" => &["userDescription"],
        "consistency.scan" => &["chapterContent"],
        "blueprint.generate_step" => &["stepTitle"],
        _ => &[],
    }
}

fn normalize_optional(value: Option<&str>) -> String {
    value.unwrap_or("").trim().to_string()
}

fn tail_chars(value: &str, max_chars: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max_chars {
        return chars.into_iter().collect();
    }
    chars[chars.len().saturating_sub(max_chars)..]
        .iter()
        .collect::<String>()
}

fn load_chapter_tail(project_root: &str, chapter_id: &str, max_chars: usize) -> Result<String, AppErrorDto> {
    let root = project_root.trim();
    let chapter_id = chapter_id.trim();
    if root.is_empty() || chapter_id.is_empty() {
        return Ok(String::new());
    }

    let project_root_path = Path::new(root);
    let conn = open_database(project_root_path).map_err(|err| {
        AppErrorDto::new("DB_OPEN_FAILED", "无法读取章节前文", false).with_detail(err.to_string())
    })?;

    let content_path = conn
        .query_row(
            "SELECT content_path FROM chapters WHERE id = ?1 AND is_deleted = 0",
            params![chapter_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|err| {
            AppErrorDto::new("DB_QUERY_FAILED", "无法读取章节前文", false).with_detail(err.to_string())
        })?;

    let chapter_file = resolve_project_relative_path(project_root_path, &content_path)
        .map_err(|detail| AppErrorDto::new("PROJECT_PATH_INVALID_ENTRY", &detail, false))?;
    let content = fs::read_to_string(&chapter_file).map_err(|err| {
        AppErrorDto::new("CHAPTER_READ_FAILED", "无法读取章节正文", false).with_detail(err.to_string())
    })?;
    Ok(tail_chars(&content, max_chars))
}

fn extract_placeholders(template: &str) -> Vec<String> {
    let chars = template.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut seen = HashSet::new();
    let mut placeholders = Vec::new();

    while index < chars.len() {
        if chars[index] != '{' {
            index += 1;
            continue;
        }
        let mut end = index + 1;
        while end < chars.len() && chars[end] != '}' {
            end += 1;
        }
        if end >= chars.len() {
            break;
        }
        let key = chars[index + 1..end].iter().collect::<String>();
        if !key.is_empty()
            && key
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
            && seen.insert(key.clone())
        {
            placeholders.push(key);
        }
        index = end + 1;
    }

    placeholders
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, RwLock};

    use uuid::Uuid;

    use super::*;
    use crate::services::context_service::{
        BlueprintStepSummary, ChapterSummary, CollectedContext, GlobalContext, RelatedContext,
    };
    use crate::services::task_routing::CORE_TASK_ROUTE_TYPES;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{}-{}", prefix, Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

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

    fn sample_input(task_type: &str) -> RunAiTaskPipelineInput {
        match task_type {
            "chapter.draft" => RunAiTaskPipelineInput {
                project_root: "F:\\NovelForge".to_string(),
                task_type: task_type.to_string(),
                chapter_id: Some("ch-1".to_string()),
                ui_action: Some("test.chapter.draft".to_string()),
                user_instruction: "推进主线冲突".to_string(),
                selected_text: None,
                chapter_content: None,
                blueprint_step_key: None,
                blueprint_step_title: None,
                auto_persist: false,
            },
            "chapter.plan" => RunAiTaskPipelineInput {
                project_root: "F:\\NovelForge".to_string(),
                task_type: task_type.to_string(),
                chapter_id: Some("ch-1".to_string()),
                ui_action: Some("test.chapter.plan".to_string()),
                user_instruction: "给出场景拆分和节奏建议".to_string(),
                selected_text: None,
                chapter_content: None,
                blueprint_step_key: None,
                blueprint_step_title: None,
                auto_persist: false,
            },
            "chapter.continue" => RunAiTaskPipelineInput {
                project_root: "F:\\NovelForge".to_string(),
                task_type: task_type.to_string(),
                chapter_id: Some("ch-1".to_string()),
                ui_action: Some("test.chapter.continue".to_string()),
                user_instruction: "保持紧张节奏".to_string(),
                selected_text: Some("他推开门，黑暗里传来一声咳嗽。".to_string()),
                chapter_content: None,
                blueprint_step_key: None,
                blueprint_step_title: None,
                auto_persist: false,
            },
            "chapter.rewrite" | "prose.naturalize" => RunAiTaskPipelineInput {
                project_root: "F:\\NovelForge".to_string(),
                task_type: task_type.to_string(),
                chapter_id: Some("ch-1".to_string()),
                ui_action: Some("test.editor".to_string()),
                user_instruction: "让语气更克制".to_string(),
                selected_text: Some("然而，他感到一种无法言说的悲伤。".to_string()),
                chapter_content: None,
                blueprint_step_key: None,
                blueprint_step_title: None,
                auto_persist: false,
            },
            "character.create"
            | "world.create_rule"
            | "plot.create_node"
            | "glossary.create_term"
            | "narrative.create_obligation" => RunAiTaskPipelineInput {
                project_root: "F:\\NovelForge".to_string(),
                task_type: task_type.to_string(),
                chapter_id: None,
                ui_action: Some("test.asset.create".to_string()),
                user_instruction: "围绕家国复仇主题生成".to_string(),
                selected_text: None,
                chapter_content: None,
                blueprint_step_key: None,
                blueprint_step_title: None,
                auto_persist: false,
            },
            "consistency.scan" => RunAiTaskPipelineInput {
                project_root: "F:\\NovelForge".to_string(),
                task_type: task_type.to_string(),
                chapter_id: Some("ch-1".to_string()),
                ui_action: Some("test.consistency".to_string()),
                user_instruction: String::new(),
                selected_text: None,
                chapter_content: Some("角色A在本章中首次见到角色B。".to_string()),
                blueprint_step_key: None,
                blueprint_step_title: None,
                auto_persist: true,
            },
            "blueprint.generate_step" => RunAiTaskPipelineInput {
                project_root: "F:\\NovelForge".to_string(),
                task_type: task_type.to_string(),
                chapter_id: None,
                ui_action: Some("test.blueprint".to_string()),
                user_instruction: "给出两个可选方向".to_string(),
                selected_text: None,
                chapter_content: None,
                blueprint_step_key: Some("step-03-premise".to_string()),
                blueprint_step_title: Some("故事母题".to_string()),
                auto_persist: true,
            },
            "timeline.review" | "relationship.review" | "dashboard.review" | "export.review" => {
                RunAiTaskPipelineInput {
                    project_root: "F:\\NovelForge".to_string(),
                    task_type: task_type.to_string(),
                    chapter_id: None,
                    ui_action: Some("test.review".to_string()),
                    user_instruction: "重点找出高风险逻辑问题".to_string(),
                    selected_text: None,
                    chapter_content: None,
                    blueprint_step_key: None,
                    blueprint_step_title: None,
                    auto_persist: false,
                }
            }
            _ => panic!("unexpected task type: {}", task_type),
        }
    }

    fn create_registry_with_builtin_templates() -> Arc<RwLock<SkillRegistry>> {
        let app_data_dir = temp_dir("prompt-resolver-app-data");
        let skills_dir = app_data_dir.join("skills");
        let builtin_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("resources")
            .join("builtin-skills");
        let reg = SkillRegistry::new(skills_dir, builtin_dir);
        reg.initialize().expect("initialize skill registry");
        Arc::new(RwLock::new(reg))
    }

    #[test]
    fn all_core_tasks_render_without_unresolved_placeholders() {
        let resolver = PromptResolver;
        let context = sample_context();
        let registry = create_registry_with_builtin_templates();

        for task in CORE_TASK_ROUTE_TYPES {
            let input = sample_input(task);
            let rendered = resolver
                .resolve_or_build_prompt(&registry, &context, task, &input)
                .unwrap_or_else(|err| panic!("task {} render failed: {} {}", task, err.code, err.message));
            assert!(
                extract_placeholders(&rendered).is_empty(),
                "task {} still has placeholders: {:?}",
                task,
                extract_placeholders(&rendered)
            );
            assert!(
                !rendered.trim_start().starts_with("---"),
                "task {} prompt should not include frontmatter",
                task
            );
        }
    }

    #[test]
    fn missing_required_input_returns_missing_required_input() {
        let resolver = PromptResolver;
        let context = sample_context();
        let registry = create_registry_with_builtin_templates();

        let input = RunAiTaskPipelineInput {
            project_root: "F:\\NovelForge".to_string(),
            task_type: "character.create".to_string(),
            chapter_id: None,
            ui_action: Some("test.character".to_string()),
            user_instruction: String::new(),
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: false,
        };

        let err = resolver
            .resolve_or_build_prompt(&registry, &context, "character.create", &input)
            .expect_err("should reject missing userDescription");
        assert_eq!(err.code, "MISSING_REQUIRED_INPUT");
    }

    #[test]
    fn unknown_task_returns_prompt_template_not_found() {
        let resolver = PromptResolver;
        let context = sample_context();
        let registry = create_registry_with_builtin_templates();
        let input = sample_input("chapter.draft");

        let err = resolver
            .resolve_or_build_prompt(&registry, &context, "unknown.task", &input)
            .expect_err("unknown task must fail");
        assert_eq!(err.code, "PROMPT_TEMPLATE_NOT_FOUND");
    }
}
