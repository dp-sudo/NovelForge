use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::infra::time::now_iso;

const MAX_RECENT_ITEMS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProjectItem {
    pub project_path: String,
    pub opened_at: String,
}

fn recent_projects_file_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory is unavailable"))?;
    Ok(home.join(".novelforge").join("recent-projects.json"))
}

fn ensure_recent_projects_file() -> io::Result<PathBuf> {
    let file_path = recent_projects_file_path()?;
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !file_path.exists() {
        fs::write(&file_path, "[]")?;
    }
    Ok(file_path)
}

pub fn list_recent_projects() -> io::Result<Vec<RecentProjectItem>> {
    let file_path = ensure_recent_projects_file()?;
    let raw = fs::read_to_string(file_path)?;
    match serde_json::from_str::<Vec<RecentProjectItem>>(&raw) {
        Ok(items) => Ok(items),
        Err(_) => Ok(Vec::new()),
    }
}

fn write_recent_projects(items: &[RecentProjectItem]) -> io::Result<()> {
    let file_path = ensure_recent_projects_file()?;
    let temp_path = file_path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let payload = serde_json::to_string_pretty(items)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    fs::write(&temp_path, payload)?;
    match fs::rename(&temp_path, &file_path) {
        Ok(_) => Ok(()),
        Err(_) => {
            let _ = fs::remove_file(&file_path);
            fs::rename(temp_path, file_path)
        }
    }
}

pub fn mark_recent_project(project_path: &str) -> io::Result<()> {
    let mut items = list_recent_projects()?;
    items.retain(|item| item.project_path != project_path);
    items.insert(
        0,
        RecentProjectItem {
            project_path: project_path.to_string(),
            opened_at: now_iso(),
        },
    );
    items.truncate(MAX_RECENT_ITEMS);
    write_recent_projects(&items)
}
