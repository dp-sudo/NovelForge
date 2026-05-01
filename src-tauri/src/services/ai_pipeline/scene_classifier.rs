use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneType {
    Dialogue,
    Action,
    Exposition,
    Introspection,
    Combat,
}

impl SceneType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dialogue => "dialogue",
            Self::Action => "action",
            Self::Exposition => "exposition",
            Self::Introspection => "introspection",
            Self::Combat => "combat",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneClassification {
    pub scene_type: String,
    pub confidence: f32,
    pub matched_features: Vec<String>,
}

#[derive(Default, Clone)]
pub struct SceneClassifier;

fn count_hits(haystack: &str, keywords: &[&str]) -> i64 {
    keywords
        .iter()
        .filter(|keyword| !keyword.is_empty() && haystack.contains(**keyword))
        .count() as i64
}

impl SceneClassifier {
    pub fn classify(
        &self,
        user_instruction: &str,
        selected_text: Option<&str>,
        chapter_content: Option<&str>,
    ) -> SceneClassification {
        let combined = format!(
            "{}\n{}\n{}",
            user_instruction,
            selected_text.unwrap_or(""),
            chapter_content.unwrap_or("")
        );
        let lowered = combined.to_ascii_lowercase();

        let dialogue_score = count_hits(
            &lowered,
            &[
                "dialogue", "conversation", "对话", "说道", "问道", "回答", "争辩", "告白",
            ],
        );
        let action_score = count_hits(
            &lowered,
            &[
                "action", "chase", "run", "行动", "追逐", "逃离", "潜入", "移动", "突袭",
            ],
        );
        let exposition_score = count_hits(
            &lowered,
            &[
                "exposition",
                "worldbuilding",
                "background",
                "设定",
                "背景",
                "解释",
                "历史",
                "说明",
            ],
        );
        let introspection_score = count_hits(
            &lowered,
            &[
                "introspection",
                "inner monologue",
                "内心",
                "独白",
                "心理",
                "回忆",
                "自省",
                "思考",
            ],
        );
        let combat_score = count_hits(
            &lowered,
            &[
                "combat", "battle", "fight", "skirmish", "战斗", "厮杀", "交锋", "搏斗", "决战",
                "反击", "受伤", "流血",
            ],
        );

        let mut scored = vec![
            (SceneType::Dialogue, dialogue_score),
            (SceneType::Action, action_score),
            (SceneType::Exposition, exposition_score),
            (SceneType::Introspection, introspection_score),
            (SceneType::Combat, combat_score),
        ];
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        let (scene, max_score) = scored.first().copied().unwrap_or((SceneType::Action, 0));
        let total = scored.iter().map(|(_, score)| *score).sum::<i64>().max(1);
        let confidence = (max_score as f32 / total as f32).clamp(0.2, 0.99);

        let mut matched_features = Vec::new();
        if dialogue_score > 0 {
            matched_features.push("dialogue-cues".to_string());
        }
        if action_score > 0 {
            matched_features.push("action-cues".to_string());
        }
        if exposition_score > 0 {
            matched_features.push("exposition-cues".to_string());
        }
        if introspection_score > 0 {
            matched_features.push("introspection-cues".to_string());
        }
        if combat_score > 0 {
            matched_features.push("combat-cues".to_string());
        }
        if matched_features.is_empty() {
            matched_features.push("fallback-action-default".to_string());
        }

        SceneClassification {
            scene_type: scene.as_str().to_string(),
            confidence,
            matched_features,
        }
    }

    pub fn default_post_tasks(scene_type: &str) -> Vec<String> {
        match scene_type.trim().to_ascii_lowercase().as_str() {
            "dialogue" => vec!["extract_state".to_string()],
            "combat" => vec![
                "review_continuity".to_string(),
                "extract_state".to_string(),
            ],
            "exposition" => vec!["extract_assets".to_string()],
            "introspection" => vec!["extract_state".to_string()],
            _ => vec!["extract_state".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SceneClassifier, SceneType};

    #[test]
    fn classify_dialogue_scene() {
        let classifier = SceneClassifier;
        let classified = classifier.classify("请增强对话张力", Some("他说：我们别无选择。"), None);
        assert_eq!(classified.scene_type, SceneType::Dialogue.as_str());
    }

    #[test]
    fn classify_combat_scene() {
        let classifier = SceneClassifier;
        let classified = classifier.classify(
            "把战斗节奏拉满",
            None,
            Some("二人交锋，刀光掠过，主角受伤流血。"),
        );
        assert_eq!(classified.scene_type, SceneType::Combat.as_str());
        let defaults = SceneClassifier::default_post_tasks(&classified.scene_type);
        assert_eq!(
            defaults,
            vec![
                "review_continuity".to_string(),
                "extract_state".to_string()
            ]
        );
    }
}
