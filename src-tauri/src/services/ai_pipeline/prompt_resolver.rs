use std::sync::{Arc, RwLock};

use crate::errors::AppErrorDto;
use crate::services::ai_pipeline_service::RunAiTaskPipelineInput;
use crate::services::context_service::CollectedContext;
use crate::services::prompt_builder::PromptBuilder;
use crate::services::skill_registry::SkillRegistry;

#[derive(Clone, Default)]
pub struct PromptResolver;

impl PromptResolver {
    pub fn resolve_or_build_prompt(
        &self,
        skill_registry: &Arc<RwLock<SkillRegistry>>,
        context: &CollectedContext,
        canonical_task: &str,
        input: &RunAiTaskPipelineInput,
    ) -> Result<String, AppErrorDto> {
        let selected_text = input.selected_text.as_deref().unwrap_or("");
        let user_instruction = input.user_instruction.as_str();
        if let Ok(guard) = skill_registry.read() {
            if let Ok(Some(content)) = guard.read_skill_content(canonical_task) {
                let context_str = Self::context_to_string(context);
                let rendered = content
                    .replace("{projectContext}", &context_str)
                    .replace("{userInstruction}", user_instruction)
                    .replace("{selectedText}", selected_text);
                return Ok(rendered);
            }
        }

        let prompt = match canonical_task {
            "chapter.continue" => {
                PromptBuilder::build_continue(context, selected_text, user_instruction)
            }
            "chapter.rewrite" => PromptBuilder::build_rewrite(context, selected_text, user_instruction),
            "prose.naturalize" => PromptBuilder::build_naturalize(context, selected_text),
            "chapter.plan" => PromptBuilder::build_chapter_plan(context, user_instruction),
            "character.create" => PromptBuilder::build_character_create(context, user_instruction),
            "world.create_rule" => {
                PromptBuilder::build_world_create_rule(context, user_instruction)
            }
            "plot.create_node" => PromptBuilder::build_plot_create_node(context, user_instruction),
            "glossary.create_term" => {
                PromptBuilder::build_glossary_create_term(context, user_instruction)
            }
            "narrative.create_obligation" => {
                PromptBuilder::build_narrative_create_obligation(context, user_instruction)
            }
            "consistency.scan" => {
                let chapter_content = input.chapter_content.as_deref().unwrap_or("");
                PromptBuilder::build_consistency_scan(context, chapter_content)
            }
            "blueprint.generate_step" => {
                let step_key = input.blueprint_step_key.as_deref().unwrap_or_default();
                let step_title = input.blueprint_step_title.as_deref().unwrap_or_default();
                PromptBuilder::build_blueprint_step(context, step_key, step_title, user_instruction)
            }
            "timeline.review" => PromptBuilder::build_timeline_review(context, user_instruction),
            "relationship.review" => {
                PromptBuilder::build_relationship_review(context, user_instruction)
            }
            "dashboard.review" => PromptBuilder::build_dashboard_review(context, user_instruction),
            "export.review" => PromptBuilder::build_export_review(context, user_instruction),
            _ => PromptBuilder::build_chapter_draft(context, user_instruction),
        };
        Ok(prompt)
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
        for step in &global.blueprint_summary {
            if step.status == "completed" {
                if let Some(ref content) = step.content {
                    let preview: String = content.chars().take(200).collect();
                    parts.push(format!("[蓝图] {}: {}", step.title, preview));
                }
            }
        }
        if let Some(ref ch) = related.chapter {
            parts.push(format!("章节：{}", ch.title));
            if !ch.summary.is_empty() {
                parts.push(format!("摘要：{}", ch.summary));
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
