use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMeta {
    pub name: String,
    pub author: String,
    pub genre: String,
    pub target_words: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectStatus {
    Active,
    Archived,
}

impl ProjectMeta {
    pub fn is_valid_genre(genre: &str) -> bool {
        matches!(
            genre,
            "玄幻"
                | "都市"
                | "科幻"
                | "悬疑"
                | "言情"
                | "历史"
                | "奇幻"
                | "轻小说"
                | "剧本"
                | "其他"
        )
    }
}
