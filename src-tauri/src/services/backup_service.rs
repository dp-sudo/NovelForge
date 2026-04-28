use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use zip::read::ZipArchive;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::errors::AppErrorDto;
use crate::infra::time::now_iso;

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
        let root = Path::new(project_root);
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
                AppErrorDto::new("BACKUP_FAILED", "读取项目文件失败", true).with_detail(e.to_string())
            })?;

            let path = entry.path();
            if path == backup_path || path == root {
                continue;
            }

            let relative = path.strip_prefix(root).unwrap_or(path);
            let name = relative.to_string_lossy().to_string().replace('\\', "/");

            if entry.file_type().is_dir() {
                let options = FileOptions::<()>::default()
                    .compression_method(CompressionMethod::Deflated);
                zip.add_directory(&name, options).map_err(|err| {
                    AppErrorDto::new("BACKUP_ZIP_FAILED", "添加目录到备份失败", true)
                        .with_detail(err.to_string())
                        .with_suggested_action("请检查备份文件完整性")
                })?;
            } else {
                let mut content = Vec::new();
                fs::File::open(path)
                    .and_then(|mut f| f.read_to_end(&mut content))
                    .map_err(|e| {
                        AppErrorDto::new("BACKUP_FAILED", "读取文件失败", true).with_detail(e.to_string())
                    })?;
                let options = FileOptions::<()>::default()
                    .compression_method(CompressionMethod::Deflated);
                zip.start_file(&name, options).map_err(|err| {
                    AppErrorDto::new("BACKUP_ZIP_FAILED", "添加文件到备份失败", true)
                        .with_detail(err.to_string())
                        .with_suggested_action("请检查备份文件完整性")
                })?;
                zip.write_all(&content).map_err(|err| {
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

    pub fn list_backups(&self, project_root: &str) -> Result<Vec<BackupResult>, AppErrorDto> {
        let backup_dir = Path::new(project_root).join("backups");
        if !backup_dir.exists() {
            return Ok(vec![]);
        }

        let mut backups: Vec<BackupResult> = Vec::new();
        let entries = fs::read_dir(&backup_dir).map_err(|e| {
            AppErrorDto::new("BACKUP_LIST_FAILED", "读取备份列表失败", true).with_detail(e.to_string())
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

    pub fn restore_backup(&self, project_root: &str, backup_path: &str) -> Result<RestoreResult, AppErrorDto> {
        let backup_file = Path::new(backup_path);
        let root = Path::new(project_root);

        let file = fs::File::open(backup_file).map_err(|e| {
            AppErrorDto::new("RESTORE_FAILED", "打开备份文件失败", true).with_detail(e.to_string())
        })?;
        let mut archive = ZipArchive::new(file).map_err(|e| {
            AppErrorDto::new("RESTORE_FAILED", "读取备份文件失败", true).with_detail(e.to_string())
        })?;

        // Validate: check project.json exists in archive
        let has_project_json = (0..archive.len()).any(|i| {
            archive.by_index(i).ok().map(|f| f.name().to_string()).unwrap_or_default() == "project.json"
        });
        if !has_project_json {
            return Err(AppErrorDto::new("RESTORE_INVALID", "备份文件不包含 project.json，不是有效的项目备份", false));
        }

        let mut restored = 0usize;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| {
                AppErrorDto::new("RESTORE_FAILED", "读取备份条目失败", true).with_detail(e.to_string())
            })?;

            let name = entry.name().to_string();
            if name.is_empty() || name.ends_with('/') {
                continue;
            }

            let target_path = root.join(&name);
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).ok();
            }

            let mut content = Vec::new();
            entry.read_to_end(&mut content).map_err(|e| {
                AppErrorDto::new("RESTORE_FAILED", "读取备份内容失败", true).with_detail(e.to_string())
            })?;

            fs::write(&target_path, &content).map_err(|e| {
                AppErrorDto::new("RESTORE_FAILED", "写入恢复文件失败", true).with_detail(e.to_string())
            })?;
            restored += 1;
        }

        Ok(RestoreResult {
            project_root: project_root.to_string(),
            files_restored: restored,
        })
    }
}
