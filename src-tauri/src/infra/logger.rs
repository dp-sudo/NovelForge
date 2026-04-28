//! Application logging infrastructure.
//!
//! Provides helper macros for consistent structured logging across all services.
//! Logs are written to both console (via tauri-plugin-log) and the app log file.
//!
//! Log levels used:
//! - ERROR: Unrecoverable errors, operation failures
//! - WARN:  Recoverable errors, unexpected states, degraded paths
//! - INFO:  Key lifecycle events, command invocations, state transitions
//! - DEBUG: Detailed operation tracing, request/response summaries

use log::{debug, error, info, warn};

/// Log a command invocation with its input summary.
pub fn log_command(command: &str, input_summary: &str) {
    info!("[CMD] {} | input: {}", command, input_summary);
}

/// Log a command completion with its result summary.
pub fn log_command_result(command: &str, result_summary: &str) {
    info!("[CMD] {} | result: {}", command, result_summary);
}

/// Log a command failure.
pub fn log_command_error(command: &str, error: &str) {
    error!("[CMD] {} | FAILED: {}", command, error);
}

/// Log a service operation.
pub fn log_service(service: &str, operation: &str, detail: &str) {
    debug!("[SVC] {}.{} | {}", service, operation, detail);
}

/// Log an AI provider call (never includes API key or raw content).
pub fn log_ai_call(provider: &str, model: &str, task: &str, tokens_estimate: Option<u32>) {
    let tokens = tokens_estimate
        .map(|t| format!(", ~{}toks", t))
        .unwrap_or_default();
    info!(
        "[AI] {} / {} | {}{}",
        provider, model, task, tokens
    );
}

/// Log a database operation.
pub fn log_db(operation: &str, table: &str, detail: &str) {
    debug!("[DB] {}({}) | {}", operation, table, detail);
}

/// Log a security-relevant event (API key access, auth, etc).
pub fn log_security(event: &str, detail: &str) {
    warn!("[SEC] {} | {}", event, detail);
}

/// Log a filesystem operation.
pub fn log_fs(operation: &str, path: &str, detail: &str) {
    debug!("[FS] {} | {}{}", operation, path,
        if detail.is_empty() { String::new() } else { format!(" | {}", detail) }
    );
}

/// Log startup info banner.
pub fn log_startup(version: &str) {
    info!("========================================");
    info!("  NovelForge v{} starting up", version);
    info!("========================================");
}

/// Log a route/page navigation event.
pub fn log_navigation(page: &str) {
    info!("[NAV] -> {}", page);
}

/// Log a user action (save, export, etc).
pub fn log_user_action(action: &str, detail: &str) {
    info!("[USER] {} | {}", action, detail);
}
