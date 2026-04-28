use std::path::PathBuf;

use crate::errors::AppErrorDto;

const APP_DIR_NAME: &str = "NovelForge";
const LEGACY_APP_DIR_NAME: &str = ".novelforge";

pub fn app_data_dir() -> Result<PathBuf, AppErrorDto> {
    let resolved = resolve_base_app_data_dir()?;
    std::fs::create_dir_all(&resolved).map_err(|err| {
        AppErrorDto::new(
            "APP_DIR_CREATE_FAILED",
            "Cannot create app data directory",
            true,
        )
        .with_detail(err.to_string())
    })?;
    Ok(resolved)
}

pub fn legacy_home_app_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(LEGACY_APP_DIR_NAME))
}

fn resolve_base_app_data_dir() -> Result<PathBuf, AppErrorDto> {
    if let Ok(raw) = std::env::var("NOVELFORGE_APP_DATA_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    if let Some(local_data) = dirs::data_local_dir() {
        return Ok(local_data.join(APP_DIR_NAME));
    }

    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(LEGACY_APP_DIR_NAME));
    }

    Err(AppErrorDto::new(
        "APP_DIR_UNAVAILABLE",
        "Cannot determine app data directory",
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_data_dir_is_resolvable() {
        let resolved = app_data_dir().expect("resolve app data dir");
        assert!(!resolved.to_string_lossy().trim().is_empty());
    }
}
