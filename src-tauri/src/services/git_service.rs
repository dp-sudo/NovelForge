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
        let root = Path::new(project_root);
        if !root.exists() {
            return Err(
                AppErrorDto::new("PROJECT_PATH_NOT_FOUND", "Project path does not exist", true)
                    .with_detail(project_root.to_string()),
            );
        }

        if !root.join(".git").exists() {
            run_git(project_root, &["init"])?;
        }
        self.ensure_local_identity(project_root)?;
        self.ensure_gitignore(project_root)?;
        self.read_status(project_root)
    }

    pub fn read_status(&self, project_root: &str) -> Result<GitRepositoryStatus, AppErrorDto> {
        let root = Path::new(project_root);
        let initialized = root.join(".git").exists();
        if !initialized {
            return Ok(GitRepositoryStatus {
                initialized: false,
                branch: "N/A".to_string(),
                has_changes: false,
            });
        }

        let branch = run_git(project_root, &["branch", "--show-current"])
            .unwrap_or_default()
            .trim()
            .to_string();
        let branch = if branch.is_empty() {
            "main".to_string()
        } else {
            branch
        };
        let has_changes = !run_git(project_root, &["status", "--porcelain"])?
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
        self.init_repository(project_root)?;
        run_git(project_root, &["add", "-A"])?;

        let status = run_git(project_root, &["status", "--porcelain"])?;
        if status.trim().is_empty() {
            let latest = self.read_latest_commit(project_root).ok();
            return Ok(GitSnapshotResult {
                no_changes: true,
                commit: latest,
            });
        }

        let commit_message = message
            .map(|msg| msg.trim().to_string())
            .filter(|msg| !msg.is_empty())
            .unwrap_or_else(|| format!("NovelForge snapshot {}", now_iso()));
        run_git(project_root, &["commit", "-m", commit_message.as_str()])?;

        Ok(GitSnapshotResult {
            no_changes: false,
            commit: Some(self.read_latest_commit(project_root)?),
        })
    }

    pub fn list_history(
        &self,
        project_root: &str,
        limit: usize,
    ) -> Result<Vec<GitCommitRecord>, AppErrorDto> {
        let cap = limit.clamp(1, 100);
        if !Path::new(project_root).join(".git").exists() {
            return Ok(Vec::new());
        }
        let output = run_git(
            project_root,
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
        let output = run_git(project_root, &["log", "--pretty=format:%H%x09%s%x09%cI", "-n", "1"])?;
        output
            .lines()
            .find_map(|line| parse_log_line(line.trim()))
            .ok_or_else(|| AppErrorDto::new("GIT_LOG_PARSE_FAILED", "Cannot parse git commit log", true))
    }

    fn ensure_local_identity(&self, project_root: &str) -> Result<(), AppErrorDto> {
        let user_name = run_git(project_root, &["config", "--get", "user.name"]).unwrap_or_default();
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
                AppErrorDto::new("GIT_INIT_FAILED", "Cannot update .gitignore", true)
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
                AppErrorDto::new(
                    "GIT_NOT_INSTALLED",
                    "Git executable is not available on this machine",
                    true,
                )
                .with_detail(err.to_string())
            } else {
                AppErrorDto::new("GIT_COMMAND_FAILED", "Failed to execute git command", true)
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
        AppErrorDto::new("GIT_COMMAND_FAILED", "Git command returned non-zero exit status", true)
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    use uuid::Uuid;

    use crate::services::project_service::{CreateProjectInput, ProjectService};

    use super::GitService;

    fn create_temp_workspace() -> PathBuf {
        let workspace = std::env::temp_dir().join(format!("novelforge-git-tests-{}", Uuid::new_v4()));
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
}
