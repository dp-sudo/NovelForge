pub mod anthropic;
pub mod gemini;
pub mod llm_service;
pub mod llm_types;
pub mod openai_compatible;

pub use llm_service::LlmService;
pub use llm_types::*;

/// Build the appropriate LLM adapter for the given provider configuration.
///
/// Dispatch order:
/// 1. Explicit vendor match (`anthropic`, `minimax`, `gemini`)
/// 2. Protocol-based fallback (`anthropic_messages`, `custom_anthropic_compatible`, `gemini_generate_content`)
/// 3. Default to OpenAI-compatible
pub fn build_adapter(config: ProviderConfig) -> Box<dyn LlmService> {
    let is_anthropic_protocol = matches!(
        config.protocol.as_str(),
        "anthropic_messages" | "custom_anthropic_compatible"
    );
    let is_gemini_protocol = matches!(config.protocol.as_str(), "gemini_generate_content");

    match config.vendor.as_str() {
        "anthropic" | "minimax" => Box::new(anthropic::AnthropicAdapter::new(config)),
        "gemini" => Box::new(gemini::GeminiAdapter::new(config)),
        _ if is_anthropic_protocol => Box::new(anthropic::AnthropicAdapter::new(config)),
        _ if is_gemini_protocol => Box::new(gemini::GeminiAdapter::new(config)),
        _ => Box::new(openai_compatible::OpenAiCompatibleAdapter::new(config)),
    }
}
