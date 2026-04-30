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
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeprecatedCommandUsageEntry {
    pub command: String,
    pub source: String,
    pub count: u64,
}

type DeprecatedUsageMap = BTreeMap<(String, String), u64>;

static DEPRECATED_COMMAND_USAGE: OnceLock<Mutex<DeprecatedUsageMap>> = OnceLock::new();

fn deprecated_usage_store() -> &'static Mutex<DeprecatedUsageMap> {
    DEPRECATED_COMMAND_USAGE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

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

/// Record compatibility-only command usage and keep an in-memory aggregate.
pub fn record_deprecated_command_usage(command: &str, source: &str) {
    let store = deprecated_usage_store();
    let mut guard = match store.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let key = (command.to_string(), source.to_string());
    let entry = guard.entry(key).or_insert(0);
    *entry += 1;
    info!(
        "[DEPRECATED_COMMAND_USAGE] command={} source={} count={}",
        command, source, *entry
    );
}

/// Snapshot deprecated command usage report grouped by command and source.
pub fn read_deprecated_command_usage() -> Vec<DeprecatedCommandUsageEntry> {
    let store = deprecated_usage_store();
    let guard = match store.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    guard
        .iter()
        .map(|((command, source), count)| DeprecatedCommandUsageEntry {
            command: command.clone(),
            source: source.clone(),
            count: *count,
        })
        .collect()
}
