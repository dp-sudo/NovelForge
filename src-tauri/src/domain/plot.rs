use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Beginning,
    Turn,
    Conflict,
    Failure,
    Victory,
    Climax,
    Ending,
    SideStory,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Beginning => "\u{5f00}\u{7aef}",
            Self::Turn => "\u{8f6c}\u{6298}",
            Self::Conflict => "\u{51b2}\u{7a81}",
            Self::Failure => "\u{5931}\u{8d25}",
            Self::Victory => "\u{80dc}\u{5229}",
            Self::Climax => "\u{9ad8}\u{6f6e}",
            Self::Ending => "\u{7ed3}\u{5c40}",
            Self::SideStory => "\u{652f}\u{7ebf}",
        }
    }
}
