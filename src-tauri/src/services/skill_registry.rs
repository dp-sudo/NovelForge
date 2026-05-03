use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::errors::AppErrorDto;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: i32,
    pub source: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    #[serde(default = "default_true")]
    pub requires_user_confirmation: bool,
    pub writes_to_project: bool,
    #[serde(default = "default_strategy")]
    pub prompt_strategy: String,
    pub author: Option<String>,
    pub icon: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Optional task route override: if set, overrides global llm_task_routes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_route: Option<SkillTaskRouteOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillTaskRouteOverride {
    pub task_type: String,
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub model_id: String,
}

fn default_true() -> bool {
    true
}
fn default_strategy() -> String {
    "replace".to_string()
}

/// Parsed skill file: metadata + body text.
pub struct SkillFile {
    pub manifest: SkillManifest,
    pub body: String,
}

pub struct SkillRegistry {
    skills_dir: PathBuf,
    builtin_dir: PathBuf,
    manifests: RwLock<Vec<SkillManifest>>,
}

impl SkillRegistry {
    /// Create a new registry. Does NOT scan disk; call `initialize()` or `reload()`.
    pub fn new(skills_dir: PathBuf, builtin_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            builtin_dir,
            manifests: RwLock::new(Vec::new()),
        }
    }

    /// First-time initialization: create skills dir, copy builtins, run DB migration.
    /// Safe to call repeatedly (checks for existing files).
    pub fn initialize(&self) -> Result<(), AppErrorDto> {
        // 1. Create skills directory
        fs::create_dir_all(&self.skills_dir).map_err(|e| {
            AppErrorDto::new("SKILLS_DIR_FAILED", "Cannot create skills directory", true)
                .with_detail(e.to_string())
        })?;

        // 2. Copy builtin .md files if the skills directory is empty or missing builtins
        if self.builtin_dir.exists() {
            let builtin_ids = self.list_builtin_ids();
            for entry in fs::read_dir(&self.builtin_dir).map_err(|e| {
                AppErrorDto::new(
                    "SKILLS_READ_BUILTIN_FAILED",
                    "Cannot read builtin skills",
                    true,
                )
                .with_detail(e.to_string())
            })? {
                let entry = entry.map_err(|e| {
                    AppErrorDto::new("SKILLS_ENTRY_FAILED", "Cannot read builtin entry", true)
                        .with_detail(e.to_string())
                })?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let file_name = path.file_name().unwrap_or_default();
                let target = self.skills_dir.join(file_name);
                if !target.exists() {
                    fs::copy(&path, &target).map_err(|e| {
                        AppErrorDto::new("SKILLS_COPY_FAILED", "Cannot copy builtin skill", true)
                            .with_detail(e.to_string())
                    })?;
                }
            }

            // Ensure builtin IDs exist (in case user deleted some)
            for id in &builtin_ids {
                let target = self.skills_dir.join(format!("{}.md", id));
                if !target.exists() {
                    let src = self.builtin_dir.join(format!("{}.md", id));
                    if src.exists() {
                        fs::copy(&src, &target).ok();
                    }
                }
            }
        }

        // 3. Scan skills directory into memory
        self.reload()?;

        Ok(())
    }

    /// Re-scan the skills directory and refresh the in-memory manifest list.
    pub fn reload(&self) -> Result<(), AppErrorDto> {
        let mut list = Vec::new();
        if !self.skills_dir.exists() {
            let mut guard = self.manifests.write().map_err(|e| {
                AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                    .with_detail(e.to_string())
            })?;
            *guard = list;
            return Ok(());
        }

        for entry in fs::read_dir(&self.skills_dir).map_err(|e| {
            AppErrorDto::new(
                "SKILLS_READ_DIR_FAILED",
                "Cannot read skills directory",
                true,
            )
            .with_detail(e.to_string())
        })? {
            let entry = entry.map_err(|e| {
                AppErrorDto::new("SKILLS_ENTRY_FAILED", "Cannot read skill entry", true)
                    .with_detail(e.to_string())
            })?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Ok(sf) = Self::parse_file(&path) {
                list.push(sf.manifest);
            }
        }

        // Sort by id for deterministic order
        list.sort_by(|a, b| a.id.cmp(&b.id));

        let mut guard = self.manifests.write().map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                .with_detail(e.to_string())
        })?;
        *guard = list;

        Ok(())
    }

    pub fn list_skills(&self) -> Result<Vec<SkillManifest>, AppErrorDto> {
        self.manifests.read().map(|g| g.clone()).map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                .with_detail(e.to_string())
        })
    }

    pub fn get_skill(&self, id: &str) -> Result<Option<SkillManifest>, AppErrorDto> {
        let guard = self.manifests.read().map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                .with_detail(e.to_string())
        })?;
        Ok(guard.iter().find(|s| s.id == id).cloned())
    }

    /// Read the full .md content of a skill (for editing).
    pub fn read_skill_content(&self, id: &str) -> Result<Option<String>, AppErrorDto> {
        let path = self.skills_dir.join(format!("{}.md", id));
        if !path.exists() {
            return Ok(None);
        }
        fs::read_to_string(&path).map(Some).map_err(|e| {
            AppErrorDto::new("SKILLS_READ_FAILED", "Cannot read skill file", true)
                .with_detail(e.to_string())
        })
    }

    /// Create a new skill from manifest + body content.
    pub fn create_skill(&self, manifest: &SkillManifest, body: &str) -> Result<(), AppErrorDto> {
        validate_id(&manifest.id)?;

        let path = self.skills_dir.join(format!("{}.md", manifest.id));
        if path.exists() {
            return Err(AppErrorDto::new(
                "SKILLS_CONFLICT",
                &format!("Skill '{}' already exists", manifest.id),
                true,
            ));
        }

        let content = render_skill_file(manifest, body);
        fs::write(&path, &content).map_err(|e| {
            AppErrorDto::new("SKILLS_WRITE_FAILED", "Cannot write skill file", true)
                .with_detail(e.to_string())
        })?;

        self.reload()?;
        Ok(())
    }

    /// Update an existing skill's content file.
    pub fn update_skill(&self, id: &str, body: &str) -> Result<SkillManifest, AppErrorDto> {
        let path = self.skills_dir.join(format!("{}.md", id));
        if !path.exists() {
            return Err(AppErrorDto::new(
                "SKILLS_NOT_FOUND",
                &format!("Skill '{}' not found", id),
                true,
            ));
        }

        // Read existing manifest to preserve id, source, created_at
        let existing = SkillRegistry::parse_file(&path)?;
        let mut manifest = existing.manifest;
        manifest.updated_at = crate::infra::time::now_iso();
        manifest.version += 1;

        let content = render_skill_file(&manifest, body);
        fs::write(&path, &content).map_err(|e| {
            AppErrorDto::new("SKILLS_WRITE_FAILED", "Cannot write skill file", true)
                .with_detail(e.to_string())
        })?;

        self.reload()?;
        Ok(manifest)
    }

    /// Delete a skill file (only user/imported skills).
    pub fn delete_skill(&self, id: &str) -> Result<(), AppErrorDto> {
        let guard = self.manifests.read().map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "Skill registry lock failed", false)
                .with_detail(e.to_string())
        })?;

        let skill = guard.iter().find(|s| s.id == id).ok_or_else(|| {
            AppErrorDto::new(
                "SKILLS_NOT_FOUND",
                &format!("Skill '{}' not found", id),
                true,
            )
        })?;

        if skill.source == "builtin" {
            return Err(AppErrorDto::new(
                "SKILLS_CANNOT_DELETE_BUILTIN",
                "Cannot delete a built-in skill; use reset instead",
                true,
            ));
        }

        let path = self.skills_dir.join(format!("{}.md", id));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                AppErrorDto::new("SKILLS_DELETE_FAILED", "Cannot delete skill file", true)
                    .with_detail(e.to_string())
            })?;
        }

        self.reload()?;
        Ok(())
    }

    /// Reset a built-in skill to its original content.
    pub fn reset_builtin(&self, id: &str) -> Result<SkillManifest, AppErrorDto> {
        // Validate: builtin must exist in builtin dir
        let src = self.builtin_dir.join(format!("{}.md", id));
        if !src.exists() {
            return Err(AppErrorDto::new(
                "SKILLS_NOT_FOUND",
                &format!("Builtin skill '{}' not found in package", id),
                true,
            ));
        }

        let target = self.skills_dir.join(format!("{}.md", id));
        fs::copy(&src, &target).map_err(|e| {
            AppErrorDto::new("SKILLS_COPY_FAILED", "Cannot reset builtin skill", true)
                .with_detail(e.to_string())
        })?;

        self.reload()?;

        let sf = SkillRegistry::parse_file(&target)?;
        Ok(sf.manifest)
    }

    /// Import a .md file from an external path (user's file picker).
    pub fn import_file(&self, file_path: &str) -> Result<SkillManifest, AppErrorDto> {
        let src = Path::new(file_path);
        let sf = Self::parse_file(src)?;

        // Validate ID uniqueness
        if self.get_skill(&sf.manifest.id)?.is_some() {
            return Err(AppErrorDto::new(
                "SKILLS_CONFLICT",
                &format!("A skill with id '{}' already exists", sf.manifest.id),
                true,
            ));
        }

        let target = self.skills_dir.join(format!("{}.md", sf.manifest.id));
        fs::copy(src, &target).map_err(|e| {
            AppErrorDto::new("SKILLS_COPY_FAILED", "Cannot import skill file", true)
                .with_detail(e.to_string())
        })?;

        self.reload()?;
        Ok(sf.manifest)
    }

    /// List builtin skill IDs (without extension) from the bundled directory.
    fn list_builtin_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        if !self.builtin_dir.exists() {
            return ids;
        }
        if let Ok(entries) = fs::read_dir(&self.builtin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        ids.push(stem.to_string());
                    }
                }
            }
        }
        ids
    }

    /// Parse a single .md file into SkillManifest + body.
    pub fn parse_file(path: &Path) -> Result<SkillFile, AppErrorDto> {
        let content = fs::read_to_string(path).map_err(|e| {
            AppErrorDto::new("SKILLS_READ_FAILED", "Cannot read skill file", true)
                .with_detail(e.to_string())
        })?;

        let (frontmatter_str, body) = split_frontmatter(&content)?;

        let mut manifest: SkillManifest = serde_yaml::from_str(frontmatter_str).map_err(|e| {
            AppErrorDto::new(
                "SKILLS_PARSE_FAILED",
                "Cannot parse skill frontmatter",
                true,
            )
            .with_detail(e.to_string())
        })?;

        // Auto-set source if missing
        if manifest.source.is_empty() {
            manifest.source = "user".to_string();
        }
        if manifest.category.is_empty() {
            manifest.category = "utility".to_string();
        }

        Ok(SkillFile {
            manifest,
            body: body.to_string(),
        })
    }

    /// Render manifest + body back into .md file content.
    fn render_to_string(manifest: &SkillManifest, body: &str) -> String {
        let yaml = serde_yaml::to_string(manifest).unwrap_or_default();
        format!("---\n{}---\n{}", yaml, body)
    }
}

/// Access the global SkillRegistry through app-level storage.
/// Called from lib.rs setup to initialize.
pub fn initialize_global_registry(
    app_data_dir: &Path,
    builtin_dir: &Path,
) -> Result<SkillRegistry, AppErrorDto> {
    let skills_dir = app_data_dir.join("skills");
    let reg = SkillRegistry::new(skills_dir, builtin_dir.to_path_buf());
    reg.initialize()?;
    Ok(reg)
}

/// Split a .md string into (frontmatter_yaml, body_markdown).
/// Expects format:
///   ---
///   key: value
///   ---
///   body
fn split_frontmatter(content: &str) -> Result<(&str, &str), AppErrorDto> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(AppErrorDto::new(
            "SKILLS_INVALID_FORMAT",
            "Skill file must start with --- frontmatter",
            true,
        ));
    }

    // Skip the opening ---\n (or ---\r\n)
    let after_opening = &trimmed[3..];
    let after_newline = after_opening
        .strip_prefix('\n')
        .or_else(|| after_opening.strip_prefix("\r\n"))
        .unwrap_or(after_opening);

    // Find the closing ---
    if let Some(end) = after_newline.find("\n---") {
        let yaml = &after_newline[..end];
        let body = &after_newline[end + 4..]; // skip \n---
                                              // Skip potential \r\n after closing ---
        let body = body
            .strip_prefix('\n')
            .or_else(|| body.strip_prefix("\r\n"))
            .unwrap_or(body)
            .trim_start();
        Ok((yaml.trim(), body))
    } else {
        Err(AppErrorDto::new(
            "SKILLS_INVALID_FORMAT",
            "Skill file has opening --- but no closing ---",
            true,
        ))
    }
}

/// Render manifest to YAML frontmatter + body = complete .md file.
fn render_skill_file(manifest: &SkillManifest, body: &str) -> String {
    let yaml = serde_yaml::to_string(manifest).unwrap_or_default();
    format!("---\n{}---\n{}\n", yaml, body.trim())
}

/// Validate skill ID: only alphanumeric, dots, hyphens, underscores.
fn validate_id(id: &str) -> Result<(), AppErrorDto> {
    if id.is_empty() {
        return Err(AppErrorDto::new(
            "SKILLS_INVALID_ID",
            "Skill ID cannot be empty",
            true,
        ));
    }
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(AppErrorDto::new(
            "SKILLS_INVALID_ID",
            "Skill ID may only contain letters, digits, dots, hyphens, underscores",
            true,
        ));
    }
    Ok(())
}
