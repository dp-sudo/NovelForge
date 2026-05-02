use std::fs;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use zip::read::ZipArchive;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::errors::AppErrorDto;
use crate::infra::fs_utils::write_bytes_atomic;
use crate::infra::path_utils::resolve_project_relative_path;
use crate::infra::time::now_iso;
use crate::services::project_service::ProjectJson;

const MAX_RESTORE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub file_path: String,
    pub file_size: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub project_root: String,
    pub files_restored: usize,
}

#[derive(Default)]
pub struct BackupService;

impl BackupService {
    pub fn create_backup(&self, project_root: &str) -> Result<BackupResult, AppErrorDto> {
        let normalized_root = normalize_project_root(project_root)?;
        let root = Path::new(&normalized_root);
        let now = now_iso();
        let safe_now = now.replace([':', '.'], "-");
        let backup_dir = root.join("backups");
        fs::create_dir_all(&backup_dir).map_err(|e| {
            AppErrorDto::new("BACKUP_FAILED", "创建备份目录失败", true).with_detail(e.to_string())
        })?;

        let backup_path = backup_dir.join(format!("{}_novelforge-backup.zip", safe_now));
        let file = fs::File::create(&backup_path).map_err(|e| {
            AppErrorDto::new("BACKUP_FAILED", "创建备份文件失败", true).with_detail(e.to_string())
        })?;
        let mut zip = ZipWriter::new(file);

        let skip_dirs: [&str; 4] = ["logs", "node_modules", "backups", ".git"];

        for entry in WalkDir::new(root).into_iter().filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                !skip_dirs.contains(&name)
            } else {
                true
            }
        }) {
            let entry = entry.map_err(|e| {
                AppErrorDto::new("BACKUP_FAILED", "读取项目文件失败", true)
                    .with_detail(e.to_string())
            })?;

            let path = entry.path();
            if path == backup_path || path == root {
                continue;
            }

            let relative = path.strip_prefix(root).unwrap_or(path);
            let name = relative.to_string_lossy().to_string().replace('\\', "/");

            if entry.file_type().is_dir() {
                let options =
                    FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);
                zip.add_directory(&name, options).map_err(|err| {
                    AppErrorDto::new("BACKUP_ZIP_FAILED", "添加目录到备份失败", true)
                        .with_detail(err.to_string())
                        .with_suggested_action("请检查备份文件完整性")
                })?;
            } else {
                let options =
                    FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);
                zip.start_file(&name, options).map_err(|err| {
                    AppErrorDto::new("BACKUP_ZIP_FAILED", "添加文件到备份失败", true)
                        .with_detail(err.to_string())
                        .with_suggested_action("请检查备份文件完整性")
                })?;
                let mut source = fs::File::open(path).map_err(|e| {
                    AppErrorDto::new("BACKUP_FAILED", "读取文件失败", true)
                        .with_detail(e.to_string())
                })?;
                std::io::copy(&mut source, &mut zip).map_err(|err| {
                    AppErrorDto::new("BACKUP_ZIP_WRITE_FAILED", "写入备份文件失败", true)
                        .with_detail(err.to_string())
                        .with_suggested_action("请检查备份文件写入权限")
                })?;
            }
        }

        let file = zip.finish().map_err(|e| {
            AppErrorDto::new("BACKUP_FAILED", "完成备份文件失败", true).with_detail(e.to_string())
        })?;

        let file_size = file.metadata().map(|m| m.len() as i64).unwrap_or(0);

        Ok(BackupResult {
            file_path: backup_path.to_string_lossy().into(),
            file_size,
            created_at: now,
        })
    }

    /// Check if a daily backup already exists, and create one if not.
    /// This is a "best effort" operation — failures are logged but not returned,
    /// so it never blocks project opening.
    pub fn try_auto_backup(&self, project_root: &str) {
        let today = crate::infra::time::today_date_str();

        // Check if any existing backup filename starts with today's date
        let existing = match self.list_backups(project_root) {
            Ok(list) => list,
            Err(_) => {
                log::warn!("[AUTO_BACKUP] Cannot list backups for {}", project_root);
                return;
            }
        };

        let already_backed_up = existing.iter().any(|b| b.file_path.contains(&today));

        if already_backed_up {
            log::info!("[AUTO_BACKUP] Daily backup already exists for {}", today);
            return;
        }

        match self.create_backup(project_root) {
            Ok(result) => {
                log::info!(
                    "[AUTO_BACKUP] Created daily backup: {} ({} bytes)",
                    result.file_path,
                    result.file_size
                );
            }
            Err(e) => {
                log::warn!("[AUTO_BACKUP] Failed to create daily backup: {}", e.message);
            }
        }
    }

    pub fn list_backups(&self, project_root: &str) -> Result<Vec<BackupResult>, AppErrorDto> {
        let normalized_root = normalize_project_root(project_root)?;
        let backup_dir = Path::new(&normalized_root).join("backups");
        if !backup_dir.exists() {
            return Ok(vec![]);
        }

        let mut backups: Vec<BackupResult> = Vec::new();
        let entries = fs::read_dir(&backup_dir).map_err(|e| {
            AppErrorDto::new("BACKUP_LIST_FAILED", "读取备份列表失败", true)
                .with_detail(e.to_string())
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("zip") {
                let metadata = fs::metadata(&path).ok();
                backups.push(BackupResult {
                    file_path: path.to_string_lossy().into(),
                    file_size: metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0),
                    created_at: metadata
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Utc> = t.into();
                            dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                        })
                        .unwrap_or_else(now_iso),
                });
            }
        }

        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(backups)
    }

    pub fn restore_backup(
        &self,
        project_root: &str,
        backup_path: &str,
    ) -> Result<RestoreResult, AppErrorDto> {
        let normalized_root = normalize_project_root(project_root)?;
        let backup_file = normalize_backup_file_path(backup_path)?;
        let root = Path::new(&normalized_root);
        let current_project = read_project_json(root)?;
        let backup_size = fs::metadata(&backup_file)
            .map(|meta| meta.len())
            .map_err(|e| {
                AppErrorDto::new("RESTORE_FAILED", "读取备份文件信息失败", true)
                    .with_detail(e.to_string())
            })?;
        if backup_size > MAX_RESTORE_BYTES {
            return Err(AppErrorDto::new(
                "BACKUP_TOO_LARGE",
                "备份文件体积超出限制，已拒绝恢复",
                true,
            )
            .with_detail(format!(
                "backupSizeBytes={}, maxAllowedBytes={}",
                backup_size, MAX_RESTORE_BYTES
            ))
            .with_suggested_action("请选择小于 100MB 的备份文件后重试"));
        }

        let file = fs::File::open(&backup_file).map_err(|e| {
            AppErrorDto::new("RESTORE_FAILED", "打开备份文件失败", true).with_detail(e.to_string())
        })?;
        let mut archive = ZipArchive::new(file).map_err(|e| {
            AppErrorDto::new("RESTORE_FAILED", "读取备份文件失败", true).with_detail(e.to_string())
        })?;

        let archived_project = read_archived_project_json(&mut archive)?;
        if archived_project.project_id != current_project.project_id
            || archived_project.schema_version != current_project.schema_version
        {
            return Err(
                AppErrorDto::new("RESTORE_PROJECT_MISMATCH", "备份文件与当前项目不匹配", false)
                    .with_detail(format!(
                        "archiveProjectId={}, currentProjectId={}, archiveSchemaVersion={}, currentSchemaVersion={}",
                        archived_project.project_id,
                        current_project.project_id,
                        archived_project.schema_version,
                        current_project.schema_version,
                    ))
                    .with_suggested_action("请选择当前项目生成的备份文件后重试"),
            );
        }

        let mut restored = 0usize;
        let mut total_uncompressed: u64 = 0;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| {
                AppErrorDto::new("RESTORE_FAILED", "读取备份条目失败", true)
                    .with_detail(e.to_string())
            })?;

            let name = entry.name().to_string();
            if name.is_empty() || name.ends_with('/') {
                continue;
            }

            let target_path = resolve_project_relative_path(root, &name).map_err(|detail| {
                AppErrorDto::new(
                    "RESTORE_INVALID_PATH",
                    "备份条目路径非法，已拒绝恢复",
                    false,
                )
                .with_detail(detail)
            })?;
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    AppErrorDto::new("RESTORE_FAILED", "创建恢复目录失败", true)
                        .with_detail(e.to_string())
                })?;
            }
            let entry_size = entry.size();
            if entry_size > MAX_RESTORE_BYTES
                || total_uncompressed.saturating_add(entry_size) > MAX_RESTORE_BYTES
            {
                return Err(AppErrorDto::new(
                    "BACKUP_TOO_LARGE",
                    "备份内容体积超出限制，已拒绝恢复",
                    true,
                )
                .with_detail(format!(
                    "entry={}, entrySizeBytes={}, accumulatedBytes={}, maxAllowedBytes={}",
                    name, entry_size, total_uncompressed, MAX_RESTORE_BYTES
                ))
                .with_suggested_action("请选择体积更小的备份文件后重试"));
            }

            let mut content = Vec::new();
            entry.read_to_end(&mut content).map_err(|e| {
                AppErrorDto::new("RESTORE_FAILED", "读取备份内容失败", true)
                    .with_detail(e.to_string())
            })?;
            total_uncompressed = total_uncompressed.saturating_add(content.len() as u64);

            write_bytes_atomic(&target_path, &content).map_err(|e| {
                AppErrorDto::new("RESTORE_FAILED", "写入恢复文件失败", true)
                    .with_detail(e.to_string())
            })?;
            restored += 1;
        }

        Ok(RestoreResult {
            project_root: normalized_root,
            files_restored: restored,
        })
    }
}

fn read_project_json(project_root: &Path) -> Result<ProjectJson, AppErrorDto> {
    let path = project_root.join("project.json");
    let payload = fs::read_to_string(&path).map_err(|err| {
        AppErrorDto::new("RESTORE_INVALID", "无法读取当前项目元数据", false)
            .with_detail(err.to_string())
    })?;
    serde_json::from_str::<ProjectJson>(&payload).map_err(|err| {
        AppErrorDto::new("RESTORE_INVALID", "当前项目元数据损坏", false)
            .with_detail(err.to_string())
    })
}

fn read_archived_project_json<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<ProjectJson, AppErrorDto> {
    let mut project_json = archive.by_name("project.json").map_err(|_| {
        AppErrorDto::new(
            "RESTORE_INVALID",
            "备份文件不包含 project.json，不是有效的项目备份",
            false,
        )
    })?;
    let mut payload = String::new();
    project_json.read_to_string(&mut payload).map_err(|err| {
        AppErrorDto::new("RESTORE_INVALID", "无法读取备份项目元数据", false)
            .with_detail(err.to_string())
    })?;
    serde_json::from_str::<ProjectJson>(&payload).map_err(|err| {
        AppErrorDto::new("RESTORE_INVALID", "备份项目元数据损坏", false)
            .with_detail(err.to_string())
    })
}

fn normalize_project_root(project_root: &str) -> Result<String, AppErrorDto> {
    let normalized = project_root.trim();
    if normalized.is_empty() {
        return Err(
            AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录不能为空", true)
                .with_suggested_action("请输入有效的项目目录路径"),
        );
    }

    let root = Path::new(normalized);
    if !root.is_absolute() {
        return Err(
            AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录必须是绝对路径", true)
                .with_suggested_action("请输入有效的 Windows 绝对路径"),
        );
    }
    if !root.exists() || !root.is_dir() {
        return Err(
            AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录不存在或不可用", true)
                .with_detail(normalized.to_string())
                .with_suggested_action("请检查目录路径并重试"),
        );
    }

    Ok(normalized.to_string())
}

fn normalize_backup_file_path(backup_path: &str) -> Result<String, AppErrorDto> {
    let normalized = backup_path.trim();
    if normalized.is_empty() {
        return Err(
            AppErrorDto::new("RESTORE_INVALID", "备份文件路径不能为空", true)
                .with_suggested_action("请选择有效的备份文件"),
        );
    }

    let path = Path::new(normalized);
    if !path.is_absolute() {
        return Err(
            AppErrorDto::new("RESTORE_INVALID", "备份文件路径必须是绝对路径", true)
                .with_suggested_action("请选择有效的备份文件"),
        );
    }
    if !path.exists() || !path.is_file() {
        return Err(
            AppErrorDto::new("RESTORE_INVALID", "备份文件不存在或不可用", true)
                .with_detail(normalized.to_string())
                .with_suggested_action("请检查备份文件路径并重试"),
        );
    }

    Ok(normalized.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    use uuid::Uuid;
    use zip::write::FileOptions;
    use zip::CompressionMethod;
    use zip::ZipWriter;

    use super::BackupService;
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-backup-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn restore_rejects_zip_slip_entries() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "恶意恢复".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project root");
        let project_root = PathBuf::from(&project.project_root);
        let project_json =
            fs::read_to_string(project_root.join("project.json")).expect("read project json");

        let backup_path = workspace.join("malicious.zip");
        let file = fs::File::create(&backup_path).expect("create backup file");
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("project.json", options)
            .expect("start project.json");
        zip.write_all(project_json.as_bytes())
            .expect("write project json");
        zip.start_file("../outside.txt", options)
            .expect("start malicious entry");
        zip.write_all(b"should never be restored")
            .expect("write malicious entry");
        zip.finish().expect("finish zip");

        let outside_path = workspace.join("outside.txt");
        let service = BackupService;
        let err = service
            .restore_backup(
                project.project_root.as_str(),
                backup_path.to_string_lossy().as_ref(),
            )
            .expect_err("zip slip should be rejected");

        assert_eq!(err.code, "RESTORE_INVALID_PATH");
        assert!(!outside_path.exists());

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn create_backup_rejects_blank_project_root() {
        let service = BackupService;
        let err = service
            .create_backup("   ")
            .expect_err("blank project path should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }

    #[test]
    fn restore_rejects_blank_backup_path() {
        let workspace = create_temp_workspace();
        let project_root = workspace.join("project");
        fs::create_dir_all(&project_root).expect("create project root");

        let service = BackupService;
        let err = service
            .restore_backup(project_root.to_string_lossy().as_ref(), "   ")
            .expect_err("blank backup path should be rejected");
        assert_eq!(err.code, "RESTORE_INVALID");

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn restore_rejects_backup_from_different_project() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let backup_service = BackupService;

        let project_a = project_service
            .create_project(CreateProjectInput {
                name: "备份A".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project a");
        let project_b = project_service
            .create_project(CreateProjectInput {
                name: "备份B".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project b");

        let backup = backup_service
            .create_backup(&project_a.project_root)
            .expect("create backup");

        let err = backup_service
            .restore_backup(&project_b.project_root, &backup.file_path)
            .expect_err("cross-project restore should be rejected");
        assert_eq!(err.code, "RESTORE_PROJECT_MISMATCH");

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn restore_rejects_oversized_backup_file() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "超大备份".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project");

        let backup_path = workspace.join("oversized.zip");
        let file = fs::File::create(&backup_path).expect("create oversized backup");
        file.set_len(200_u64 * 1024 * 1024)
            .expect("set oversized file length");

        let service = BackupService;
        let err = service
            .restore_backup(
                project.project_root.as_str(),
                backup_path.to_string_lossy().as_ref(),
            )
            .expect_err("oversized backup should be rejected");
        assert_eq!(err.code, "BACKUP_TOO_LARGE");

        remove_temp_workspace(&workspace);
    }
}
