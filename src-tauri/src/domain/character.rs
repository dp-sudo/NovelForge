use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoleType {
    Protagonist,
    Antagonist,
    Supporting,
    Minor,
    Organization,
}

impl RoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Protagonist => "\u{4e3b}\u{89d2}",
            Self::Antagonist => "\u{53cd}\u{6d3e}",
            Self::Supporting => "\u{914d}\u{89d2}",
            Self::Minor => "\u{8def}\u{4eba}",
            Self::Organization => "\u{7ec4}\u{7ec7}\u{89d2}\u{8272}",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "\u{4e3b}\u{89d2}" => Some(Self::Protagonist),
            "\u{53cd}\u{6d3e}" => Some(Self::Antagonist),
            "\u{914d}\u{89d2}" => Some(Self::Supporting),
            "\u{8def}\u{4eba}" => Some(Self::Minor),
            "\u{7ec4}\u{7ec7}\u{89d2}\u{8272}" => Some(Self::Organization),
            _ => None,
        }
    }
}
