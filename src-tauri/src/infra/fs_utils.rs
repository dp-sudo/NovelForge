use std::fs;
use std::io;
use std::path::Path;

use uuid::Uuid;

pub fn write_file_atomic(target_path: &Path, content: &str) -> io::Result<()> {
    let parent = target_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid target path: {}", target_path.to_string_lossy()),
        )
    })?;
    fs::create_dir_all(parent)?;

    let file_name = target_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "target file name is missing")
        })?;
    let temp_path = parent.join(format!("{file_name}.{}.tmp", Uuid::new_v4()));

    fs::write(&temp_path, content)?;
    match fs::rename(&temp_path, target_path) {
        Ok(_) => Ok(()),
        Err(_) => {
            let _ = fs::remove_file(target_path);
            fs::rename(temp_path, target_path)
        }
    }
}

pub fn read_text_if_exists(file_path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(file_path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}
