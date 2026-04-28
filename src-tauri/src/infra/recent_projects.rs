use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::infra::app_paths::app_data_dir;
use crate::infra::fs_utils::write_file_atomic;
use crate::infra::time::now_iso;

const MAX_RECENT_ITEMS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProjectItem {
    pub project_path: String,
    pub opened_at: String,
}

fn recent_projects_file_path() -> io::Result<PathBuf> {
    let dir = app_data_dir().map_err(|err| io::Error::other(err.message))?;
    Ok(dir.join("recent-projects.json"))
}

fn ensure_recent_projects_file() -> io::Result<PathBuf> {
    let file_path = recent_projects_file_path()?;
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !file_path.exists() {
        write_file_atomic(&file_path, "[]")?;
    }
    Ok(file_path)
}

pub fn list_recent_projects() -> io::Result<Vec<RecentProjectItem>> {
    let file_path = ensure_recent_projects_file()?;
    let raw = fs::read_to_string(file_path)?;
    let items = match serde_json::from_str::<Vec<RecentProjectItem>>(&raw) {
        Ok(items) => items,
        Err(_) => {
            write_recent_projects(&[])?;
            return Ok(Vec::new());
        }
    };

    let total = items.len();
    let filtered: Vec<RecentProjectItem> = items
        .into_iter()
        .filter(|item| is_valid_recent_project_path(&item.project_path))
        .collect();

    if filtered.len() != total {
        write_recent_projects(&filtered)?;
    }

    Ok(filtered)
}

fn write_recent_projects(items: &[RecentProjectItem]) -> io::Result<()> {
    let file_path = ensure_recent_projects_file()?;
    let payload = serde_json::to_string_pretty(items)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    write_file_atomic(&file_path, &payload)
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

pub fn clear_recent_projects() -> io::Result<()> {
    write_recent_projects(&[])
}

fn is_valid_recent_project_path(project_path: &str) -> bool {
    let root = Path::new(project_path);
    root.join("project.json").exists() && root.join("database").join("project.sqlite").exists()
}
