use std::path::{Component, Path, PathBuf};

pub fn sanitize_project_directory_name(name: &str) -> String {
    let trimmed = name.trim();
    let mut no_illegal_chars = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            no_illegal_chars.push('_');
        } else {
            no_illegal_chars.push(ch);
        }
    }

    let collapsed = no_illegal_chars
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.is_empty() {
        "novelforge-project".to_string()
    } else {
        collapsed
    }
}

pub fn chapter_file_name(index: i64) -> String {
    format!("ch-{index:04}.md")
}

pub fn to_posix_relative(from_dir: &Path, target: &Path) -> String {
    match target.strip_prefix(from_dir) {
        Ok(relative) => {
            let mut parts = Vec::new();
            for part in relative.components() {
                parts.push(part.as_os_str().to_string_lossy().into_owned());
            }
            parts.join("/")
        }
        Err(_) => target.to_string_lossy().replace('\\', "/"),
    }
}

pub fn resolve_project_relative_path(
    project_root: &Path,
    stored_path: &str,
) -> Result<PathBuf, String> {
    let normalized = stored_path.trim();
    if normalized.is_empty() {
        return Err("Path is empty".to_string());
    }

    let candidate = Path::new(normalized);
    if candidate.is_absolute() {
        return Err(format!("Path must be relative: {normalized}"));
    }

    for component in candidate.components() {
        match component {
            Component::CurDir | Component::Normal(_) => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Path escapes project root: {normalized}"));
            }
        }
    }

    Ok(project_root.join(candidate))
}
