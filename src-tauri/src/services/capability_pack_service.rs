use serde::Serialize;

use crate::services::context_service::CollectedContext;
use crate::services::task_routing::TaskExecutionContract;

/// Concrete scene capability parameters injected into prompts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneCapabilityPack {
    pub pack_id: String,
    pub pack_label: String,
    pub description_logic: DescriptionLogic,
    pub emotional_tone: EmotionalTone,
    pub narrative_rhythm: NarrativeRhythm,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DescriptionLogic {
    pub density: i32,
    pub focus_elements: Vec<String>,
    pub avoid_elements: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotionalTone {
    pub primary: String,
    pub secondary: Option<String>,
    pub intensity: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeRhythm {
    pub pacing: String,
    pub scene_beat_density: i32,
    pub transition_style: String,
}

#[derive(Default, Clone)]
pub struct CapabilityPackService;

impl CapabilityPackService {
    /// Build a concrete capability pack based on task contract and context.
    pub fn resolve_pack(
        &self,
        contract: &TaskExecutionContract,
        context: &CollectedContext,
    ) -> SceneCapabilityPack {
        let writing_style = &context.global_context.writing_style;

        match contract.capability_pack {
            "scene-production-pack" => self.build_scene_production_pack(writing_style, context),
            "asset-building-pack" => self.build_asset_building_pack(),
            "blueprint-planning-pack" => self.build_blueprint_planning_pack(),
            "review-guard-pack" => self.build_review_guard_pack(),
            _ => self.build_default_pack(contract.capability_pack),
        }
    }

    /// Format the resolved pack as prompt-injectable text.
    pub fn format_for_prompt(&self, pack: &SceneCapabilityPack) -> String {
        let mut lines = vec![format!("# 场景能力包：{}", pack.pack_label)];

        lines.push(format!(
            "- 描写密度：{}/7（{}）",
            pack.description_logic.density,
            if pack.description_logic.density <= 2 {
                "点到为止"
            } else if pack.description_logic.density <= 5 {
                "适度描写"
            } else {
                "详细刻画"
            }
        ));

        if !pack.description_logic.focus_elements.is_empty() {
            lines.push(format!(
                "- 重点描写：{}",
                pack.description_logic.focus_elements.join("、")
            ));
        }
        if !pack.description_logic.avoid_elements.is_empty() {
            lines.push(format!(
                "- 避免描写：{}",
                pack.description_logic.avoid_elements.join("、")
            ));
        }

        lines.push(format!(
            "- 情感基调：{}（强度 {}/7）",
            pack.emotional_tone.primary, pack.emotional_tone.intensity
        ));
        if let Some(ref sec) = pack.emotional_tone.secondary {
            lines.push(format!("- 次要情感：{}", sec));
        }

        lines.push(format!(
            "- 叙事节奏：{}（节拍密度 {}/7）",
            match pack.narrative_rhythm.pacing.as_str() {
                "fast" => "快节奏",
                "slow" => "慢节奏",
                "moderate" => "中等",
                "varied" => "变化",
                _ => &pack.narrative_rhythm.pacing,
            },
            pack.narrative_rhythm.scene_beat_density
        ));
        lines.push(format!(
            "- 转场风格：{}",
            match pack.narrative_rhythm.transition_style.as_str() {
                "sharp" => "硬切",
                "smooth" => "平滑过渡",
                "interleaved" => "交织",
                _ => &pack.narrative_rhythm.transition_style,
            }
        ));

        if !pack.constraints.is_empty() {
            lines.push("- 场景约束：".to_string());
            for c in &pack.constraints {
                lines.push(format!("  - {}", c));
            }
        }

        lines.push(String::new());
        lines.join("\n")
    }

    // --- Pack builders ---

    fn build_scene_production_pack(
        &self,
        writing_style: &Option<crate::services::project_service::WritingStyle>,
        context: &CollectedContext,
    ) -> SceneCapabilityPack {
        let (density, dialogue_hint) = match writing_style {
            Some(ws) => (ws.description_density, ws.dialogue_ratio),
            None => (4, 4),
        };

        let atmosphere = writing_style
            .as_ref()
            .map(|ws| ws.atmosphere.clone())
            .unwrap_or_else(|| "neutral".to_string());

        let primary_emotion = match atmosphere.as_str() {
            "warm" => "温暖",
            "cold" => "冷峻",
            "humorous" => "幽默",
            "serious" => "严肃",
            "suspenseful" => "紧张",
            _ => "中性",
        };

        let psych_depth = writing_style
            .as_ref()
            .map(|ws| ws.psychological_depth)
            .unwrap_or(4);

        let mut focus = vec!["动作与对话推进冲突".to_string()];
        if psych_depth >= 5 {
            focus.push("角色内心活动".to_string());
        }
        if density >= 5 {
            focus.push("环境氛围细节".to_string());
        }

        let mut avoid = vec!["空泛总结句".to_string(), "无关信息堆砌".to_string()];
        if dialogue_hint <= 2 {
            avoid.push("过多对话".to_string());
        }

        let has_conflict = context
            .related_context
            .plot_nodes
            .iter()
            .any(|n| n.conflict.is_some());

        let pacing = if has_conflict { "fast" } else { "moderate" };

        let mut constraints = vec![
            "服务于冲突推进".to_string(),
            "保持叙事视角一致".to_string(),
        ];
        if context.related_context.characters.len() > 3 {
            constraints.push("多角色场景注意视角焦点".to_string());
        }

        SceneCapabilityPack {
            pack_id: "scene-production-pack".to_string(),
            pack_label: "场景生产包".to_string(),
            description_logic: DescriptionLogic {
                density: density as i32,
                focus_elements: focus,
                avoid_elements: avoid,
            },
            emotional_tone: EmotionalTone {
                primary: primary_emotion.to_string(),
                secondary: None,
                intensity: psych_depth as i32,
            },
            narrative_rhythm: NarrativeRhythm {
                pacing: pacing.to_string(),
                scene_beat_density: 4,
                transition_style: "smooth".to_string(),
            },
            constraints,
        }
    }

    fn build_asset_building_pack(&self) -> SceneCapabilityPack {
        SceneCapabilityPack {
            pack_id: "asset-building-pack".to_string(),
            pack_label: "资产构建包".to_string(),
            description_logic: DescriptionLogic {
                density: 5,
                focus_elements: vec!["结构化字段完整性".to_string(), "与现有资产一致性".to_string()],
                avoid_elements: vec!["过度文学化描述".to_string()],
            },
            emotional_tone: EmotionalTone {
                primary: "客观".to_string(),
                secondary: None,
                intensity: 2,
            },
            narrative_rhythm: NarrativeRhythm {
                pacing: "moderate".to_string(),
                scene_beat_density: 2,
                transition_style: "sharp".to_string(),
            },
            constraints: vec![
                "输出必须符合指定 JSON 格式".to_string(),
                "不得与已有资产冲突".to_string(),
            ],
        }
    }

    fn build_blueprint_planning_pack(&self) -> SceneCapabilityPack {
        SceneCapabilityPack {
            pack_id: "blueprint-planning-pack".to_string(),
            pack_label: "蓝图规划包".to_string(),
            description_logic: DescriptionLogic {
                density: 4,
                focus_elements: vec!["可操作性".to_string(), "与已有步骤一致".to_string()],
                avoid_elements: vec!["空泛建议".to_string()],
            },
            emotional_tone: EmotionalTone {
                primary: "专业".to_string(),
                secondary: None,
                intensity: 2,
            },
            narrative_rhythm: NarrativeRhythm {
                pacing: "moderate".to_string(),
                scene_beat_density: 3,
                transition_style: "sharp".to_string(),
            },
            constraints: vec![
                "建议应具体、可执行".to_string(),
                "与已完成蓝图步骤保持一致".to_string(),
            ],
        }
    }

    fn build_review_guard_pack(&self) -> SceneCapabilityPack {
        SceneCapabilityPack {
            pack_id: "review-guard-pack".to_string(),
            pack_label: "审查守卫包".to_string(),
            description_logic: DescriptionLogic {
                density: 3,
                focus_elements: vec!["问题定位精确".to_string(), "修复建议可操作".to_string()],
                avoid_elements: vec!["泛化评论".to_string()],
            },
            emotional_tone: EmotionalTone {
                primary: "严谨".to_string(),
                secondary: None,
                intensity: 3,
            },
            narrative_rhythm: NarrativeRhythm {
                pacing: "moderate".to_string(),
                scene_beat_density: 2,
                transition_style: "sharp".to_string(),
            },
            constraints: vec![
                "检查结果必须引用原文".to_string(),
                "每个问题必须有明确的严重等级".to_string(),
            ],
        }
    }

    fn build_default_pack(&self, pack_id: &str) -> SceneCapabilityPack {
        SceneCapabilityPack {
            pack_id: pack_id.to_string(),
            pack_label: "通用能力包".to_string(),
            description_logic: DescriptionLogic {
                density: 4,
                focus_elements: vec![],
                avoid_elements: vec![],
            },
            emotional_tone: EmotionalTone {
                primary: "中性".to_string(),
                secondary: None,
                intensity: 4,
            },
            narrative_rhythm: NarrativeRhythm {
                pacing: "moderate".to_string(),
                scene_beat_density: 3,
                transition_style: "smooth".to_string(),
            },
            constraints: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::context_service::{
        CollectedContext, GlobalContext, RelatedContext,
    };

    fn minimal_context() -> CollectedContext {
        CollectedContext {
            global_context: GlobalContext {
                project_name: "测试".to_string(),
                genre: "玄幻".to_string(),
                narrative_pov: None,
                writing_style: None,
                locked_terms: vec![],
                banned_terms: vec![],
                blueprint_summary: vec![],
            },
            related_context: RelatedContext {
                chapter: None,
                characters: vec![],
                world_rules: vec![],
                plot_nodes: vec![],
                relationship_edges: vec![],
                previous_chapter_summary: None,
            },
        }
    }

    #[test]
    fn scene_production_pack_resolves() {
        let svc = CapabilityPackService;
        let contract = task_routing::task_execution_contract("chapter.draft");
        let ctx = minimal_context();
        let pack = svc.resolve_pack(&contract, &ctx);
        assert_eq!(pack.pack_id, "scene-production-pack");
        assert!(pack.description_logic.density > 0);
    }

    #[test]
    fn asset_building_pack_resolves() {
        let svc = CapabilityPackService;
        let contract = task_routing::task_execution_contract("character.create");
        let ctx = minimal_context();
        let pack = svc.resolve_pack(&contract, &ctx);
        assert_eq!(pack.pack_id, "asset-building-pack");
    }

    #[test]
    fn format_for_prompt_produces_text() {
        let svc = CapabilityPackService;
        let contract = task_routing::task_execution_contract("chapter.draft");
        let ctx = minimal_context();
        let pack = svc.resolve_pack(&contract, &ctx);
        let text = svc.format_for_prompt(&pack);
        assert!(text.contains("场景能力包"));
        assert!(text.contains("描写密度"));
    }
}
