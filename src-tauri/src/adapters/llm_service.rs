use async_trait::async_trait;
use tokio::sync::mpsc;

use super::llm_types::*;

#[async_trait]
pub trait LlmService: Send + Sync {
    async fn generate_text(&self, req: UnifiedGenerateRequest) -> Result<UnifiedGenerateResponse, LlmError>;

    async fn stream_text(
        &self,
        req: UnifiedGenerateRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<(), LlmError>;

    async fn test_connection(&self) -> Result<(), LlmError>;

    /// Probe the provider endpoint to detect supported capabilities.
    /// Returns a list of capability flags determined through live API calls.
    async fn detect_capabilities(&self) -> Result<CapabilityReport, LlmError>;

    /// Fetch available model names from the provider (e.g. GET /v1/models).
    async fn fetch_models(&self) -> Result<Vec<String>, LlmError>;
}
