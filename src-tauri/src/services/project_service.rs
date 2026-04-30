use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::{initialize_database, open_database};
use crate::infra::fs_utils::write_file_atomic;
use crate::infra::path_utils::{resolve_project_relative_path, sanitize_project_directory_name};
use crate::infra::recent_projects::{
    clear_recent_projects, list_recent_projects, mark_recent_project, RecentProjectItem,
};
use crate::infra::time::now_iso;

const PROJECT_SCHEMA_VERSION: &str = "1.0.0";
const PROJECT_APP_MIN_VERSION: &str = "0.1.0";

// 问题2修复: 移除未接入的 prompts 目录初始化残留。
const REQUIRED_DIRS: [&str; 14] = [
    "database",
    "database/backups",
    "manuscript",
    "manuscript/chapters",
    "manuscript/drafts",
    "manuscript/snapshots",
    "blueprint",
    "assets",
    "assets/covers",
    "assets/attachments",
    "exports",
    "backups",
    "workflows",
    "logs",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    pub author: Option<String>,
    pub genre: String,
    pub target_words: Option<i64>,
    pub save_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    pub default_narrative_pov: String,
    #[serde(default)]
    pub writing_style: WritingStyle,
    pub language: String,
    pub autosave_interval_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WritingStyle {
    pub language_style: String,
    pub description_density: i64,
    pub dialogue_ratio: i64,
    pub sentence_rhythm: String,
    pub atmosphere: String,
    pub psychological_depth: i64,
}

impl Default for WritingStyle {
    fn default() -> Self {
        Self {
            language_style: "balanced".to_string(),
            description_density: 4,
            dialogue_ratio: 4,
            sentence_rhythm: "mixed".to_string(),
            atmosphere: "neutral".to_string(),
            psychological_depth: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiStrategyProfile {
    pub automation_default: String,
    pub review_strictness: i64,
    pub default_workflow_stack: Vec<String>,
    pub always_on_policy_skills: Vec<String>,
    pub default_capability_bundles: Vec<String>,
    pub state_write_policy: String,
    pub continuity_pack_depth: String,
    pub chapter_generation_mode: String,
    pub window_planning_horizon: i64,
}

impl Default for AiStrategyProfile {
    fn default() -> Self {
        Self {
            automation_default: "supervised".to_string(),
            review_strictness: 4,
            default_workflow_stack: vec!["chapter.plan".to_string(), "chapter.draft".to_string()],
            always_on_policy_skills: vec![],
            default_capability_bundles: vec![],
            state_write_policy: "chapter_confirmed".to_string(),
            continuity_pack_depth: "standard".to_string(),
            chapter_generation_mode: "plan_scene_draft".to_string(),
            window_planning_horizon: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectJson {
    pub schema_version: String,
    pub app_min_version: String,
    pub project_id: String,
    pub name: String,
    pub author: String,
    pub genre: String,
    pub target_words: i64,
    pub created_at: String,
    pub updated_at: String,
    pub database: String,
    pub manuscript_root: String,
    pub settings: ProjectSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectOpenResult {
    pub project_root: String,
    pub project: ProjectJson,
}

#[derive(Default)]
pub struct ProjectService;

impl ProjectService {
    pub fn validate_name(&self, name: &str) -> Result<String, AppErrorDto> {
        let normalized = name.trim();

        if normalized.is_empty() {
            return Err(
                AppErrorDto::new("PROJECT_NAME_INVALID", "项目名称不能为空", true)
                    .with_suggested_action("请填写 1-80 字的项目名称"),
            );
        }

        if normalized.chars().count() > 80 {
            return Err(AppErrorDto::new(
                "PROJECT_NAME_INVALID",
                "项目名称长度不能超过 80 字",
                true,
            )
            .with_suggested_action("请缩短项目名称"));
        }

        Ok(normalized.to_string())
    }

    pub fn create_project(
        &self,
        input: CreateProjectInput,
    ) -> Result<ProjectOpenResult, AppErrorDto> {
        let normalized_name = self.validate_name(&input.name)?;
        let normalized_genre = input.genre.trim().to_string();
        if normalized_genre.is_empty() {
            return Err(
                AppErrorDto::new("PROJECT_GENRE_REQUIRED", "题材不能为空", true)
                    .with_suggested_action("请填写题材"),
            );
        }

        let save_directory_path = Path::new(&input.save_directory);
        if !save_directory_path.is_absolute() {
            return Err(
                AppErrorDto::new("PROJECT_INVALID_PATH", "保存目录必须是绝对路径", true)
                    .with_suggested_action("请输入有效的 Windows 绝对路径"),
            );
        }
        if !save_directory_path.exists() || !save_directory_path.is_dir() {
            return Err(
                AppErrorDto::new("PROJECT_INVALID_PATH", "保存目录不存在或不可用", true)
                    .with_detail(input.save_directory.clone())
                    .with_suggested_action("请先创建目录后再新建项目"),
            );
        }

        let sanitized_directory_name = sanitize_project_directory_name(&normalized_name);
        let project_root_path = save_directory_path.join(sanitized_directory_name);
        reject_dev_watch_conflict(&project_root_path)?;
        if project_root_path.exists() {
            return Err(
                AppErrorDto::new("PROJECT_PATH_EXISTS", "目标目录已存在同名项目", true)
                    .with_detail(project_root_path.to_string_lossy())
                    .with_suggested_action("请更换项目名称或保存位置"),
            );
        }

        fs::create_dir(&project_root_path).map_err(|err| {
            AppErrorDto::new("PROJECT_CREATE_FAILED", "创建项目失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查目标目录权限或更换保存路径")
        })?;

        let result = self.create_project_inner(
            &project_root_path,
            &normalized_name,
            &normalized_genre,
            &input,
        );
        match result {
            Ok(project) => Ok(project),
            Err(err) => {
                let _ = fs::remove_dir_all(project_root_path);
                Err(err)
            }
        }
    }

    fn create_project_inner(
        &self,
        project_root_path: &Path,
        normalized_name: &str,
        normalized_genre: &str,
        input: &CreateProjectInput,
    ) -> Result<ProjectOpenResult, AppErrorDto> {
        initialize_project_directories(project_root_path).map_err(|err| {
            AppErrorDto::new("PROJECT_CREATE_FAILED", "创建项目失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查目标目录权限或更换保存路径")
        })?;

        let created_at = now_iso();
        initialize_database(project_root_path).map_err(|err| {
            AppErrorDto::new("PROJECT_CREATE_FAILED", "创建项目失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查数据库初始化权限")
        })?;

        let target_words = input.target_words.unwrap_or(300_000);
        let project_json = ProjectJson {
            schema_version: PROJECT_SCHEMA_VERSION.to_string(),
            app_min_version: PROJECT_APP_MIN_VERSION.to_string(),
            project_id: Uuid::new_v4().to_string(),
            name: normalized_name.to_string(),
            author: input.author.clone().unwrap_or_default(),
            genre: normalized_genre.to_string(),
            target_words,
            created_at: created_at.clone(),
            updated_at: created_at.clone(),
            database: "database/project.sqlite".to_string(),
            manuscript_root: "manuscript/chapters".to_string(),
            settings: ProjectSettings {
                default_narrative_pov: "third_limited".to_string(),
                writing_style: WritingStyle::default(),
                language: "zh-CN".to_string(),
                autosave_interval_ms: 5_000,
            },
        };

        write_project_json(project_root_path, &project_json).map_err(|err| {
            AppErrorDto::new("PROJECT_CREATE_FAILED", "创建项目失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 project.json 写入权限")
        })?;

        let writing_style_json = serde_json::to_string(&project_json.settings.writing_style)
            .map_err(|err| {
                AppErrorDto::new("PROJECT_CREATE_FAILED", "创建项目失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查默认写作风格配置")
            })?;
        let ai_strategy_profile_json = serde_json::to_string(&AiStrategyProfile::default())
            .map_err(|err| {
                AppErrorDto::new("PROJECT_CREATE_FAILED", "创建项目失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查默认 AI 策略配置")
            })?;

        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否可读写")
        })?;
        conn
      .execute(
        "
        INSERT INTO projects(
          id, name, author, genre, narrative_pov, writing_style, ai_strategy_profile,
          target_words, current_words, project_path, schema_version, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ",
        params![
          project_json.project_id,
          project_json.name,
          project_json.author,
          project_json.genre,
          project_json.settings.default_narrative_pov.clone(),
          writing_style_json,
          ai_strategy_profile_json,
          project_json.target_words,
          0_i64,
          project_root_path.to_string_lossy().to_string(),
          project_json.schema_version,
          created_at,
          project_json.updated_at
        ],
      )
      .map_err(|err| {
        AppErrorDto::new("PROJECT_CREATE_FAILED", "创建项目失败", true)
          .with_detail(err.to_string())
          .with_suggested_action("请检查项目数据库结构")
      })?;

        let project_root = project_root_path.to_string_lossy().to_string();
        let _ = mark_recent_project(&project_root);
        Ok(ProjectOpenResult {
            project_root,
            project: project_json,
        })
    }

    pub fn open_project(&self, project_root: &str) -> Result<ProjectOpenResult, AppErrorDto> {
        let normalized_root = project_root.trim();
        if normalized_root.is_empty() {
            return Err(
                AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录不能为空", true)
                    .with_suggested_action("请输入有效的 Windows 绝对路径"),
            );
        }

        let project_root_path = Path::new(normalized_root);
        if !project_root_path.is_absolute() {
            return Err(
                AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录必须是绝对路径", true)
                    .with_suggested_action("请输入有效的 Windows 绝对路径"),
            );
        }
        if !project_root_path.exists() || !project_root_path.is_dir() {
            return Err(
                AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录不存在或不可用", true)
                    .with_detail(normalized_root.to_string())
                    .with_suggested_action("请检查目录路径并重试"),
            );
        }

        reject_dev_watch_conflict(project_root_path)?;
        let project_json_path = project_root_path.join("project.json");
        let db_path = project_root_path.join("database").join("project.sqlite");

        if !project_json_path.exists() || !db_path.exists() {
            return Err(
                AppErrorDto::new("PROJECT_INVALID_PATH", "不是有效项目目录", true)
                    .with_suggested_action(
                        "请选择包含 project.json 和 database/project.sqlite 的目录",
                    ),
            );
        }

        let project = read_project_json(project_root_path).map_err(|err| {
            AppErrorDto::new("PROJECT_INVALID_JSON", "project.json 读取失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 project.json 是否损坏")
        })?;
        if project.schema_version != PROJECT_SCHEMA_VERSION {
            return Err(
                AppErrorDto::new("PROJECT_VERSION_UNSUPPORTED", "项目版本不兼容", false)
                    .with_detail(format!("schemaVersion={}", project.schema_version))
                    .with_suggested_action("请先执行项目迁移"),
            );
        }

        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;
        validate_project_db_paths(project_root_path, &conn)?;

        let _ = mark_recent_project(normalized_root);
        Ok(ProjectOpenResult {
            project_root: normalized_root.to_string(),
            project,
        })
    }

    pub fn list_recent_projects(&self) -> Result<Vec<RecentProjectItem>, AppErrorDto> {
        list_recent_projects().map_err(|err| {
            AppErrorDto::new("RECENT_PROJECTS_READ_FAILED", "读取最近项目失败", true)
                .with_detail(err.to_string())
        })
    }

    pub fn clear_recent_projects(&self) -> Result<(), AppErrorDto> {
        clear_recent_projects().map_err(|err| {
            AppErrorDto::new("RECENT_PROJECTS_CLEAR_FAILED", "清除最近项目失败", true)
                .with_detail(err.to_string())
        })
    }

    pub fn save_writing_style(
        &self,
        project_root: &str,
        style: &WritingStyle,
    ) -> Result<(), AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let style_json = serde_json::to_string(style).map_err(|err| {
            AppErrorDto::new("STYLE_SERIALIZE_FAILED", "无法序列化写作风格配置", true)
                .with_detail(err.to_string())
        })?;
        let now = now_iso();
        conn.execute(
            "UPDATE projects SET writing_style = ?1, updated_at = ?2 WHERE id = ?3",
            params![style_json, now, project_id],
        )
        .map_err(|err| {
            AppErrorDto::new("DB_WRITE_FAILED", "保存写作风格失败", true)
                .with_detail(err.to_string())
        })?;
        Ok(())
    }

    pub fn get_writing_style(&self, project_root: &str) -> Result<WritingStyle, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let style_json: Option<String> = conn
            .query_row(
                "SELECT writing_style FROM projects WHERE id = ?1",
                params![project_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(|err| {
                AppErrorDto::new("DB_QUERY_FAILED", "读取写作风格失败", true)
                    .with_detail(err.to_string())
            })?;

        match style_json {
            Some(raw) => serde_json::from_str::<WritingStyle>(&raw).map_err(|err| {
                AppErrorDto::new("STYLE_PARSE_FAILED", "写作风格配置损坏", true)
                    .with_detail(err.to_string())
            }),
            None => Ok(WritingStyle::default()),
        }
    }

    pub fn save_ai_strategy_profile(
        &self,
        project_root: &str,
        profile: &AiStrategyProfile,
    ) -> Result<(), AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let profile_json = serde_json::to_string(profile).map_err(|err| {
            AppErrorDto::new("STRATEGY_SERIALIZE_FAILED", "无法序列化 AI 策略配置", true)
                .with_detail(err.to_string())
        })?;
        let now = now_iso();
        conn.execute(
            "UPDATE projects SET ai_strategy_profile = ?1, updated_at = ?2 WHERE id = ?3",
            params![profile_json, now, project_id],
        )
        .map_err(|err| {
            AppErrorDto::new("STRATEGY_SAVE_FAILED", "保存 AI 策略失败", true)
                .with_detail(err.to_string())
        })?;
        Ok(())
    }

    pub fn get_ai_strategy_profile(
        &self,
        project_root: &str,
    ) -> Result<AiStrategyProfile, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let profile_json: Option<String> = conn
            .query_row(
                "SELECT ai_strategy_profile FROM projects WHERE id = ?1",
                params![project_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(|err| {
                AppErrorDto::new("STRATEGY_READ_FAILED", "读取 AI 策略失败", true)
                    .with_detail(err.to_string())
            })?;

        match profile_json {
            Some(raw) if raw.trim().is_empty() => Ok(AiStrategyProfile::default()),
            Some(raw) => serde_json::from_str::<AiStrategyProfile>(&raw).map_err(|err| {
                AppErrorDto::new("STRATEGY_PARSE_FAILED", "AI 策略配置损坏", true)
                    .with_detail(err.to_string())
            }),
            None => Ok(AiStrategyProfile::default()),
        }
    }
}

fn reject_dev_watch_conflict(project_root: &Path) -> Result<(), AppErrorDto> {
    if !cfg!(debug_assertions) {
        return Ok(());
    }
    let cwd = std::env::current_dir().map_err(|err| {
        AppErrorDto::new("PROJECT_PATH_CHECK_FAILED", "项目路径检查失败", true)
            .with_detail(err.to_string())
    })?;
    if is_path_within(project_root, &cwd) {
        return Err(AppErrorDto::new(
            "PROJECT_DEV_WATCH_CONFLICT",
            "开发模式下，请勿将项目放在代码仓库目录内",
            true,
        )
        .with_detail(project_root.to_string_lossy())
        .with_suggested_action("请将项目保存到仓库外目录（如 D:\\NovelProjects）"));
    }
    Ok(())
}

fn normalize_path_prefix(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn is_path_within(path: &Path, base: &Path) -> bool {
    let normalized_path = normalize_path_prefix(path);
    let normalized_base = normalize_path_prefix(base);
    if normalized_base.is_empty() {
        return false;
    }
    if normalized_path == normalized_base {
        return true;
    }
    normalized_path.starts_with(&(normalized_base + "\\"))
}

fn initialize_project_directories(project_root: &Path) -> Result<(), std::io::Error> {
    for dir in REQUIRED_DIRS {
        fs::create_dir_all(project_root.join(dir))?;
    }
    Ok(())
}

fn project_json_path(project_root: &Path) -> PathBuf {
    project_root.join("project.json")
}

fn write_project_json(
    project_root: &Path,
    project_json: &ProjectJson,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = project_json_path(project_root);
    let payload = serde_json::to_string_pretty(project_json)?;
    write_file_atomic(&path, &payload)?;
    Ok(())
}

fn read_project_json(project_root: &Path) -> Result<ProjectJson, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(project_json_path(project_root))?;
    Ok(serde_json::from_str::<ProjectJson>(&raw)?)
}

pub fn get_project_id(conn: &rusqlite::Connection) -> Result<String, AppErrorDto> {
    conn.query_row("SELECT id FROM projects LIMIT 1", [], |row| {
        row.get::<_, String>(0)
    })
    .map_err(|err| {
        AppErrorDto::new("PROJECT_NOT_INITIALIZED", "项目未初始化", false)
            .with_detail(err.to_string())
            .with_suggested_action("请重新创建或打开有效项目")
    })
}

fn validate_project_db_paths(
    project_root: &Path,
    conn: &rusqlite::Connection,
) -> Result<(), AppErrorDto> {
    validate_path_column(
        project_root,
        conn,
        "SELECT content_path FROM chapters WHERE is_deleted = 0",
        "chapters.content_path",
    )?;
    validate_path_column(
        project_root,
        conn,
        "SELECT file_path FROM snapshots",
        "snapshots.file_path",
    )?;
    Ok(())
}

fn validate_path_column(
    project_root: &Path,
    conn: &rusqlite::Connection,
    sql: &str,
    column_label: &str,
) -> Result<(), AppErrorDto> {
    let mut stmt = conn.prepare(sql).map_err(|err| {
        AppErrorDto::new("PROJECT_PATH_SCAN_FAILED", "项目路径校验失败", false)
            .with_detail(err.to_string())
    })?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| {
            AppErrorDto::new("PROJECT_PATH_SCAN_FAILED", "项目路径校验失败", false)
                .with_detail(err.to_string())
        })?;
    for row in rows {
        let stored_path = row.map_err(|err| {
            AppErrorDto::new("PROJECT_PATH_SCAN_FAILED", "项目路径校验失败", false)
                .with_detail(err.to_string())
        })?;
        resolve_project_relative_path(project_root, &stored_path).map_err(|detail| {
            AppErrorDto::new(
                "PROJECT_PATH_INVALID_ENTRY",
                "项目数据库包含非法路径记录",
                false,
            )
            .with_detail(format!("{column_label}: {detail}"))
            .with_suggested_action("请修复数据库中的路径字段后重试")
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use super::{AiStrategyProfile, CreateProjectInput, ProjectService, WritingStyle};
    use crate::infra::database::open_database;
    use crate::services::chapter_service::{ChapterInput, ChapterService};

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn validate_name_succeeds_for_normal_input() {
        let service = ProjectService;
        let output = service
            .validate_name("  长夜行舟  ")
            .expect("expected success");
        assert_eq!(output, "长夜行舟");
    }

    #[test]
    fn validate_name_rejects_empty_input_with_standard_error() {
        let service = ProjectService;
        let err = service
            .validate_name("   ")
            .expect_err("expected validation error");
        assert_eq!(err.code, "PROJECT_NAME_INVALID");
        assert!(err.recoverable);
        assert_eq!(err.message, "项目名称不能为空");
    }

    #[test]
    fn create_and_open_project_roundtrip_succeeds() {
        let workspace = create_temp_workspace();
        let service = ProjectService;

        let create_result = service
            .create_project(CreateProjectInput {
                name: "夜潮计划".to_string(),
                author: Some("测试作者".to_string()),
                genre: "玄幻".to_string(),
                target_words: Some(120_000),
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project should succeed");

        assert!(PathBuf::from(&create_result.project_root)
            .join("project.json")
            .exists());

        let reopen = service
            .open_project(&create_result.project_root)
            .expect("open project should succeed");
        assert_eq!(reopen.project.project_id, create_result.project.project_id);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn create_project_rejects_existing_directory_without_deleting_it() {
        let workspace = create_temp_workspace();
        let service = ProjectService;
        let existing_root = workspace.join("同名项目");
        fs::create_dir_all(&existing_root).expect("create existing root");
        let marker = existing_root.join("keep.txt");
        fs::write(&marker, "keep").expect("write marker");

        let err = service
            .create_project(CreateProjectInput {
                name: "同名项目".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect_err("expected directory exists error");
        assert_eq!(err.code, "PROJECT_PATH_EXISTS");
        assert!(marker.exists());

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn writing_style_save_and_get_roundtrip_succeeds() {
        let workspace = create_temp_workspace();
        let service = ProjectService;

        let create_result = service
            .create_project(CreateProjectInput {
                name: "风格测试".to_string(),
                author: Some("测试作者".to_string()),
                genre: "都市".to_string(),
                target_words: Some(60_000),
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project should succeed");

        let default_style = service
            .get_writing_style(&create_result.project_root)
            .expect("get default writing style should succeed");
        assert_eq!(default_style, WritingStyle::default());

        let custom_style = WritingStyle {
            language_style: "ornate".to_string(),
            description_density: 6,
            dialogue_ratio: 3,
            sentence_rhythm: "long".to_string(),
            atmosphere: "suspenseful".to_string(),
            psychological_depth: 7,
        };
        service
            .save_writing_style(&create_result.project_root, &custom_style)
            .expect("save writing style should succeed");

        let loaded_style = service
            .get_writing_style(&create_result.project_root)
            .expect("get saved writing style should succeed");
        assert_eq!(loaded_style, custom_style);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn ai_strategy_profile_save_and_get_roundtrip_succeeds() {
        let workspace = create_temp_workspace();
        let service = ProjectService;

        let create_result = service
            .create_project(CreateProjectInput {
                name: "策略测试".to_string(),
                author: Some("测试作者".to_string()),
                genre: "玄幻".to_string(),
                target_words: Some(90_000),
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project should succeed");

        let default_profile = service
            .get_ai_strategy_profile(&create_result.project_root)
            .expect("get default ai strategy profile should succeed");
        assert_eq!(default_profile, AiStrategyProfile::default());

        let custom_profile = AiStrategyProfile {
            automation_default: "confirm".to_string(),
            review_strictness: 5,
            default_workflow_stack: vec![
                "chapter.plan".to_string(),
                "chapter.draft".to_string(),
                "consistency.scan".to_string(),
            ],
            always_on_policy_skills: vec!["term-lock".to_string()],
            default_capability_bundles: vec!["emotion-flow".to_string()],
            state_write_policy: "manual_only".to_string(),
            continuity_pack_depth: "deep".to_string(),
            chapter_generation_mode: "plan_draft".to_string(),
            window_planning_horizon: 12,
        };
        service
            .save_ai_strategy_profile(&create_result.project_root, &custom_profile)
            .expect("save ai strategy profile should succeed");

        let loaded_profile = service
            .get_ai_strategy_profile(&create_result.project_root)
            .expect("get saved ai strategy profile should succeed");
        assert_eq!(loaded_profile, custom_profile);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn open_project_rejects_db_paths_that_escape_project_root() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;

        let create_result = project_service
            .create_project(CreateProjectInput {
                name: "路径安全".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project should succeed");
        let chapter = chapter_service
            .create_chapter(
                &create_result.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: None,
                    target_words: None,
                    status: None,
                },
            )
            .expect("chapter created");

        let project_path = PathBuf::from(&create_result.project_root);
        let conn = open_database(&project_path).expect("open database");
        conn.execute(
            "UPDATE chapters SET content_path = ?1 WHERE id = ?2",
            rusqlite::params!["../outside.md", chapter.id],
        )
        .expect("inject invalid path");

        let err = project_service
            .open_project(&create_result.project_root)
            .expect_err("open should fail for invalid db path");
        assert_eq!(err.code, "PROJECT_PATH_INVALID_ENTRY");

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn is_path_within_matches_same_and_nested_paths() {
        let base = PathBuf::from(r"F:\NovelForge");
        assert!(super::is_path_within(
            &PathBuf::from(r"F:\NovelForge"),
            &base
        ));
        assert!(super::is_path_within(
            &PathBuf::from(r"F:\NovelForge\workspace\demo"),
            &base
        ));
        assert!(!super::is_path_within(
            &PathBuf::from(r"F:\NovelForgeX\workspace"),
            &base
        ));
    }

    #[test]
    fn open_project_rejects_relative_root_path() {
        let service = ProjectService;
        let err = service
            .open_project("relative\\project")
            .expect_err("relative path should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
