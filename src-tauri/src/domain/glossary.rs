use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TermType {
    PersonName,
    PlaceName,
    OrganizationName,
    Term,
    Alias,
    BannedWord,
}

impl TermType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PersonName => "\u{4eba}\u{540d}",
            Self::PlaceName => "\u{5730}\u{540d}",
            Self::OrganizationName => "\u{7ec4}\u{7ec7}\u{540d}",
            Self::Term => "\u{672f}\u{8bed}",
            Self::Alias => "\u{522b}\u{540d}",
            Self::BannedWord => "\u{7981}\u{7528}\u{8bcd}",
        }
    }
}
