use std::fs;
use std::io;
use std::path::Path;

use uuid::Uuid;

pub fn write_file_atomic(target_path: &Path, content: &str) -> io::Result<()> {
    write_bytes_atomic(target_path, content.as_bytes())
}

pub fn write_bytes_atomic(target_path: &Path, content: &[u8]) -> io::Result<()> {
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
        Err(err) => {
            let _ = fs::remove_file(&temp_path);
            Err(err)
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

#[cfg(test)]
mod tests {
    use super::write_file_atomic;
    use std::fs;
    use std::path::PathBuf;
    use std::thread;
    use uuid::Uuid;

    fn create_temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("novelforge-fs-utils-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn concurrent_atomic_writes_keep_file_content_consistent() {
        let dir = create_temp_dir();
        let target = dir.join("atomic.txt");
        let mut handles = Vec::new();
        for index in 0..16 {
            let target_path = target.clone();
            handles.push(thread::spawn(move || {
                let payload = format!("payload-{index}-{}", "x".repeat(2048));
                write_file_atomic(&target_path, &payload).expect("atomic write");
                payload
            }));
        }

        let expected_payloads = handles
            .into_iter()
            .map(|handle| handle.join().expect("join thread"))
            .collect::<Vec<_>>();
        let final_content = fs::read_to_string(&target).expect("read final file");
        assert!(
            expected_payloads
                .iter()
                .any(|candidate| candidate == &final_content),
            "final content should match one completed write",
        );

        let _ = fs::remove_dir_all(dir);
    }
}
