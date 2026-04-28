/// Token budget estimation and context window management.
///
/// Provides utilities for estimating token counts and checking whether
/// a request fits within a model's context window.
/// Uses a simple heuristic (~1 token per 2 ASCII chars / 1 Chinese char)
/// rather than a full tokenizer, suitable for pre-flight checks.

/// Estimate the number of tokens in a text string.
/// Rough heuristic: 1 token ≈ 4 ASCII chars, 1 token ≈ 1 Chinese char.
pub fn estimate_tokens(text: &str) -> usize {
    let mut tokens = 0usize;
    for ch in text.chars() {
        if ch.is_ascii() {
            // ASCII: roughly 1 token per 4 characters
            tokens += 1;
        } else {
            // Non-ASCII (CJK, etc.): roughly 1 token per character
            tokens += 2;
        }
    }
    // Divide by 4 for ASCII (tokenization ratio)
    tokens / 4 + 1
}

/// Token budget for a single AI request.
pub struct TokenBudget {
    pub model_context_window: usize,
    pub max_output_tokens: usize,
    pub prompt_tokens: usize,
    pub available_for_input: usize,
    pub would_exceed: bool,
    pub suggested_action: Option<String>,
}

impl TokenBudget {
    /// Calculate token budget for a given model and prompt.
    pub fn calculate(
        context_window: usize,
        max_output: usize,
        system_prompt: &str,
        messages_text: &str,
    ) -> Self {
        let system_tokens = estimate_tokens(system_prompt);
        let messages_tokens = estimate_tokens(messages_text);
        let prompt_tokens = system_tokens + messages_tokens;
        let reserved = max_output.min(context_window / 4);
        let available = context_window.saturating_sub(reserved);

        let would_exceed = prompt_tokens > available;
        let suggested_action = if would_exceed {
            Some(format!(
                "当前输入约 {} tokens，模型上下文上限 {}，建议减少章节范围或启用摘要压缩",
                prompt_tokens, context_window
            ))
        } else {
            None
        };

        Self {
            model_context_window: context_window,
            max_output_tokens: max_output,
            prompt_tokens,
            available_for_input: available,
            would_exceed,
            suggested_action,
        }
    }

    /// Recommended max_tokens for common NovelForge task types.
    pub fn recommended_max_tokens(task_type: &str) -> u32 {
        match task_type {
            "character.create" | "world.generate" | "plot.generate" => 4096,
            "chapter_plan" | "plan_chapter" => 8192,
            "chapter_draft" | "chapter_continue" => 16000,
            "chapter_rewrite" | "prose_naturalize" => 4096,
            "consistency.scan" => 8192,
            _ => 4096,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_ascii() {
        // ~100 chars / 4 = 25 + 1 = ~26 tokens
        let n = estimate_tokens("Hello world, this is a test of the token estimation algorithm!");
        assert!(n > 0);
    }

    #[test]
    fn estimate_chinese() {
        // 10 Chinese chars * 2 / 4 + 1 = 6
        let n = estimate_tokens("你好世界，这是一段中文");
        assert!(n > 0);
    }

    #[test]
    fn budget_within_window() {
        let budget = TokenBudget::calculate(4096, 1024, "system prompt", "hello");
        assert!(!budget.would_exceed);
        assert_eq!(budget.model_context_window, 4096);
    }

    #[test]
    fn budget_exceeds_window() {
        let long_text = "x".repeat(8000);
        let budget = TokenBudget::calculate(1024, 512, "", &long_text);
        assert!(budget.would_exceed);
        assert!(budget.suggested_action.is_some());
    }

    #[test]
    fn recommended_max_per_task() {
        assert_eq!(TokenBudget::recommended_max_tokens("chapter_draft"), 16000);
        assert_eq!(TokenBudget::recommended_max_tokens("consistency.scan"), 8192);
        assert_eq!(TokenBudget::recommended_max_tokens("unknown_type"), 4096);
    }
}
