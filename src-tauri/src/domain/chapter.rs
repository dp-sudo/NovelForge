use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChapterStatus {
    Planned,
    Drafting,
    Revising,
    Completed,
    Archived,
}

impl ChapterStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Drafting => "drafting",
            Self::Revising => "revising",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "planned" => Some(Self::Planned),
            "drafting" => Some(Self::Drafting),
            "revising" => Some(Self::Revising),
            "completed" => Some(Self::Completed),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}
