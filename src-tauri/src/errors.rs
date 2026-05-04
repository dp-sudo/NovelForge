use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
    pub recoverable: bool,
    pub suggested_action: Option<String>,
}

impl AppErrorDto {
    pub fn new(code: &str, message: &str, recoverable: bool) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            detail: None,
            recoverable,
            suggested_action: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_suggested_action(mut self, suggested_action: impl Into<String>) -> Self {
        self.suggested_action = Some(suggested_action.into());
        self
    }
}

impl From<rusqlite::Error> for AppErrorDto {
    fn from(e: rusqlite::Error) -> Self {
        AppErrorDto::new("DB_ERROR", "数据库操作失败", true).with_detail(e.to_string())
    }
}
