use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackEventStatus {
    Open,
    Acknowledged,
    Resolved,
    Ignored,
}

impl FeedbackEventStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Acknowledged => "acknowledged",
            Self::Resolved => "resolved",
            Self::Ignored => "ignored",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "acknowledged" => Self::Acknowledged,
            "resolved" => Self::Resolved,
            "ignored" => Self::Ignored,
            _ => Self::Open,
        }
    }
}
