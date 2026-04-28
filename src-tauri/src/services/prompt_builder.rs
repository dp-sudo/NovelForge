use crate::services::context_service::CollectedContext;
use crate::services::project_service::WritingStyle;

/// Structured prompt builder following the spec Document 4 §8 template format.
pub struct PromptBuilder;

impl PromptBuilder {
    /// Format writing style into a human-readable block for prompt injection.
    fn format_writing_style(style: &WritingStyle) -> String {
        let lang_label = match style.language_style.as_str() {
            "plain" => "平实",
            "balanced" => "适中",
            "ornate" => "华丽",
            "colloquial" => "口语化",
            _ => "适中",
        };

        let rhythm_label = match style.sentence_rhythm.as_str() {
            "short" => "短句为主",
            "long" => "长句为主",
            "mixed" => "混合",
            _ => "混合",
        };

        let atmosphere_label = match style.atmosphere.as_str() {
            "warm" => "温暖",
            "cold" => "冷峻",
            "humorous" => "幽默",
            "serious" => "严肃",
            "suspenseful" => "悬疑",
            "neutral" => "中性",
            _ => "中性",
        };

        format!(
            "写作风格：\n- 语言风格：{}\n- 描写密度：{}（1=点到为止，7=详细刻画）\n- 对话比例：{}（1=偏叙述，7=偏对话）\n- 句子节奏：{}\n- 氛围基调：{}\n- 心理描写深度：{}（1=仅外部行为，7=深入内心）",
            lang_label,
            style.description_density,
            style.dialogue_ratio,
            rhythm_label,
            atmosphere_label,
            style.psychological_depth,
        )
    }

    /// Build a chapter draft generation prompt.
    pub fn build_chapter_draft(context: &CollectedContext, user_instruction: &str) -> String {
        let global = &context.global_context;
        let related = &context.related_context;

        let mut parts = vec![];

        // Role
        parts.push("# 角色".to_string());
        parts.push("你是专业长篇小说章节写作助手，擅长按照既定角色、世界规则和剧情节点生成稳定的章节草稿。".to_string());
        parts.push(String::new());

        // Task
        parts.push("# 任务".to_string());
        parts.push("根据当前章节目标生成一版章节正文草稿。".to_string());
        parts.push(String::new());

        // Project context
        parts.push("# 固定上下文".to_string());
        parts.push(format!("作品名称：{}", global.project_name));
        parts.push(format!("题材：{}", global.genre));
        if let Some(ref pov) = global.narrative_pov {
            parts.push(format!("叙事视角：{}", pov));
        }
        if !global.locked_terms.is_empty() {
            parts.push(format!("锁定名词：{}", global.locked_terms.join("、")));
        }
        if !global.banned_terms.is_empty() {
            parts.push(format!("禁用词：{}", global.banned_terms.join("、")));
        }
        for step in &global.blueprint_summary {
            if step.status == "completed" {
                if let Some(ref content) = step.content {
                    let preview: String = content.chars().take(200).collect();
                    parts.push(format!("[蓝图表] {}: {}", step.title, preview));
                }
            }
        }
        if let Some(ref writing_style) = global.writing_style {
            parts.push(Self::format_writing_style(writing_style));
        }
        parts.push(String::new());

        // Current chapter info
        if let Some(ref ch) = related.chapter {
            parts.push("# 当前章节信息".to_string());
            parts.push(format!("章节标题：{}", ch.title));
            if !ch.summary.is_empty() {
                parts.push(format!("章节摘要：{}", ch.summary));
            }
            parts.push(format!("章节状态：{}", ch.status));
            parts.push(String::new());
        }

        // Plot nodes
        if !related.plot_nodes.is_empty() {
            parts.push("# 关联剧情节点".to_string());
            for node in &related.plot_nodes {
                parts.push(format!(
                    "- [{}] {}（目标：{} / 冲突：{}）",
                    node.node_type,
                    node.title,
                    node.goal.as_deref().unwrap_or("未设定"),
                    node.conflict.as_deref().unwrap_or("未设定"),
                ));
            }
            parts.push(String::new());
        }

        // Characters
        if !related.characters.is_empty() {
            parts.push("# 出场角色".to_string());
            for ch in &related.characters {
                let mut desc = format!("- {}（类型：{}）", ch.name, ch.role_type);
                if let Some(ref motivation) = ch.motivation {
                    desc.push_str(&format!(" 动机：{}", motivation));
                }
                parts.push(desc);
            }
            parts.push(String::new());
        }

        // World rules
        if !related.world_rules.is_empty() {
            parts.push("# 相关世界规则".to_string());
            for rule in &related.world_rules {
                let preview: String = rule.description.chars().take(120).collect();
                parts.push(format!(
                    "- [{}] {}：{}",
                    rule.constraint_level, rule.title, preview
                ));
            }
            parts.push(String::new());
        }

        // Previous chapter summary
        if let Some(ref prev_summary) = related.previous_chapter_summary {
            if !prev_summary.is_empty() {
                parts.push("# 上一章摘要".to_string());
                parts.push(prev_summary.clone());
                parts.push(String::new());
            }
        }

        // User instruction
        parts.push("# 写作要求".to_string());
        parts.push(user_instruction.to_string());
        parts.push(String::new());

        // Constraints
        parts.push("# 严格约束".to_string());
        parts.push("1. 不得改写已锁定设定。".to_string());
        parts.push("2. 不得新增没有铺垫的重大世界规则。".to_string());
        parts.push("3. 不得让角色做出明显违背动机的行为。".to_string());
        parts.push("4. 不要使用空泛总结句，例如「这一刻，他明白了命运的重量」。".to_string());
        parts.push("5. 对话、动作、环境描写要服务于冲突推进。".to_string());
        parts.push("6. 保持叙事视角一致。".to_string());
        if !global.banned_terms.is_empty() {
            parts.push(format!(
                "7. 禁止使用以下词汇：{}。",
                global.banned_terms.join("、")
            ));
        }
        parts.push(String::new());

        // Output format
        parts.push("# 输出".to_string());
        parts.push("请只输出章节正文，不要输出解释。".to_string());

        parts.join("\n")
    }

    /// Build a chapter continue prompt.
    pub fn build_continue(
        context: &CollectedContext,
        preceding_text: &str,
        user_instruction: &str,
    ) -> String {
        let mut prompt = Self::build_chapter_draft(context, user_instruction);
        prompt.push_str("\n\n# 前文内容\n");
        prompt.push_str(preceding_text);
        prompt.push_str("\n\n请从上文结尾处继续续写。只输出续写内容。");
        prompt
    }

    /// Build a rewrite selection prompt.
    pub fn build_rewrite(
        context: &CollectedContext,
        selected_text: &str,
        user_instruction: &str,
    ) -> String {
        let global = &context.global_context;

        let mut parts = vec![];

        parts.push("# 角色".to_string());
        parts.push("你是专业小说文本修订编辑，擅长改写文本并保持事实不变。".to_string());
        parts.push(String::new());

        parts.push("# 任务".to_string());
        parts.push(match user_instruction.trim() {
            i if i.is_empty() => "改写选中的文本，让表达更自然、更具体、更有画面感。".to_string(),
            _ => format!("根据以下要求改写选中的文本：{}", user_instruction),
        });
        parts.push(String::new());

        parts.push("# 项目上下文".to_string());
        parts.push(format!("作品名称：{}", global.project_name));
        parts.push(format!("题材：{}", global.genre));
        if let Some(ref pov) = global.narrative_pov {
            parts.push(format!("叙事视角：{}", pov));
        }
        if let Some(ref writing_style) = global.writing_style {
            parts.push(Self::format_writing_style(writing_style));
        }
        parts.push(String::new());

        parts.push("# 原文".to_string());
        parts.push(selected_text.to_string());
        parts.push(String::new());

        parts.push("# 约束".to_string());
        parts.push("1. 不改变事实和人物关系。".to_string());
        parts.push("2. 不新增重大设定。".to_string());
        parts.push("3. 不改变叙事视角。".to_string());
        parts.push("4. 减少空泛感叹和总结。".to_string());
        parts.push("5. 保留原文核心信息。".to_string());
        parts.push(String::new());

        parts.push("# 输出".to_string());
        parts.push("只输出改写后的文本，不要输出解释。".to_string());

        parts.join("\n")
    }

    /// Build a de-AI-ify prompt.
    pub fn build_naturalize(_context: &CollectedContext, selected_text: &str) -> String {
        let mut parts = vec![];

        parts.push("# 角色".to_string());
        parts.push("你是中文小说文本修订编辑，擅长去除模板化 AI 腔，保持事实不变。".to_string());
        parts.push(String::new());

        parts.push("# 任务".to_string());
        parts.push("改写选中的文本，让表达更自然、更具体、更有动作和画面感。".to_string());
        parts.push(String::new());

        parts.push("# 原文".to_string());
        parts.push(selected_text.to_string());
        parts.push(String::new());

        parts.push("# 约束".to_string());
        parts.push("1. 不改变事实。".to_string());
        parts.push("2. 不改变人物关系。".to_string());
        parts.push("3. 不新增重大设定。".to_string());
        parts.push("4. 不改变叙事视角。".to_string());
        parts.push("5. 减少空泛感叹和总结。".to_string());
        parts.push("6. 保留原文核心信息。".to_string());
        parts.push(String::new());

        parts.push("# 输出".to_string());
        parts.push("只输出改写后的文本。".to_string());

        parts.join("\n")
    }

    /// Build a blueprint step generation prompt.
    pub fn build_blueprint_step(
        context: &CollectedContext,
        step_key: &str,
        step_title: &str,
        user_instruction: &str,
    ) -> String {
        let mut parts = vec![];

        parts.push("# 角色".to_string());
        let agent = match step_key {
            "step-01-anchor" => "你是小说灵感策划师，擅长帮助作者明确创作方向和读者定位。",
            "step-02-genre" => "你是类型小说专家，深入了解各类型规则、节奏和读者预期。",
            "step-03-premise" => "你是故事架构师，擅长将灵感转化为完整故事梗概。",
            "step-04-characters" => "你是角色设计师，擅长创造有深度、有弧光的角色。",
            "step-05-world" => "你是世界观架构师，擅长构建自洽且富有想象力的世界。",
            "step-06-glossary" => "你是名词管理专家，擅长建立统一的作品术语体系。",
            "step-07-plot" => "你是剧情策划师，擅长设计有张力的主线骨架。",
            "step-08-chapters" => "你是章节规划师，擅长将剧情拆解为可执行的章节路线。",
            _ => "你是文火 NovelForge 的小说创作助手。",
        };
        parts.push(agent.to_string());
        parts.push(String::new());

        parts.push("# 任务".to_string());
        parts.push(format!(
            "为作品「{}」生成「{}」步骤的建议内容。",
            context.global_context.project_name, step_title
        ));
        if !user_instruction.is_empty() {
            parts.push(format!("用户需求：{}", user_instruction));
        }
        parts.push(String::new());

        parts.push("# 项目上下文".to_string());
        parts.push(format!("作品名称：{}", context.global_context.project_name));
        parts.push(format!("题材：{}", context.global_context.genre));
        for step in &context.global_context.blueprint_summary {
            if step.status == "completed" && step.step_key != step_key {
                if let Some(ref content) = step.content {
                    let preview: String = content.chars().take(150).collect();
                    parts.push(format!("[已有设定] {}: {}", step.title, preview));
                }
            }
        }
        parts.push(String::new());

        parts.push("# 输出要求".to_string());
        parts.push("内容应具体、可操作，与已完成的蓝图步骤保持一致。".to_string());
        parts.push("只输出建议内容，不要输出解释。".to_string());

        parts.join("\n")
    }

    /// Build a character creation prompt. Returns JSON-oriented output.
    pub fn build_character_create(context: &CollectedContext, user_description: &str) -> String {
        let mut parts = vec![];

        parts.push("# 角色".to_string());
        parts.push("你是小说角色设计师，擅长根据用户设想创建结构化角色卡。".to_string());
        parts.push(String::new());

        parts.push("# 任务".to_string());
        parts.push(
            "根据用户设想生成结构化角色卡，包含完整的动机、欲望、恐惧、缺陷和成长弧线。"
                .to_string(),
        );
        parts.push(String::new());

        parts.push("# 项目上下文".to_string());
        parts.push(format!("作品名称：{}", context.global_context.project_name));
        parts.push(format!("题材：{}", context.global_context.genre));
        for step in &context.global_context.blueprint_summary {
            if step.status == "completed" && step.step_key == "step-04-characters" {
                if let Some(ref content) = step.content {
                    let preview: String = content.chars().take(200).collect();
                    parts.push(format!("[已有角色设定] {}: {}", step.title, preview));
                }
            }
        }
        parts.push(String::new());

        parts.push("# 用户设想".to_string());
        parts.push(user_description.to_string());
        parts.push(String::new());

        parts.push("# 输出 JSON 格式".to_string());
        parts.push(
            r#"{
  "name": "角色名",
  "aliases": ["别名"],
  "roleType": "主角/反派/配角/路人/组织角色",
  "identityText": "身份",
  "motivation": "核心动机",
  "desire": "欲望",
  "fear": "恐惧",
  "flaw": "缺陷",
  "arcStage": "成长弧线",
  "appearance": "外貌描述",
  "notes": "备注"
}"#
            .to_string(),
        );

        parts.join("\n")
    }

    /// Build a consistency scan prompt.
    pub fn build_consistency_scan(context: &CollectedContext, chapter_content: &str) -> String {
        let global = &context.global_context;
        let related = &context.related_context;

        let mut parts = vec![];

        parts.push("# 角色".to_string());
        parts.push("你是长篇小说一致性审稿员。".to_string());
        parts.push(String::new());

        parts.push("# 任务".to_string());
        parts.push("检查当前章节是否违反角色、名词、世界规则、时间线或文风约束。".to_string());
        parts.push(String::new());

        // Locked terms
        if !global.locked_terms.is_empty() {
            parts.push("# 已锁定名词".to_string());
            for term in &global.locked_terms {
                parts.push(format!("- {}（锁定）", term));
            }
            parts.push(String::new());
        }

        // Banned terms
        if !global.banned_terms.is_empty() {
            parts.push("# 禁用词".to_string());
            for term in &global.banned_terms {
                parts.push(format!("- {}（禁用）", term));
            }
            parts.push(String::new());
        }

        if let Some(ref writing_style) = global.writing_style {
            parts.push("## 写作风格约束".to_string());
            parts.push(Self::format_writing_style(writing_style));
            parts.push("一致性扫描应检查文本是否偏离设定的写作风格。".to_string());
            parts.push(String::new());
        }

        // Characters
        if !related.characters.is_empty() {
            parts.push("# 角色卡".to_string());
            for ch in &related.characters {
                parts.push(format!("- {}（{}）", ch.name, ch.role_type));
                if let Some(ref motivation) = ch.motivation {
                    parts.push(format!("  动机：{}", motivation));
                }
            }
            parts.push(String::new());
        }

        // World rules
        if !related.world_rules.is_empty() {
            parts.push("# 世界规则".to_string());
            for rule in &related.world_rules {
                let preview: String = rule.description.chars().take(120).collect();
                parts.push(format!(
                    "- [{}] {}：{}",
                    rule.constraint_level, rule.title, preview
                ));
            }
            parts.push(String::new());
        }

        // Chapter content
        parts.push("# 当前章节正文".to_string());
        let content_preview: String = chapter_content.chars().take(2000).collect();
        parts.push(content_preview);
        parts.push(String::new());

        parts.push("# 检查维度".to_string());
        parts.push("1. 名词误写或别名误用。".to_string());
        parts.push("2. 角色动机、身份、关系冲突。".to_string());
        parts.push("3. 世界规则冲突。".to_string());
        parts.push("4. 新增未登记的重要角色、地点、组织。".to_string());
        parts.push("5. 明显 AI 腔、套话、空泛总结。".to_string());
        parts.push(String::new());

        parts.push("# 输出 JSON 格式".to_string());
        parts.push(
            r#"{
  "issues": [
    {
      "issueType": "glossary/character/world_rule/timeline/prose_style",
      "severity": "low/medium/high/blocker",
      "sourceText": "原文片段",
      "explanation": "问题说明",
      "suggestedFix": "修复建议"
    }
  ]
}"#
            .to_string(),
        );

        parts.join("\n")
    }

    /// Build a chapter plan prompt. Returns JSON-oriented output.
    pub fn build_chapter_plan(context: &CollectedContext, user_instruction: &str) -> String {
        let global = &context.global_context;
        let related = &context.related_context;

        let mut parts = vec![];
        parts.push("# 角色".to_string());
        parts.push("你是长篇小说剧情规划师，擅长将创作蓝图拆解为可执行的章节计划。".to_string());
        parts.push(String::new());

        parts.push("# 任务".to_string());
        parts.push("为当前章节生成可执行的章节计划。".to_string());
        parts.push(String::new());

        parts.push("# 项目上下文".to_string());
        parts.push(format!("作品名称：{}", global.project_name));
        parts.push(format!("题材：{}", global.genre));
        if let Some(ref pov) = global.narrative_pov {
            parts.push(format!("叙事视角：{}", pov));
        }
        if let Some(ref writing_style) = global.writing_style {
            parts.push(Self::format_writing_style(writing_style));
        }
        parts.push(String::new());

        if let Some(ref ch) = related.chapter {
            parts.push("# 当前章节".to_string());
            parts.push(format!("标题：{}", ch.title));
            if !ch.summary.is_empty() {
                parts.push(format!("摘要：{}", ch.summary));
            }
            parts.push(String::new());
        }

        if !related.plot_nodes.is_empty() {
            parts.push("# 关联主线节点".to_string());
            for node in &related.plot_nodes {
                parts.push(format!("- {}（{}）", node.title, node.node_type));
            }
            parts.push(String::new());
        }

        if !related.characters.is_empty() {
            parts.push("# 出场角色".to_string());
            for ch in &related.characters {
                parts.push(format!("- {}（{}）", ch.name, ch.role_type));
            }
            parts.push(String::new());
        }

        if !user_instruction.is_empty() {
            parts.push("# 用户要求".to_string());
            parts.push(user_instruction.to_string());
            parts.push(String::new());
        }

        parts.push("# 约束".to_string());
        parts.push("1. 章节计划必须服务于主线推进。".to_string());
        parts.push("2. 场景节拍应符合叙事节奏。".to_string());
        parts.push("3. 伏笔应自然嵌入场景描述中。".to_string());
        parts.push(String::new());

        parts.push(
            r#"# 输出 JSON
{
  "title": "章节标题",
  "summary": "章节摘要",
  "sceneBeats": ["场景节拍1", "场景节拍2"],
  "conflict": "本章核心冲突",
  "characterProgress": "角色推进",
  "foreshadowing": ["可埋伏笔"],
  "risks": ["潜在风险"]
}"#
            .to_string(),
        );

        parts.join("\n")
    }

    /// Build a world rule creation prompt.
    pub fn build_world_create_rule(context: &CollectedContext, user_instruction: &str) -> String {
        let global = &context.global_context;

        let mut parts = vec![];
        parts.push("# 角色".to_string());
        parts.push("你是世界设定专家，擅长构建自洽且富有想象力的虚构世界体系。".to_string());
        parts.push(String::new());

        parts.push("# 任务".to_string());
        parts.push("根据用户需求生成一条世界设定。".to_string());
        parts.push(String::new());

        parts.push("# 项目上下文".to_string());
        parts.push(format!("作品名称：{}", global.project_name));
        parts.push(format!("题材：{}", global.genre));
        parts.push(String::new());

        // Include existing completed blueprint steps for context
        for step in &global.blueprint_summary {
            if step.status == "completed" {
                if let Some(ref content) = step.content {
                    let preview: String = content.chars().take(150).collect();
                    parts.push(format!("[已有设定] {}: {}", step.title, preview));
                }
            }
        }
        parts.push(String::new());

        if !user_instruction.is_empty() {
            parts.push("# 用户需求".to_string());
            parts.push(user_instruction.to_string());
            parts.push(String::new());
        }

        parts.push("# 约束".to_string());
        parts.push("1. 新设定必须与现有设定一致，不得冲突。".to_string());
        parts.push("2. 设定应有明确的约束等级。".to_string());
        parts.push("3. 设定应具体、可操作、可检查。".to_string());
        parts.push(String::new());

        parts.push(
            r#"# 输出 JSON
{
  "title": "设定标题",
  "category": "世界规则|地点|组织|道具|能力|历史事件|术语",
  "description": "详细描述",
  "constraintLevel": "weak|normal|strong|absolute",
  "examples": "示例",
  "contradictionPolicy": "冲突处理策略"
}"#
            .to_string(),
        );

        parts.join("\n")
    }

    /// Build a plot node creation prompt.
    pub fn build_plot_create_node(context: &CollectedContext, user_instruction: &str) -> String {
        let global = &context.global_context;

        let mut parts = vec![];
        parts.push("# 角色".to_string());
        parts.push("你是剧情策划师，擅长设计有张力的故事节点和冲突。".to_string());
        parts.push(String::new());

        parts.push("# 任务".to_string());
        parts.push("根据用户需求生成一个剧情节点。".to_string());
        parts.push(String::new());

        parts.push("# 项目上下文".to_string());
        parts.push(format!("作品名称：{}", global.project_name));
        parts.push(format!("题材：{}", global.genre));
        parts.push(String::new());

        for step in &global.blueprint_summary {
            if step.status == "completed" {
                if let Some(ref content) = step.content {
                    let preview: String = content.chars().take(150).collect();
                    parts.push(format!("[已有设定] {}: {}", step.title, preview));
                }
            }
        }
        parts.push(String::new());

        if !user_instruction.is_empty() {
            parts.push("# 用户需求".to_string());
            parts.push(user_instruction.to_string());
            parts.push(String::new());
        }

        parts.push("# 约束".to_string());
        parts.push("1. 新节点必须符合整体主线走向。".to_string());
        parts.push("2. 节点应有明确的冲突和目标。".to_string());
        parts.push("3. 节点顺序应符合叙事节奏。".to_string());
        parts.push(String::new());

        parts.push(
            r#"# 输出 JSON
{
  "title": "节点标题",
  "nodeType": "开端|转折|冲突|失败|胜利|高潮|结局|支线",
  "goal": "剧情目标",
  "conflict": "核心冲突",
  "emotionalCurve": "情绪曲线",
  "order": 1
}"#
            .to_string(),
        );

        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::PromptBuilder;
    use crate::services::context_service::{
        BlueprintStepSummary, CollectedContext, GlobalContext, RelatedContext,
    };
    use crate::services::project_service::WritingStyle;

    fn sample_context(writing_style: Option<WritingStyle>) -> CollectedContext {
        CollectedContext {
            global_context: GlobalContext {
                project_name: "测试作品".to_string(),
                genre: "奇幻".to_string(),
                narrative_pov: Some("third_limited".to_string()),
                writing_style,
                locked_terms: vec![],
                banned_terms: vec![],
                blueprint_summary: vec![BlueprintStepSummary {
                    step_key: "step-03-premise".to_string(),
                    title: "故事核心".to_string(),
                    content: Some("主角背负诅咒踏上旅程".to_string()),
                    status: "completed".to_string(),
                }],
            },
            related_context: RelatedContext {
                chapter: None,
                characters: vec![],
                world_rules: vec![],
                plot_nodes: vec![],
                previous_chapter_summary: None,
            },
        }
    }

    #[test]
    fn chapter_draft_includes_writing_style_block_when_present() {
        let context = sample_context(Some(WritingStyle {
            language_style: "ornate".to_string(),
            description_density: 6,
            dialogue_ratio: 3,
            sentence_rhythm: "long".to_string(),
            atmosphere: "suspenseful".to_string(),
            psychological_depth: 7,
        }));

        let prompt = PromptBuilder::build_chapter_draft(&context, "推进主线冲突");

        assert!(prompt.contains("写作风格："));
        assert!(prompt.contains("语言风格：华丽"));
        assert!(prompt.contains("句子节奏：长句为主"));
        assert!(prompt.contains("氛围基调：悬疑"));
    }

    #[test]
    fn consistency_scan_omits_writing_style_block_when_absent() {
        let context = sample_context(None);
        let prompt = PromptBuilder::build_consistency_scan(&context, "测试章节正文");
        assert!(!prompt.contains("写作风格约束"));
        assert!(!prompt.contains("写作风格："));
    }
}
