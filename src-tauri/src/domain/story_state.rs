use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StateCategory {
    Emotion,
    SceneEnvironment,
    RelationshipTemperature,
    CharacterAction,
    CharacterAppearance,
    CharacterKnowledge,
    SceneDangerLevel,
    SceneSpatialConstraint,
    Generic,
}

impl StateCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Emotion => "emotion",
            Self::SceneEnvironment => "scene_environment",
            Self::RelationshipTemperature => "relationship_temperature",
            Self::CharacterAction => "character_action",
            Self::CharacterAppearance => "character_appearance",
            Self::CharacterKnowledge => "character_knowledge",
            Self::SceneDangerLevel => "scene_danger_level",
            Self::SceneSpatialConstraint => "scene_spatial_constraint",
            Self::Generic => "generic",
        }
    }
}
