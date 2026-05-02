use std::fs;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::errors::AppErrorDto;
use crate::infra::time::now_iso;

const DEFAULT_GITIGNORE: [&str; 6] = [
    "backups/",
    "logs/",
    "exports/",
    "database/backups/",
    "node_modules/",
    "target/",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryStatus {
    pub initialized: bool,
    pub branch: String,
    pub has_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitRecord {
    pub commit_id: String,
    pub summary: String,
    pub committed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshotResult {
    pub no_changes: bool,
    pub commit: Option<GitCommitRecord>,
}

#[derive(Default)]
pub struct GitService;

impl GitService {
    pub fn init_repository(&self, project_root: &str) -> Result<GitRepositoryStatus, AppErrorDto> {
        let normalized_root = normalize_project_root(project_root)?;
        let root = Path::new(&normalized_root);

        if !root.join(".git").exists() {
            run_git(&normalized_root, &["init"])?;
        }
        self.ensure_local_identity(&normalized_root)?;
        self.ensure_gitignore(&normalized_root)?;
        self.read_status(&normalized_root)
    }

    pub fn read_status(&self, project_root: &str) -> Result<GitRepositoryStatus, AppErrorDto> {
        let normalized_root = normalize_project_root(project_root)?;
        let root = Path::new(&normalized_root);
        let initialized = root.join(".git").exists();
        if !initialized {
            return Ok(GitRepositoryStatus {
                initialized: false,
                branch: "N/A".to_string(),
                has_changes: false,
            });
        }

        let branch = run_git(&normalized_root, &["branch", "--show-current"])
            .unwrap_or_default()
            .trim()
            .to_string();
        let branch = if branch.is_empty() {
            "main".to_string()
        } else {
            branch
        };
        let has_changes = !run_git(&normalized_root, &["status", "--porcelain"])?
            .trim()
            .is_empty();

        Ok(GitRepositoryStatus {
            initialized: true,
            branch,
            has_changes,
        })
    }

    pub fn commit_snapshot(
        &self,
        project_root: &str,
        message: Option<String>,
    ) -> Result<GitSnapshotResult, AppErrorDto> {
        let normalized_root = normalize_project_root(project_root)?;
        self.init_repository(&normalized_root)?;
        run_git(&normalized_root, &["add", "-A"])?;

        let status = run_git(&normalized_root, &["status", "--porcelain"])?;
        if status.trim().is_empty() {
            let latest = self.read_latest_commit(&normalized_root).ok();
            return Ok(GitSnapshotResult {
                no_changes: true,
                commit: latest,
            });
        }

        let commit_message = message
            .map(|msg| msg.trim().to_string())
            .filter(|msg| !msg.is_empty())
            .unwrap_or_else(|| format!("NovelForge snapshot {}", now_iso()));
        run_git(&normalized_root, &["commit", "-m", commit_message.as_str()])?;

        Ok(GitSnapshotResult {
            no_changes: false,
            commit: Some(self.read_latest_commit(&normalized_root)?),
        })
    }

    pub fn list_history(
        &self,
        project_root: &str,
        limit: usize,
    ) -> Result<Vec<GitCommitRecord>, AppErrorDto> {
        let normalized_root = normalize_project_root(project_root)?;
        let cap = limit.clamp(1, 100);
        if !Path::new(&normalized_root).join(".git").exists() {
            return Ok(Vec::new());
        }
        let output = run_git(
            &normalized_root,
            &[
                "log",
                "--pretty=format:%H%x09%s%x09%cI",
                "-n",
                &cap.to_string(),
            ],
        )?;
        Ok(output
            .lines()
            .filter_map(|line| parse_log_line(line.trim()))
            .collect())
    }

    fn read_latest_commit(&self, project_root: &str) -> Result<GitCommitRecord, AppErrorDto> {
        let output = run_git(
            project_root,
            &["log", "--pretty=format:%H%x09%s%x09%cI", "-n", "1"],
        )?;
        output
            .lines()
            .find_map(|line| parse_log_line(line.trim()))
            .ok_or_else(|| AppErrorDto::new("GIT_LOG_PARSE_FAILED", "无法解析 Git 提交记录", true))
    }

    fn ensure_local_identity(&self, project_root: &str) -> Result<(), AppErrorDto> {
        let user_name =
            run_git(project_root, &["config", "--get", "user.name"]).unwrap_or_default();
        if user_name.trim().is_empty() {
            run_git(project_root, &["config", "user.name", "NovelForge"])?;
        }
        let user_email =
            run_git(project_root, &["config", "--get", "user.email"]).unwrap_or_default();
        if user_email.trim().is_empty() {
            run_git(
                project_root,
                &["config", "user.email", "novelforge@local.invalid"],
            )?;
        }
        Ok(())
    }

    fn ensure_gitignore(&self, project_root: &str) -> Result<(), AppErrorDto> {
        let path = Path::new(project_root).join(".gitignore");
        let mut existing = fs::read_to_string(&path).unwrap_or_default();
        let mut changed = false;
        for entry in DEFAULT_GITIGNORE {
            if !existing.lines().any(|line| line.trim() == entry) {
                if !existing.is_empty() && !existing.ends_with('\n') {
                    existing.push('\n');
                }
                existing.push_str(entry);
                existing.push('\n');
                changed = true;
            }
        }
        if changed {
            fs::write(&path, existing).map_err(|err| {
                AppErrorDto::new("GIT_INIT_FAILED", "无法更新 .gitignore", true)
                    .with_detail(err.to_string())
            })?;
        }
        Ok(())
    }
}

fn run_git(project_root: &str, args: &[&str]) -> Result<String, AppErrorDto> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AppErrorDto::new("GIT_NOT_INSTALLED", "当前系统未安装 Git 可执行程序", true)
                    .with_detail(err.to_string())
            } else {
                AppErrorDto::new("GIT_COMMAND_FAILED", "执行 Git 命令失败", true)
                    .with_detail(err.to_string())
            }
        })?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let detail = if stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        stderr
    };
    Err(
        AppErrorDto::new("GIT_COMMAND_FAILED", "Git 命令返回非零退出状态", true)
            .with_detail(format!("git {}: {}", args.join(" "), detail)),
    )
}

fn parse_log_line(line: &str) -> Option<GitCommitRecord> {
    let mut parts = line.splitn(3, '\t');
    let commit_id = parts.next()?.trim();
    let summary = parts.next()?.trim();
    let committed_at = parts.next()?.trim();
    if commit_id.is_empty() || summary.is_empty() || committed_at.is_empty() {
        return None;
    }
    Some(GitCommitRecord {
        commit_id: commit_id.to_string(),
        summary: summary.to_string(),
        committed_at: committed_at.to_string(),
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    use uuid::Uuid;

    use crate::services::project_service::{CreateProjectInput, ProjectService};

    use super::GitService;

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-git-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn init_commit_and_history_succeeds() {
        if !git_available() {
            return;
        }

        let workspace = create_temp_workspace();
        let project = ProjectService
            .create_project(CreateProjectInput {
                name: "Git 集成测试".to_string(),
                author: None,
                genre: "悬疑".to_string(),
                target_words: Some(50000),
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project");
        let service = GitService;

        let status = service
            .init_repository(&project.project_root)
            .expect("init repository");
        assert!(status.initialized);

        let first_commit = service
            .commit_snapshot(&project.project_root, Some("Initial snapshot".to_string()))
            .expect("create first snapshot");
        assert!(!first_commit.no_changes);
        assert!(first_commit.commit.is_some());

        let history = service
            .list_history(&project.project_root, 10)
            .expect("list history");
        assert!(!history.is_empty());

        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn read_status_rejects_blank_project_root() {
        let service = GitService;
        let err = service
            .read_status("   ")
            .expect_err("blank path should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }

    #[test]
    fn read_status_rejects_relative_project_root() {
        let service = GitService;
        let err = service
            .read_status("relative\\project")
            .expect_err("relative path should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
