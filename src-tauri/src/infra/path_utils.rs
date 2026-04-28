use std::path::Path;

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
