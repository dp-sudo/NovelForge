use serde::Serialize;

/// A skill manifest following spec Document 4 §6.1 format.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub requires_user_confirmation: bool,
    pub writes_to_project: bool,
}

/// Registry of built-in AI skills.
#[derive(Default)]
pub struct SkillRegistry {
    skills: Vec<SkillManifest>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        let mut reg = Self { skills: Vec::new() };
        reg.register_builtins();
        reg
    }

    fn register_builtins(&mut self) {
        self.skills.push(SkillManifest {
            id: "context.collect".into(),
            name: "收集上下文".into(),
            description: "收集当前章节相关的项目资产上下文".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "chapterId": { "type": "string" },
                    "scope": { "type": "string", "enum": ["current_chapter", "full"] }
                }
            }),
            output_schema: serde_json::json!({ "type": "object" }),
            requires_user_confirmation: false,
            writes_to_project: false,
        });
        self.skills.push(SkillManifest {
            id: "chapter.draft".into(),
            name: "生成章节草稿".into(),
            description: "根据章节目标、角色、设定、主线节点生成章节正文草稿".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "chapterId": { "type": "string" },
                    "userInstruction": { "type": "string" },
                    "targetWords": { "type": "number" }
                }
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "draft": { "type": "string" },
                    "summary": { "type": "string" }
                }
            }),
            requires_user_confirmation: true,
            writes_to_project: false,
        });
        self.skills.push(SkillManifest {
            id: "chapter.continue".into(),
            name: "续写章节".into(),
            description: "根据光标前文续写章节正文".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "chapterId": { "type": "string" },
                    "precedingText": { "type": "string" },
                    "userInstruction": { "type": "string" }
                }
            }),
            output_schema: serde_json::json!({ "type": "object" }),
            requires_user_confirmation: true,
            writes_to_project: false,
        });
        self.skills.push(SkillManifest {
            id: "chapter.rewrite".into(),
            name: "改写选区".into(),
            description: "在不改变事实的前提下改写所选文本".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "selectedText": { "type": "string" },
                    "userInstruction": { "type": "string" }
                }
            }),
            output_schema: serde_json::json!({ "type": "object" }),
            requires_user_confirmation: true,
            writes_to_project: false,
        });
        self.skills.push(SkillManifest {
            id: "prose.naturalize".into(),
            name: "去 AI 味".into(),
            description: "优化文本表达，减少模板化 AI 腔".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "selectedText": { "type": "string" }
                }
            }),
            output_schema: serde_json::json!({ "type": "object" }),
            requires_user_confirmation: true,
            writes_to_project: false,
        });
        self.skills.push(SkillManifest {
            id: "blueprint.generate_step".into(),
            name: "生成蓝图步骤".into(),
            description: "基于项目已有设定生成单个蓝图步骤的建议内容".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "stepKey": { "type": "string" },
                    "userInstruction": { "type": "string" }
                }
            }),
            output_schema: serde_json::json!({ "type": "object" }),
            requires_user_confirmation: true,
            writes_to_project: false,
        });
        self.skills.push(SkillManifest {
            id: "character.create".into(),
            name: "创建角色卡".into(),
            description: "根据用户描述生成结构化角色卡".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "userDescription": { "type": "string" }
                }
            }),
            output_schema: serde_json::json!({ "type": "object" }),
            requires_user_confirmation: true,
            writes_to_project: false,
        });
        self.skills.push(SkillManifest {
            id: "consistency.scan".into(),
            name: "一致性扫描".into(),
            description: "检查当前章节是否与已有设定冲突".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "chapterId": { "type": "string" },
                    "scope": { "type": "string" }
                }
            }),
            output_schema: serde_json::json!({ "type": "object" }),
            requires_user_confirmation: false,
            writes_to_project: true,
        });
    }

    pub fn list_skills(&self) -> &[SkillManifest] {
        &self.skills
    }

    pub fn get_skill(&self, id: &str) -> Option<&SkillManifest> {
        self.skills.iter().find(|s| s.id == id)
    }
}
