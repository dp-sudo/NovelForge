use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConstraintLevel {
    Weak,
    Normal,
    Strong,
    Absolute,
}

impl ConstraintLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Weak => "weak",
            Self::Normal => "normal",
            Self::Strong => "strong",
            Self::Absolute => "absolute",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "weak" => Some(Self::Weak),
            "normal" => Some(Self::Normal),
            "strong" => Some(Self::Strong),
            "absolute" => Some(Self::Absolute),
            _ => None,
        }
    }
}
