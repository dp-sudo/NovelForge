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

#[derive(Debug, Default, Clone, Copy)]
struct SceneFeatures {
    dialogue_ratio: f32,
    dialogue_hits: i64,
    action_hits: i64,
    combat_hits: i64,
    exposition_hits: i64,
    introspection_hits: i64,
    action_density: f32,
}

#[derive(Default, Clone)]
pub struct SceneClassifier;

fn count_hits(haystack: &str, keywords: &[&str]) -> i64 {
    keywords
        .iter()
        .filter(|keyword| !keyword.is_empty() && haystack.contains(**keyword))
        .count() as i64
}

fn estimate_dialogue_ratio(text: &str) -> f32 {
    let mut dialogue_chars = 0usize;
    let mut total_chars = 0usize;
    let chars = text.chars().collect::<Vec<_>>();
    let mut inside_cn_quote = false;
    let mut inside_en_quote = false;
    for ch in chars {
        if ch == '“' {
            inside_cn_quote = true;
            continue;
        }
        if ch == '”' {
            inside_cn_quote = false;
            continue;
        }
        if ch == '"' {
            inside_en_quote = !inside_en_quote;
            continue;
        }
        if ch.is_whitespace() {
            continue;
        }
        total_chars += 1;
        if inside_cn_quote || inside_en_quote {
            dialogue_chars += 1;
        }
    }
    if total_chars == 0 {
        return 0.0;
    }
    dialogue_chars as f32 / total_chars as f32
}

fn extract_features(
    user_instruction: &str,
    selected_text: Option<&str>,
    chapter_content: Option<&str>,
) -> SceneFeatures {
    let combined = format!(
        "{}\n{}\n{}",
        user_instruction,
        selected_text.unwrap_or(""),
        chapter_content.unwrap_or("")
    );
    let lowered = combined.to_ascii_lowercase();
    let length = lowered.chars().count().max(1) as f32;

    let dialogue_hits = count_hits(
        &lowered,
        &[
            "dialogue",
            "conversation",
            "对话",
            "说道",
            "问道",
            "回答",
            "争辩",
            "告白",
            "“",
            "”",
            "\"",
        ],
    );
    let action_hits = count_hits(
        &lowered,
        &[
            "action", "chase", "run", "rush", "move", "行动", "追逐", "逃离", "潜入", "移动",
            "突袭", "翻身", "闪避", "挥刀", "跃起",
        ],
    );
    let combat_hits = count_hits(
        &lowered,
        &[
            "combat", "battle", "fight", "skirmish", "战斗", "厮杀", "交锋", "搏斗", "决战",
            "反击", "受伤", "流血", "重伤", "骨折",
        ],
    );
    let exposition_hits = count_hits(
        &lowered,
        &[
            "exposition",
            "worldbuilding",
            "background",
            "lore",
            "设定",
            "背景",
            "解释",
            "历史",
            "说明",
            "传说",
            "规则",
            "起源",
        ],
    );
    let introspection_hits = count_hits(
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
            "犹豫",
            "恐惧",
            "后悔",
        ],
    );

    SceneFeatures {
        dialogue_ratio: estimate_dialogue_ratio(&combined),
        dialogue_hits,
        action_hits,
        combat_hits,
        exposition_hits,
        introspection_hits,
        action_density: action_hits as f32 / length,
    }
}

impl SceneClassifier {
    pub fn classify(
        &self,
        user_instruction: &str,
        selected_text: Option<&str>,
        chapter_content: Option<&str>,
    ) -> SceneClassification {
        let features = extract_features(user_instruction, selected_text, chapter_content);
        let mut matched_features = Vec::new();
        let mut scores = vec![
            (
                SceneType::Dialogue,
                features.dialogue_hits as f32 + features.dialogue_ratio * 10.0,
            ),
            (
                SceneType::Action,
                features.action_hits as f32 + features.action_density * 120.0,
            ),
            (SceneType::Exposition, features.exposition_hits as f32),
            (SceneType::Introspection, features.introspection_hits as f32),
            (
                SceneType::Combat,
                features.combat_hits as f32 + features.action_density * 40.0,
            ),
        ];

        if features.dialogue_ratio >= 0.6 {
            matched_features.push(format!("dialogue_ratio={:.2}", features.dialogue_ratio));
            if let Some((_, score)) = scores.iter_mut().find(|(ty, _)| *ty == SceneType::Dialogue) {
                *score += 8.0;
            }
        }

        if features.combat_hits >= 2 && features.action_density >= 0.01 {
            matched_features.push(format!(
                "combat_signal=hits:{} density:{:.3}",
                features.combat_hits, features.action_density
            ));
            if let Some((_, score)) = scores.iter_mut().find(|(ty, _)| *ty == SceneType::Combat) {
                *score += 10.0;
            }
        }

        if features.exposition_hits >= 2
            && features.dialogue_ratio <= 0.35
            && features.action_density <= 0.01
        {
            matched_features.push(format!("exposition_hits={}", features.exposition_hits));
            if let Some((_, score)) = scores
                .iter_mut()
                .find(|(ty, _)| *ty == SceneType::Exposition)
            {
                *score += 6.0;
            }
        }

        if features.introspection_hits >= 2 && features.action_hits <= 2 {
            matched_features.push(format!(
                "introspection_hits={}",
                features.introspection_hits
            ));
            if let Some((_, score)) = scores
                .iter_mut()
                .find(|(ty, _)| *ty == SceneType::Introspection)
            {
                *score += 6.0;
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let (scene, top_score) = scores.first().copied().unwrap_or((SceneType::Action, 0.0));
        let second_score = scores.get(1).map(|(_, score)| *score).unwrap_or(0.0);
        let spread = (top_score - second_score).max(0.0);
        let confidence = ((0.55 + spread / 20.0).clamp(0.2, 0.99)
            + features.dialogue_ratio.min(0.2))
        .clamp(0.2, 0.99);

        if matched_features.is_empty() {
            matched_features.push(format!(
                "fallback_scores:d={:.2},a={:.2},e={:.2},i={:.2},c={:.2}",
                scores
                    .iter()
                    .find(|(ty, _)| *ty == SceneType::Dialogue)
                    .map(|(_, score)| *score)
                    .unwrap_or(0.0),
                scores
                    .iter()
                    .find(|(ty, _)| *ty == SceneType::Action)
                    .map(|(_, score)| *score)
                    .unwrap_or(0.0),
                scores
                    .iter()
                    .find(|(ty, _)| *ty == SceneType::Exposition)
                    .map(|(_, score)| *score)
                    .unwrap_or(0.0),
                scores
                    .iter()
                    .find(|(ty, _)| *ty == SceneType::Introspection)
                    .map(|(_, score)| *score)
                    .unwrap_or(0.0),
                scores
                    .iter()
                    .find(|(ty, _)| *ty == SceneType::Combat)
                    .map(|(_, score)| *score)
                    .unwrap_or(0.0),
            ));
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
            "combat" => vec!["review_continuity".to_string(), "extract_state".to_string()],
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
    fn classify_dialogue_scene_by_ratio() {
        let classifier = SceneClassifier;
        let classified = classifier.classify(
            "请增强人物对话冲突",
            Some("“你别再骗我了。”“我没有骗你。”两人对视，语速越来越快。"),
            None,
        );
        assert_eq!(classified.scene_type, SceneType::Dialogue.as_str());
        assert!(classified.confidence >= 0.6);
    }

    #[test]
    fn classify_combat_scene_by_semantic_signals() {
        let classifier = SceneClassifier;
        let classified = classifier.classify(
            "强化打斗节奏",
            None,
            Some("两人翻身闪避后再度交锋，刀光连闪。主角受伤流血却仍反击。"),
        );
        assert_eq!(classified.scene_type, SceneType::Combat.as_str());
        let defaults = SceneClassifier::default_post_tasks(&classified.scene_type);
        assert_eq!(
            defaults,
            vec!["review_continuity".to_string(), "extract_state".to_string()]
        );
    }

    #[test]
    fn classify_exposition_scene_by_information_density() {
        let classifier = SceneClassifier;
        let classified = classifier.classify(
            "补设定",
            None,
            Some("这里补充王朝历史、宗门起源与禁术规则说明，并解释代价机制。"),
        );
        assert_eq!(classified.scene_type, SceneType::Exposition.as_str());
    }
}
