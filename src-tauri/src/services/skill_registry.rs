use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::errors::AppErrorDto;
use crate::services::task_routing;

/// ── SkillManifest (extended, .md frontmatter-aligned) ──

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
    pub author: Option<String>,
    pub icon: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, alias = "skill_class")]
    pub skill_class: Option<String>,
    #[serde(default, alias = "bundle_ids")]
    pub bundle_ids: Vec<String>,
    #[serde(default, alias = "always_on")]
    pub always_on: bool,
    #[serde(default, alias = "trigger_conditions")]
    pub trigger_conditions: Vec<String>,
    #[serde(default, alias = "required_contexts")]
    pub required_contexts: Vec<String>,
    #[serde(default, alias = "state_writes")]
    pub state_writes: Vec<String>,
    #[serde(default, alias = "automation_tier")]
    pub automation_tier: Option<String>,
    #[serde(default, alias = "scene_tags")]
    pub scene_tags: Vec<String>,
    #[serde(default, alias = "affects_layers")]
    pub affects_layers: Vec<String>,
    /// Optional task route override: if set, overrides global llm_task_routes.
    #[serde(default, alias = "task_route", skip_serializing_if = "Option::is_none")]
    pub task_route: Option<SkillTaskRouteOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillTaskRouteOverride {
    pub task_type: String,
    #[serde(default, alias = "provider", alias = "provider_id")]
    pub provider_id: String,
    #[serde(default, alias = "model", alias = "model_id")]
    pub model_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RouteOverride {
    pub provider: String,
    pub model: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct SelectedSkills {
    pub workflow_skills: Vec<SkillManifest>,
    pub capability_skills: Vec<SkillManifest>,
    pub extractor_skills: Vec<SkillManifest>,
    pub policy_skills: Vec<SkillManifest>,
    pub review_skills: Vec<SkillManifest>,
    pub route_override: Option<RouteOverride>,
}

impl SelectedSkills {
    pub fn all_skills(&self) -> impl Iterator<Item = &SkillManifest> {
        self.workflow_skills
            .iter()
            .chain(self.capability_skills.iter())
            .chain(self.extractor_skills.iter())
            .chain(self.policy_skills.iter())
            .chain(self.review_skills.iter())
    }

    pub fn all_state_writes(&self) -> Vec<String> {
        let mut state_writes = Vec::new();
        for skill in self.all_skills() {
            for item in &skill.state_writes {
                if !state_writes.iter().any(|existing| existing == item) {
                    state_writes.push(item.clone());
                }
            }
        }
        state_writes
    }

    pub fn all_affects_layers(&self) -> Vec<String> {
        let mut affects_layers: Vec<String> = Vec::new();
        for skill in self.all_skills() {
            for item in &skill.affects_layers {
                let normalized = item.trim();
                if normalized.is_empty() {
                    continue;
                }
                if !affects_layers
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(normalized))
                {
                    affects_layers.push(normalized.to_string());
                }
            }
        }
        affects_layers
    }

    pub fn all_skill_ids(&self) -> Vec<String> {
        self.all_skills().map(|skill| skill.id.clone()).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SkillSelectionContext {
    pub explicit_skill_ids: Vec<String>,
    pub active_bundle_ids: Vec<String>,
    pub scene_tags: Vec<String>,
    pub available_contexts: Vec<String>,
    pub automation_tier: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifestPatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default, alias = "skill_class")]
    pub skill_class: Option<String>,
    #[serde(default, alias = "bundle_ids")]
    pub bundle_ids: Option<Vec<String>>,
    #[serde(default, alias = "always_on")]
    pub always_on: Option<bool>,
    #[serde(default, alias = "trigger_conditions")]
    pub trigger_conditions: Option<Vec<String>>,
    #[serde(default, alias = "required_contexts")]
    pub required_contexts: Option<Vec<String>>,
    #[serde(default, alias = "state_writes")]
    pub state_writes: Option<Vec<String>>,
    #[serde(default, alias = "automation_tier")]
    pub automation_tier: Option<String>,
    #[serde(default, alias = "scene_tags")]
    pub scene_tags: Option<Vec<String>>,
    #[serde(default, alias = "affects_layers")]
    pub affects_layers: Option<Vec<String>>,
}

/// Parsed skill file: metadata + body text.
pub struct SkillFile {
    pub manifest: SkillManifest,
    pub body: String,
}

/// ── SkillRegistry (filesystem-backed) ──
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
            AppErrorDto::new("SKILLS_DIR_FAILED", "无法创建技能目录", true)
                .with_detail(e.to_string())
        })?;

        // 2. Copy builtin .md files if the skills directory is empty or missing builtins
        if self.builtin_dir.exists() {
            let builtin_ids = self.list_builtin_ids();
            for entry in fs::read_dir(&self.builtin_dir).map_err(|e| {
                AppErrorDto::new("SKILLS_READ_BUILTIN_FAILED", "无法读取内置技能目录", true)
                    .with_detail(e.to_string())
            })? {
                let entry = entry.map_err(|e| {
                    AppErrorDto::new("SKILLS_ENTRY_FAILED", "无法读取内置技能条目", true)
                        .with_detail(e.to_string())
                })?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let file_name = path.file_name().unwrap_or_default();
                let target = self.skills_dir.join(file_name);
                let mut should_copy = !target.exists();
                if !should_copy {
                    let src_manifest = Self::parse_file(&path).ok().map(|sf| sf.manifest);
                    let target_manifest = Self::parse_file(&target).ok().map(|sf| sf.manifest);
                    if let (Some(src), Some(dst)) = (src_manifest, target_manifest) {
                        // Builtin template hotfixes should roll forward for existing builtin skills.
                        if dst.source == "builtin" && src.version > dst.version {
                            should_copy = true;
                        }
                    }
                }
                if should_copy {
                    fs::copy(&path, &target).map_err(|e| {
                        AppErrorDto::new("SKILLS_COPY_FAILED", "无法复制内置技能", true)
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
                AppErrorDto::new("SKILLS_LOCK_FAILED", "技能注册表锁定失败", false)
                    .with_detail(e.to_string())
            })?;
            *guard = list;
            return Ok(());
        }

        for entry in fs::read_dir(&self.skills_dir).map_err(|e| {
            AppErrorDto::new("SKILLS_READ_DIR_FAILED", "无法读取技能目录", true)
                .with_detail(e.to_string())
        })? {
            let entry = entry.map_err(|e| {
                AppErrorDto::new("SKILLS_ENTRY_FAILED", "无法读取技能条目", true)
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
            AppErrorDto::new("SKILLS_LOCK_FAILED", "技能注册表锁定失败", false)
                .with_detail(e.to_string())
        })?;
        *guard = list;

        Ok(())
    }

    // ── Public CRUD ──

    pub fn list_skills(&self) -> Result<Vec<SkillManifest>, AppErrorDto> {
        self.manifests.read().map(|g| g.clone()).map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "技能注册表锁定失败", false)
                .with_detail(e.to_string())
        })
    }

    pub fn get_skill(&self, id: &str) -> Result<Option<SkillManifest>, AppErrorDto> {
        let guard = self.manifests.read().map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "技能注册表锁定失败", false)
                .with_detail(e.to_string())
        })?;
        Ok(guard.iter().find(|s| s.id == id).cloned())
    }

    pub fn select_skills_for_task(&self, task_type: &str) -> Result<SelectedSkills, AppErrorDto> {
        self.select_skills_for_task_with_context(task_type, &SkillSelectionContext::default())
    }

    pub fn select_skills_for_task_with_context(
        &self,
        task_type: &str,
        selection: &SkillSelectionContext,
    ) -> Result<SelectedSkills, AppErrorDto> {
        let canonical_task = task_routing::canonical_task_type(task_type).into_owned();
        let guard = self.manifests.read().map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "技能注册表锁定失败", false)
                .with_detail(e.to_string())
        })?;

        let mut selected = SelectedSkills::default();
        let explicit_skill_ids = normalize_string_set(&selection.explicit_skill_ids);
        let active_bundle_ids = normalize_string_set(&selection.active_bundle_ids);
        let scene_tags = normalize_string_set(&selection.scene_tags);
        let available_contexts = normalize_string_set(&selection.available_contexts);
        let runtime_tier =
            normalize_optional_string(selection.automation_tier.clone().unwrap_or_default());

        for skill in guard.iter() {
            let explicit = explicit_skill_ids.contains(&skill.id.trim().to_ascii_lowercase());
            let matched_by_trigger = skill
                .trigger_conditions
                .iter()
                .any(|condition| task_pattern_matches(condition, &canonical_task));
            let matched_by_route = skill
                .task_route
                .as_ref()
                .map(|route| route_matches_task(route, &canonical_task))
                .unwrap_or(false);
            let matched_by_bundle = skill.bundle_ids.iter().any(|bundle_id| {
                let normalized = bundle_id.trim();
                !normalized.is_empty() && active_bundle_ids.contains(normalized)
            });
            let activation_source = explicit
                || skill.always_on
                || matched_by_trigger
                || matched_by_route
                || matched_by_bundle;
            if !activation_source {
                continue;
            }
            if !skill_matches_scene(skill, &scene_tags) {
                continue;
            }
            if !skill_has_required_contexts(skill, &available_contexts) {
                continue;
            }
            if !skill_matches_automation_tier(skill, runtime_tier.as_deref()) {
                continue;
            }

            match skill.skill_class.as_deref().unwrap_or("") {
                "workflow" => selected.workflow_skills.push(skill.clone()),
                "capability" => selected.capability_skills.push(skill.clone()),
                "extractor" => selected.extractor_skills.push(skill.clone()),
                "policy" => selected.policy_skills.push(skill.clone()),
                "review" => selected.review_skills.push(skill.clone()),
                _ => {}
            }

            if selected.route_override.is_none() {
                if let Some(route) = skill.task_route.as_ref() {
                    if !route_matches_task(route, &canonical_task) {
                        continue;
                    }
                    let provider = route.provider_id.trim().to_string();
                    let model = route.model_id.trim().to_string();
                    if provider.is_empty() && model.is_empty() {
                        continue;
                    }
                    selected.route_override = Some(RouteOverride {
                        provider,
                        model,
                        reason: route
                            .reason
                            .clone()
                            .unwrap_or_else(|| format!("skill '{}' task_route override", skill.id)),
                    });
                }
            }
        }
        Ok(selected)
    }

    /// Read the full .md content of a skill (for editing).
    pub fn read_skill_content(&self, id: &str) -> Result<Option<String>, AppErrorDto> {
        validate_id(id)?;
        let path = self.skills_dir.join(format!("{}.md", id));
        if !path.exists() {
            return Ok(None);
        }
        fs::read_to_string(&path).map(Some).map_err(|e| {
            AppErrorDto::new("SKILLS_READ_FAILED", "无法读取技能文件", true)
                .with_detail(e.to_string())
        })
    }

    /// Read prompt template body for runtime rendering.
    pub fn read_skill_prompt_template(&self, id: &str) -> Result<Option<String>, AppErrorDto> {
        let Some(content) = self.read_skill_content(id)? else {
            return Ok(None);
        };
        let (_frontmatter, body) = split_frontmatter(&content)?;
        Ok(Some(extract_prompt_template_body(body)))
    }

    /// Create a new skill from manifest + body content.
    pub fn create_skill(&self, manifest: &SkillManifest, body: &str) -> Result<(), AppErrorDto> {
        validate_id(&manifest.id)?;

        let path = self.skills_dir.join(format!("{}.md", manifest.id));
        if path.exists() {
            return Err(AppErrorDto::new(
                "SKILLS_CONFLICT",
                &format!("技能 '{}' 已存在", manifest.id),
                true,
            ));
        }

        let content = render_skill_file(manifest, body);
        fs::write(&path, &content).map_err(|e| {
            AppErrorDto::new("SKILLS_WRITE_FAILED", "无法写入技能文件", true)
                .with_detail(e.to_string())
        })?;

        self.reload()?;
        Ok(())
    }

    /// Update an existing skill's content file or manifest metadata.
    pub fn update_skill(
        &self,
        id: &str,
        body: Option<&str>,
        manifest_patch: Option<SkillManifestPatch>,
    ) -> Result<SkillManifest, AppErrorDto> {
        validate_id(id)?;
        let path = self.skills_dir.join(format!("{}.md", id));
        if !path.exists() {
            return Err(AppErrorDto::new(
                "SKILLS_NOT_FOUND",
                &format!("未找到技能 '{}'", id),
                true,
            ));
        }

        // Read existing manifest to preserve id, source, created_at
        let existing = SkillRegistry::parse_file(&path)?;
        let mut manifest = existing.manifest;
        if let Some(patch) = manifest_patch {
            apply_manifest_patch(&mut manifest, patch)?;
        }
        manifest.updated_at = crate::infra::time::now_iso();
        manifest.version += 1;

        let next_body = body.unwrap_or(existing.body.as_str());
        let content = render_skill_file(&manifest, next_body);
        fs::write(&path, &content).map_err(|e| {
            AppErrorDto::new("SKILLS_WRITE_FAILED", "无法写入技能文件", true)
                .with_detail(e.to_string())
        })?;

        self.reload()?;
        Ok(manifest)
    }

    /// Delete a skill file (only user/imported skills).
    pub fn delete_skill(&self, id: &str) -> Result<(), AppErrorDto> {
        validate_id(id)?;
        let guard = self.manifests.read().map_err(|e| {
            AppErrorDto::new("SKILLS_LOCK_FAILED", "技能注册表锁定失败", false)
                .with_detail(e.to_string())
        })?;

        let skill = guard.iter().find(|s| s.id == id).ok_or_else(|| {
            AppErrorDto::new("SKILLS_NOT_FOUND", &format!("未找到技能 '{}'", id), true)
        })?;

        if skill.source == "builtin" {
            return Err(AppErrorDto::new(
                "SKILLS_CANNOT_DELETE_BUILTIN",
                "内置技能不可删除，请使用重置",
                true,
            ));
        }

        let path = self.skills_dir.join(format!("{}.md", id));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                AppErrorDto::new("SKILLS_DELETE_FAILED", "无法删除技能文件", true)
                    .with_detail(e.to_string())
            })?;
        }

        self.reload()?;
        Ok(())
    }

    /// Reset a built-in skill to its original content.
    pub fn reset_builtin(&self, id: &str) -> Result<SkillManifest, AppErrorDto> {
        validate_id(id)?;
        // Validate: builtin must exist in builtin dir
        let src = self.builtin_dir.join(format!("{}.md", id));
        if !src.exists() {
            return Err(AppErrorDto::new(
                "SKILLS_NOT_FOUND",
                &format!("内置包中未找到内置技能 '{}'", id),
                true,
            ));
        }

        let target = self.skills_dir.join(format!("{}.md", id));
        fs::copy(&src, &target).map_err(|e| {
            AppErrorDto::new("SKILLS_COPY_FAILED", "无法重置内置技能", true)
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
        validate_id(&sf.manifest.id)?;

        // Validate ID uniqueness
        if self.get_skill(&sf.manifest.id)?.is_some() {
            return Err(AppErrorDto::new(
                "SKILLS_CONFLICT",
                &format!("技能ID '{}' 已存在", sf.manifest.id),
                true,
            ));
        }

        let target = self.skills_dir.join(format!("{}.md", sf.manifest.id));
        fs::copy(src, &target).map_err(|e| {
            AppErrorDto::new("SKILLS_COPY_FAILED", "无法导入技能文件", true)
                .with_detail(e.to_string())
        })?;

        self.reload()?;
        Ok(sf.manifest)
    }

    // ── Internal helpers ──

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
            AppErrorDto::new("SKILLS_READ_FAILED", "无法读取技能文件", true)
                .with_detail(e.to_string())
        })?;

        let (frontmatter_str, body) = split_frontmatter(&content)?;

        let mut manifest: SkillManifest = serde_yaml::from_str(frontmatter_str).map_err(|e| {
            AppErrorDto::new("SKILLS_PARSE_FAILED", "无法解析技能 frontmatter", true)
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

const ALLOWED_SKILL_CLASSES: [&str; 5] =
    ["workflow", "capability", "extractor", "review", "policy"];
const ALLOWED_AUTOMATION_TIERS: [&str; 3] = ["auto", "supervised", "confirm"];

fn apply_manifest_patch(
    manifest: &mut SkillManifest,
    patch: SkillManifestPatch,
) -> Result<(), AppErrorDto> {
    if let Some(name) = patch.name {
        manifest.name = name;
    }
    if let Some(description) = patch.description {
        manifest.description = description;
    }
    if let Some(category) = patch.category {
        manifest.category = category;
    }
    if let Some(tags) = patch.tags {
        manifest.tags = trim_items(tags);
    }
    if let Some(icon) = patch.icon {
        manifest.icon = normalize_optional_string(icon);
    }
    if let Some(skill_class) = patch.skill_class {
        manifest.skill_class = validate_skill_class(skill_class)?;
    }
    if let Some(bundle_ids) = patch.bundle_ids {
        manifest.bundle_ids = trim_items(bundle_ids);
    }
    if let Some(always_on) = patch.always_on {
        manifest.always_on = always_on;
    }
    if let Some(trigger_conditions) = patch.trigger_conditions {
        manifest.trigger_conditions = trim_items(trigger_conditions);
    }
    if let Some(required_contexts) = patch.required_contexts {
        manifest.required_contexts = trim_items(required_contexts);
    }
    if let Some(state_writes) = patch.state_writes {
        manifest.state_writes = trim_items(state_writes);
    }
    if let Some(automation_tier) = patch.automation_tier {
        manifest.automation_tier = validate_automation_tier(automation_tier)?;
    }
    if let Some(scene_tags) = patch.scene_tags {
        manifest.scene_tags = trim_items(scene_tags);
    }
    if let Some(affects_layers) = patch.affects_layers {
        manifest.affects_layers = trim_items(affects_layers);
    }

    Ok(())
}

fn validate_skill_class(value: String) -> Result<Option<String>, AppErrorDto> {
    let normalized = normalize_optional_string(value);
    match normalized {
        Some(ref class) if !ALLOWED_SKILL_CLASSES.contains(&class.as_str()) => {
            Err(AppErrorDto::new(
                "SKILLS_INVALID_SKILL_CLASS",
                "技能分类无效，应为 workflow/capability/extractor/review/policy",
                true,
            ))
        }
        _ => Ok(normalized),
    }
}

fn validate_automation_tier(value: String) -> Result<Option<String>, AppErrorDto> {
    let normalized = normalize_optional_string(value);
    match normalized {
        Some(ref tier) if !ALLOWED_AUTOMATION_TIERS.contains(&tier.as_str()) => {
            Err(AppErrorDto::new(
                "SKILLS_INVALID_AUTOMATION_TIER",
                "自动化档位无效，应为 auto/supervised/confirm",
                true,
            ))
        }
        _ => Ok(normalized),
    }
}

fn trim_items(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn normalize_optional_string(value: String) -> Option<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn route_matches_task(route: &SkillTaskRouteOverride, canonical_task: &str) -> bool {
    let route_task = route.task_type.trim();
    if route_task.is_empty() {
        return true;
    }
    task_pattern_matches(route_task, canonical_task)
}

fn task_pattern_matches(pattern: &str, canonical_task: &str) -> bool {
    let normalized = pattern.trim();
    if normalized.is_empty() {
        return false;
    }
    if normalized == "*" || normalized.eq_ignore_ascii_case("all") {
        return true;
    }
    if let Some(prefix) = normalized.strip_suffix(".*") {
        let canonical_prefix = task_routing::canonical_task_type(prefix).into_owned();
        if canonical_prefix.is_empty() {
            return false;
        }
        let dotted = format!("{canonical_prefix}.");
        return canonical_task.starts_with(&dotted);
    }
    let canonical_pattern = task_routing::canonical_task_type(normalized).into_owned();
    canonical_pattern == canonical_task
}

fn normalize_string_set(items: &[String]) -> std::collections::HashSet<String> {
    items
        .iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

fn skill_matches_scene(
    skill: &SkillManifest,
    runtime_scene_tags: &std::collections::HashSet<String>,
) -> bool {
    if skill.scene_tags.is_empty() {
        return true;
    }
    if runtime_scene_tags.is_empty() {
        return false;
    }
    skill.scene_tags.iter().any(|tag| {
        let normalized = tag.trim().to_ascii_lowercase();
        !normalized.is_empty() && runtime_scene_tags.contains(&normalized)
    })
}

fn skill_has_required_contexts(
    skill: &SkillManifest,
    available_contexts: &std::collections::HashSet<String>,
) -> bool {
    skill.required_contexts.iter().all(|context_key| {
        let normalized = context_key.trim().to_ascii_lowercase();
        normalized.is_empty() || available_contexts.contains(&normalized)
    })
}

fn skill_matches_automation_tier(skill: &SkillManifest, runtime_tier: Option<&str>) -> bool {
    let Some(required_tier) = skill.automation_tier.as_deref() else {
        return true;
    };
    let Some(required_rank) = automation_tier_rank(required_tier) else {
        return true;
    };
    let Some(runtime_rank) = runtime_tier.and_then(automation_tier_rank) else {
        return false;
    };
    runtime_rank >= required_rank
}

fn automation_tier_rank(value: &str) -> Option<u8> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(1),
        "supervised" => Some(2),
        "confirm" => Some(3),
        _ => None,
    }
}

// ── File format helpers ──

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
            "技能文件必须以 --- frontmatter 开头",
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
            "技能文件存在起始 --- 但缺少结束 ---",
            true,
        ))
    }
}

const PROMPT_TEMPLATE_START: &str = "<!-- PROMPT_TEMPLATE_START -->";
const PROMPT_TEMPLATE_END: &str = "<!-- PROMPT_TEMPLATE_END -->";

// 问题4修复: 运行时仅提取模板正文，不再把 frontmatter/说明文档直接发送给 LLM。
fn extract_prompt_template_body(body: &str) -> String {
    let trimmed = body.trim();
    if let Some(start) = trimmed.find(PROMPT_TEMPLATE_START) {
        let after_start = &trimmed[start + PROMPT_TEMPLATE_START.len()..];
        if let Some(end) = after_start.find(PROMPT_TEMPLATE_END) {
            return after_start[..end].trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Render manifest to YAML frontmatter + body = complete .md file.
fn render_skill_file(manifest: &SkillManifest, body: &str) -> String {
    let yaml = serde_yaml::to_string(manifest).unwrap_or_default();
    format!("---\n{}---\n{}\n", yaml, body.trim())
}

pub fn validate_skill_id(id: &str) -> Result<(), AppErrorDto> {
    validate_id(id)
}

/// Validate skill ID: only alphanumeric, dots, hyphens, underscores.
fn validate_id(id: &str) -> Result<(), AppErrorDto> {
    if id.is_empty() {
        return Err(AppErrorDto::new(
            "SKILLS_INVALID_ID",
            "技能ID不能为空",
            true,
        ));
    }
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err(AppErrorDto::new(
            "SKILLS_INVALID_ID",
            "技能ID不能包含路径分隔符或父目录片段",
            true,
        ));
    }
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(AppErrorDto::new(
            "SKILLS_INVALID_ID",
            "技能ID仅允许字母、数字、点、连字符、下划线",
            true,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_registry(name: &str) -> SkillRegistry {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root: PathBuf =
            std::env::temp_dir().join(format!("novelforge-skill-test-{name}-{unique}"));
        let skills_dir = root.join("skills");
        let builtin_dir = root.join("builtin");
        std::fs::create_dir_all(&skills_dir).expect("create skills dir");
        std::fs::create_dir_all(&builtin_dir).expect("create builtin dir");
        SkillRegistry::new(skills_dir, builtin_dir)
    }

    #[test]
    fn update_skill_manifest_roundtrip_succeeds() {
        let registry = create_test_registry("roundtrip");
        let manifest = SkillManifest {
            id: "test.skill".to_string(),
            name: "Test Skill".to_string(),
            description: "desc".to_string(),
            version: 1,
            source: "user".to_string(),
            category: "utility".to_string(),
            tags: vec!["alpha".to_string()],
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: serde_json::json!({"type":"object"}),
            requires_user_confirmation: true,
            writes_to_project: false,
            author: Some("tester".to_string()),
            icon: Some("A".to_string()),
            created_at: "2026-04-30T00:00:00Z".to_string(),
            updated_at: "2026-04-30T00:00:00Z".to_string(),
            skill_class: None,
            bundle_ids: Vec::new(),
            always_on: false,
            trigger_conditions: Vec::new(),
            required_contexts: Vec::new(),
            state_writes: Vec::new(),
            automation_tier: None,
            scene_tags: Vec::new(),
            affects_layers: Vec::new(),
            task_route: None,
        };
        registry
            .create_skill(&manifest, "original body")
            .expect("create skill");

        let updated = registry
            .update_skill(
                "test.skill",
                Some("updated body"),
                Some(SkillManifestPatch {
                    skill_class: Some("workflow".to_string()),
                    bundle_ids: Some(vec!["chapter-core".to_string()]),
                    always_on: Some(true),
                    trigger_conditions: Some(vec!["chapter.plan".to_string()]),
                    required_contexts: Some(vec!["canon".to_string()]),
                    state_writes: Some(vec!["plot.progress".to_string()]),
                    automation_tier: Some("confirm".to_string()),
                    scene_tags: Some(vec!["battle".to_string(), "dialogue".to_string()]),
                    affects_layers: Some(vec!["canon".to_string(), "state".to_string()]),
                    ..Default::default()
                }),
            )
            .expect("update skill");

        assert_eq!(updated.skill_class.as_deref(), Some("workflow"));
        assert_eq!(updated.bundle_ids, vec!["chapter-core"]);
        assert!(updated.always_on);
        assert_eq!(updated.trigger_conditions, vec!["chapter.plan"]);
        assert_eq!(updated.required_contexts, vec!["canon"]);
        assert_eq!(updated.state_writes, vec!["plot.progress"]);
        assert_eq!(updated.automation_tier.as_deref(), Some("confirm"));
        assert_eq!(updated.scene_tags, vec!["battle", "dialogue"]);
        assert_eq!(updated.affects_layers, vec!["canon", "state"]);
        assert_eq!(updated.version, 2);

        let content = registry
            .read_skill_content("test.skill")
            .expect("read skill content")
            .expect("skill exists");
        assert!(content.contains("skillClass: workflow"));
        assert!(content.contains("bundleIds:"));
        assert!(content.contains("updated body"));
    }

    #[test]
    fn update_skill_rejects_invalid_skill_class() {
        let registry = create_test_registry("invalid-class");
        let manifest = SkillManifest {
            id: "invalid.class".to_string(),
            name: "Invalid".to_string(),
            description: "desc".to_string(),
            version: 1,
            source: "user".to_string(),
            category: "utility".to_string(),
            tags: Vec::new(),
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: serde_json::json!({"type":"object"}),
            requires_user_confirmation: true,
            writes_to_project: false,
            author: None,
            icon: None,
            created_at: "2026-04-30T00:00:00Z".to_string(),
            updated_at: "2026-04-30T00:00:00Z".to_string(),
            skill_class: None,
            bundle_ids: Vec::new(),
            always_on: false,
            trigger_conditions: Vec::new(),
            required_contexts: Vec::new(),
            state_writes: Vec::new(),
            automation_tier: None,
            scene_tags: Vec::new(),
            affects_layers: Vec::new(),
            task_route: None,
        };
        registry
            .create_skill(&manifest, "body")
            .expect("create skill");

        let err = registry
            .update_skill(
                "invalid.class",
                None,
                Some(SkillManifestPatch {
                    skill_class: Some("unknown".to_string()),
                    ..Default::default()
                }),
            )
            .expect_err("should reject invalid class");
        assert_eq!(err.code, "SKILLS_INVALID_SKILL_CLASS");
    }

    fn build_manifest(id: &str, class: &str) -> SkillManifest {
        SkillManifest {
            id: id.to_string(),
            name: format!("{id} name"),
            description: "desc".to_string(),
            version: 1,
            source: "user".to_string(),
            category: "utility".to_string(),
            tags: Vec::new(),
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: serde_json::json!({"type":"object"}),
            requires_user_confirmation: true,
            writes_to_project: false,
            author: None,
            icon: None,
            created_at: "2026-04-30T00:00:00Z".to_string(),
            updated_at: "2026-04-30T00:00:00Z".to_string(),
            skill_class: Some(class.to_string()),
            bundle_ids: Vec::new(),
            always_on: false,
            trigger_conditions: Vec::new(),
            required_contexts: Vec::new(),
            state_writes: Vec::new(),
            automation_tier: None,
            scene_tags: Vec::new(),
            affects_layers: Vec::new(),
            task_route: None,
        }
    }

    #[test]
    fn select_skills_for_task_applies_always_on_trigger_and_route_override() {
        let registry = create_test_registry("selection");

        let mut policy = build_manifest("policy.term-lock", "policy");
        policy.always_on = true;
        registry
            .create_skill(&policy, "policy body")
            .expect("create policy");

        let mut capability = build_manifest("capability.scene", "capability");
        capability.trigger_conditions = vec!["chapter.plan".to_string()];
        registry
            .create_skill(&capability, "capability body")
            .expect("create capability");

        let mut workflow = build_manifest("workflow.custom", "workflow");
        workflow.trigger_conditions = vec!["custom.scene.render".to_string()];
        workflow.task_route = Some(SkillTaskRouteOverride {
            task_type: "custom.scene.render".to_string(),
            provider_id: "provider-override".to_string(),
            model_id: "model-override".to_string(),
            reason: Some("high precision scene".to_string()),
        });
        registry
            .create_skill(&workflow, "workflow body")
            .expect("create workflow");

        let chapter_selected = registry
            .select_skills_for_task("chapter.plan")
            .expect("select chapter skills");
        assert_eq!(chapter_selected.policy_skills.len(), 1);
        assert_eq!(chapter_selected.capability_skills.len(), 1);
        assert!(chapter_selected.route_override.is_none());

        let custom_selected = registry
            .select_skills_for_task("custom.scene.render")
            .expect("select custom skills");
        assert_eq!(custom_selected.workflow_skills.len(), 1);
        let route_override = custom_selected.route_override.expect("route override");
        assert_eq!(route_override.provider, "provider-override");
        assert_eq!(route_override.model, "model-override");
        assert_eq!(route_override.reason, "high precision scene");
    }

    #[test]
    fn select_skills_for_task_with_context_applies_bundle_scene_context_and_tier() {
        let registry = create_test_registry("selection-context");

        let mut policy = build_manifest("policy.term-lock", "policy");
        policy.always_on = true;
        registry
            .create_skill(&policy, "policy body")
            .expect("create policy");

        let mut bundled = build_manifest("capability.emotion-flow", "capability");
        bundled.bundle_ids = vec!["emotion-pack".to_string()];
        bundled.scene_tags = vec!["dialogue".to_string()];
        bundled.required_contexts = vec!["state".to_string(), "canon".to_string()];
        bundled.automation_tier = Some("supervised".to_string());
        registry
            .create_skill(&bundled, "bundled capability body")
            .expect("create bundled capability");

        let mut confirm_only = build_manifest("review.combat-check", "review");
        confirm_only.bundle_ids = vec!["combat-pack".to_string()];
        confirm_only.scene_tags = vec!["battle".to_string()];
        confirm_only.automation_tier = Some("confirm".to_string());
        registry
            .create_skill(&confirm_only, "confirm review body")
            .expect("create confirm review");

        let selected = registry
            .select_skills_for_task_with_context(
                "chapter.draft",
                &SkillSelectionContext {
                    active_bundle_ids: vec!["emotion-pack".to_string(), "combat-pack".to_string()],
                    scene_tags: vec!["dialogue".to_string()],
                    available_contexts: vec!["canon".to_string(), "state".to_string()],
                    automation_tier: Some("supervised".to_string()),
                    ..Default::default()
                },
            )
            .expect("select with runtime context");

        assert_eq!(selected.policy_skills.len(), 1);
        assert_eq!(selected.capability_skills.len(), 1);
        assert_eq!(selected.capability_skills[0].id, "capability.emotion-flow");
        assert!(selected.review_skills.is_empty());

        let selected_confirm = registry
            .select_skills_for_task_with_context(
                "chapter.draft",
                &SkillSelectionContext {
                    active_bundle_ids: vec!["combat-pack".to_string()],
                    scene_tags: vec!["battle".to_string()],
                    available_contexts: vec!["canon".to_string(), "state".to_string()],
                    automation_tier: Some("confirm".to_string()),
                    ..Default::default()
                },
            )
            .expect("select confirm context");

        assert_eq!(selected_confirm.review_skills.len(), 1);
        assert_eq!(selected_confirm.review_skills[0].id, "review.combat-check");
    }

    #[test]
    fn all_affects_layers_deduplicates_case_insensitively() {
        let registry = create_test_registry("affects-layers");

        let mut skill_a = build_manifest("capability.state-a", "capability");
        skill_a.trigger_conditions = vec!["chapter.draft".to_string()];
        skill_a.affects_layers = vec!["state".to_string(), "canon".to_string()];
        registry
            .create_skill(&skill_a, "state skill a")
            .expect("create skill a");

        let mut skill_b = build_manifest("capability.state-b", "capability");
        skill_b.trigger_conditions = vec!["chapter.draft".to_string()];
        skill_b.affects_layers = vec!["STATE".to_string(), "window_plan".to_string()];
        registry
            .create_skill(&skill_b, "state skill b")
            .expect("create skill b");

        let selected = registry
            .select_skills_for_task("chapter.draft")
            .expect("select draft skills");

        assert_eq!(
            selected.all_affects_layers(),
            vec![
                "state".to_string(),
                "canon".to_string(),
                "window_plan".to_string()
            ]
        );
    }

    #[test]
    fn validate_skill_id_rejects_path_traversal_fragments() {
        let err = validate_skill_id("../evil").expect_err("id should be rejected");
        assert_eq!(err.code, "SKILLS_INVALID_ID");
    }

    #[test]
    fn import_file_rejects_manifest_id_with_path_traversal() {
        let registry = create_test_registry("import-invalid-id");
        let manifest = build_manifest("../evil", "workflow");
        let skill_file = render_skill_file(&manifest, "body");
        let external_path = std::env::temp_dir().join(format!(
            "novelforge-skill-import-invalid-{}.md",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(&external_path, skill_file).expect("write external skill file");

        let err = registry
            .import_file(external_path.to_string_lossy().as_ref())
            .expect_err("import should reject traversal id");
        assert_eq!(err.code, "SKILLS_INVALID_ID");

        let _ = std::fs::remove_file(external_path);
    }
}
