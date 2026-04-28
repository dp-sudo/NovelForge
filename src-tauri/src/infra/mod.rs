//! Infrastructure helpers for local filesystem and persistence.
pub mod app_database;
pub mod credential_manager;
pub mod crypto;
pub mod database;
pub mod fs_utils;
#[allow(dead_code)]
pub mod logger {
    pub const MODULE_NAME: &str = "logger";
}
pub mod path_utils;
pub mod recent_projects;
pub mod time;
