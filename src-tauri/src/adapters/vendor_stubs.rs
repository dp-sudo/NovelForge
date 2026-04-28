//! Per-vendor type aliases.
//!
//! DeepSeek, Kimi, Zhipu AI (智谱) use OpenAI-compatible chat completion APIs.
//! MiniMax uses Anthropic Messages API (per spec §9).
//! Anthropic and Gemini have custom protocol adapters in their own modules.

pub use super::anthropic::AnthropicAdapter as MiniMaxAdapter;
pub use super::openai_compatible::OpenAiCompatibleAdapter as DeepSeekAdapter;
pub use super::openai_compatible::OpenAiCompatibleAdapter as KimiAdapter;
pub use super::openai_compatible::OpenAiCompatibleAdapter as ZhipuAdapter;
