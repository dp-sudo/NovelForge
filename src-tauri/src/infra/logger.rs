//! Application logging infrastructure.
//!
//! Provides structured logging helpers used by the current command/service flows.
//! Logs are written to both console (via tauri-plugin-log) and the app log file.
//!
//! Log levels used:
//! - ERROR: Unrecoverable errors, operation failures
//! - WARN:  Recoverable errors, unexpected states, degraded paths
//! - INFO:  Key lifecycle events, command invocations, state transitions

use log::{error, info, warn};

/// Log a command failure.
pub fn log_command_error(command: &str, error: &str) {
    error!("[CMD] {} | FAILED: {}", command, error);
}

/// Log a security-relevant event (API key access, auth, etc).
pub fn log_security(event: &str, detail: &str) {
    warn!("[SEC] {} | {}", event, detail);
}

/// Log startup info banner.
pub fn log_startup(version: &str) {
    info!("========================================");
    info!("  NovelForge v{} starting up", version);
    info!("========================================");
}

/// Log a user action (save, export, etc).
pub fn log_user_action(action: &str, detail: &str) {
    info!("[USER] {} | {}", action, detail);
}
