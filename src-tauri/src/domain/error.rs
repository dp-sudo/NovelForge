use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainError {
    pub code: String,
    pub message: String,
}

impl DomainError {
    pub const fn new(code: String, message: String) -> Self {
        Self { code, message }
    }
}
