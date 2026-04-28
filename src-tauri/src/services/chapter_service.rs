use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::fs_utils::{read_text_if_exists, write_file_atomic};
use crate::infra::path_utils::{chapter_file_name, to_posix_relative};
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInput {
    pub title: String,
    pub summary: Option<String>,
    pub target_words: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterRecord {
    pub id: String,
    pub chapter_index: i64,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub target_words: i64,
    pub current_words: i64,
    pub content_path: String,
    pub volume_id: Option<String>,
    pub version: i64,
    pub updated_at: String,
}

impl ChapterRecord {
    pub fn display_title(&self) -> String {
        if self.title.is_empty() {
            format!("#{}", self.chapter_index)
        } else {
            format!("#{} {}", self.chapter_index, self.title)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntryRecord {
    pub chapter_id: String,
    pub chapter_index: i64,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub volume_id: Option<String>,
    pub volume_title: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveChapterOutput {
    pub current_words: i64,
    pub version: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutosaveDraftInput {
    pub project_root: String,
    pub chapter_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverDraftResult {
    pub has_newer_draft: bool,
    pub draft_content: Option<String>,
}

#[derive(Default)]
pub struct ChapterService;

impl ChapterService {
    pub fn list_chapters(&self, project_root: &str) -> Result<Vec<ChapterRecord>, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;

        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare(
                "
        SELECT id, chapter_index, title, summary, status, target_words, current_words, content_path, volume_id, version, updated_at
        FROM chapters
        WHERE project_id = ?1 AND is_deleted = 0
        ORDER BY chapter_index
        ",
            )
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_LIST_FAILED", "查询章节失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节表结构")
            })?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(ChapterRecord {
                    id: row.get(0)?,
                    chapter_index: row.get(1)?,
                    title: row.get(2)?,
                    summary: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    status: row.get(4)?,
                    target_words: row.get(5)?,
                    current_words: row.get(6)?,
                    content_path: row.get(7)?,
                    volume_id: row.get::<_, Option<String>>(8)?,
                    version: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_LIST_FAILED", "查询章节失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节查询语句")
            })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|err| {
            AppErrorDto::new("CHAPTER_LIST_FAILED", "查询章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节数据")
        })
    }

    pub fn list_timeline_entries(
        &self,
        project_root: &str,
    ) -> Result<Vec<TimelineEntryRecord>, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;

        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare(
                "
        SELECT ch.id, ch.chapter_index, ch.title, ch.summary, ch.status, ch.volume_id, v.title, ch.updated_at
        FROM chapters ch
        LEFT JOIN volumes v ON ch.volume_id = v.id
        WHERE ch.project_id = ?1 AND ch.is_deleted = 0
        ORDER BY ch.chapter_index
        ",
            )
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_LIST_FAILED", "查询章节失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节表结构")
            })?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(TimelineEntryRecord {
                    chapter_id: row.get(0)?,
                    chapter_index: row.get(1)?,
                    title: row.get(2)?,
                    summary: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    status: row.get(4)?,
                    volume_id: row.get::<_, Option<String>>(5)?,
                    volume_title: row.get::<_, Option<String>>(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_LIST_FAILED", "查询章节失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节查询语句")
            })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|err| {
            AppErrorDto::new("CHAPTER_LIST_FAILED", "查询章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节数据")
        })
    }

    /// Reorder chapters by updating their indices. Takes an ordered list of chapter IDs.
    pub fn reorder_chapters(
        &self,
        project_root: &str,
        ordered_ids: Vec<String>,
    ) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        for (i, chapter_id) in ordered_ids.iter().enumerate() {
            let new_index = (i + 1) as i64;
            conn.execute(
                "UPDATE chapters SET chapter_index = ?1, updated_at = ?2 WHERE id = ?3 AND project_id = ?4",
                params![new_index, now, chapter_id, project_id],
            ).map_err(|e| AppErrorDto::new("REORDER_FAILED", "重新排序失败", true).with_detail(e.to_string()))?;
        }
        Ok(())
    }

    pub fn create_chapter(
        &self,
        project_root: &str,
        input: ChapterInput,
    ) -> Result<ChapterRecord, AppErrorDto> {
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(AppErrorDto::new(
                "CHAPTER_TITLE_REQUIRED",
                "章节标题不能为空",
                true,
            ));
        }

        let project_root_path = Path::new(project_root);
        let mut conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;

        let project_id = get_project_id(&conn)?;
        let next_index = conn
            .query_row(
                "SELECT MAX(chapter_index) FROM chapters WHERE project_id = ?1 AND is_deleted = 0",
                params![project_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_CREATE_FAILED", "创建章节失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查数据库章节索引")
            })?
            .unwrap_or(0)
            + 1;

        let chapter_id = Uuid::new_v4().to_string();
        let created_at = now_iso();
        let file_name = chapter_file_name(next_index);
        let absolute_chapter_path = project_root_path
            .join("manuscript")
            .join("chapters")
            .join(&file_name);
        let relative_chapter_path = to_posix_relative(project_root_path, &absolute_chapter_path);
        let summary = input.summary.unwrap_or_default();
        let status = input.status.unwrap_or_else(|| "drafting".to_string());
        let target_words = input.target_words.unwrap_or(0);

        let markdown = build_chapter_markdown(ChapterMarkdownInput {
            id: &chapter_id,
            index: next_index,
            title: &title,
            status: &status,
            summary: &summary,
            word_count: 0,
            created_at: &created_at,
            updated_at: &created_at,
            content: "",
        });

        fs::write(&absolute_chapter_path, markdown).map_err(|err| {
            AppErrorDto::new("CHAPTER_CREATE_FAILED", "创建章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 manuscript/chapters 写入权限")
        })?;

        let tx = conn.transaction().map_err(|err| {
            AppErrorDto::new("CHAPTER_CREATE_FAILED", "创建章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查数据库写入权限")
        })?;
        let insert_result = tx.execute(
      "
      INSERT INTO chapters(
        id, project_id, chapter_index, title, summary, status, target_words, current_words, content_path, version, created_at, updated_at
      )
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
      ",
      params![
        chapter_id,
        project_id,
        next_index,
        title,
        summary,
        status,
        target_words,
        0_i64,
        relative_chapter_path,
        1_i64,
        created_at,
        created_at
      ],
    );

        match insert_result {
            Ok(_) => {
                tx.commit().map_err(|err| {
                    AppErrorDto::new("CHAPTER_CREATE_FAILED", "创建章节失败", true)
                        .with_detail(err.to_string())
                        .with_suggested_action("请检查数据库事务状态")
                })?;
            }
            Err(err) => {
                let _ = fs::remove_file(&absolute_chapter_path);
                return Err(
                    AppErrorDto::new("CHAPTER_CREATE_FAILED", "创建章节失败", true)
                        .with_detail(err.to_string())
                        .with_suggested_action("请检查章节索引是否冲突"),
                );
            }
        }

        Ok(ChapterRecord {
            id: chapter_id,
            chapter_index: next_index,
            title,
            summary,
            status,
            target_words,
            current_words: 0,
            content_path: relative_chapter_path,
            volume_id: None,
            version: 1,
            updated_at: created_at,
        })
    }

    pub fn save_chapter_content(
        &self,
        project_root: &str,
        chapter_id: &str,
        content: &str,
    ) -> Result<SaveChapterOutput, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;

        let chapter_row = conn
            .query_row(
                "
        SELECT id, chapter_index, title, summary, status, content_path, version, created_at
        FROM chapters
        WHERE id = ?1 AND is_deleted = 0
        ",
                params![chapter_id],
                |row| {
                    Ok(ChapterRow {
                        id: row.get(0)?,
                        chapter_index: row.get(1)?,
                        title: row.get(2)?,
                        summary: row.get::<_, Option<String>>(3)?,
                        status: row.get(4)?,
                        content_path: row.get(5)?,
                        version: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_SAVE_FAILED", "保存章节失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节数据是否完整")
            })?
            .ok_or_else(|| AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true))?;

        let absolute_path = project_root_path.join(&chapter_row.content_path);
        let updated_at = now_iso();
        let current_words = content_word_count(content);
        let next_version = chapter_row.version + 1;

        let markdown = build_chapter_markdown(ChapterMarkdownInput {
            id: &chapter_row.id,
            index: chapter_row.chapter_index,
            title: &chapter_row.title,
            status: &chapter_row.status,
            summary: chapter_row.summary.as_deref().unwrap_or(""),
            word_count: current_words,
            created_at: &chapter_row.created_at,
            updated_at: &updated_at,
            content,
        });
        write_file_atomic(&absolute_path, &markdown).map_err(|err| {
            AppErrorDto::new("CHAPTER_SAVE_FAILED", "保存章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节文件写入权限")
        })?;

        conn.execute(
            "
        UPDATE chapters
        SET current_words = ?1, version = version + 1, updated_at = ?2
        WHERE id = ?3
        ",
            params![current_words, updated_at, chapter_id],
        )
        .map_err(|err| {
            AppErrorDto::new("CHAPTER_SAVE_FAILED", "保存章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节数据库写入权限")
        })?;

        let draft_path = draft_path_from_content(project_root_path, &chapter_row.content_path)?;
        let _ = fs::remove_file(draft_path);

        Ok(SaveChapterOutput {
            current_words,
            version: next_version,
            updated_at,
        })
    }

    pub fn autosave_draft(
        &self,
        project_root: &str,
        chapter_id: &str,
        content: &str,
    ) -> Result<String, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;

        let content_path = conn
            .query_row(
                "SELECT content_path FROM chapters WHERE id = ?1 AND is_deleted = 0",
                params![chapter_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_AUTOSAVE_FAILED", "自动保存失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节是否可访问")
            })?
            .ok_or_else(|| AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true))?;

        let draft_path = draft_path_from_content(project_root_path, &content_path)?;
        fs::write(&draft_path, content).map_err(|err| {
            AppErrorDto::new("CHAPTER_AUTOSAVE_FAILED", "自动保存失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 manuscript/drafts 写入权限")
        })?;
        Ok(draft_path.to_string_lossy().to_string())
    }

    pub fn recover_draft(
        &self,
        project_root: &str,
        chapter_id: &str,
    ) -> Result<RecoverDraftResult, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;

        let content_path = conn
            .query_row(
                "SELECT content_path FROM chapters WHERE id = ?1 AND is_deleted = 0",
                params![chapter_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_RECOVER_FAILED", "恢复草稿失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节是否可访问")
            })?
            .ok_or_else(|| AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true))?;

        let chapter_file = project_root_path.join(&content_path);
        let draft_path = draft_path_from_content(project_root_path, &content_path)?;
        let draft_content = match read_text_if_exists(&draft_path).map_err(|err| {
            AppErrorDto::new("CHAPTER_RECOVER_FAILED", "恢复草稿失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查草稿文件读取权限")
        })? {
            Some(content) => content,
            None => {
                return Ok(RecoverDraftResult {
                    has_newer_draft: false,
                    draft_content: None,
                });
            }
        };

        let chapter_meta = fs::metadata(&chapter_file).map_err(|err| {
            AppErrorDto::new("CHAPTER_RECOVER_FAILED", "恢复草稿失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节文件是否存在")
        })?;
        let draft_meta = fs::metadata(&draft_path).map_err(|err| {
            AppErrorDto::new("CHAPTER_RECOVER_FAILED", "恢复草稿失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查草稿文件是否存在")
        })?;

        let chapter_mtime = chapter_meta.modified().map_err(|err| {
            AppErrorDto::new("CHAPTER_RECOVER_FAILED", "恢复草稿失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节文件时间戳")
        })?;
        let draft_mtime = draft_meta.modified().map_err(|err| {
            AppErrorDto::new("CHAPTER_RECOVER_FAILED", "恢复草稿失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查草稿文件时间戳")
        })?;

        if draft_mtime <= chapter_mtime {
            return Ok(RecoverDraftResult {
                has_newer_draft: false,
                draft_content: None,
            });
        }

        Ok(RecoverDraftResult {
            has_newer_draft: true,
            draft_content: Some(draft_content),
        })
    }

    pub fn delete_chapter(&self, project_root: &str, chapter_id: &str) -> Result<(), AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let mut conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;
        let project_id = get_project_id(&conn)?;
        let tx = conn.transaction().map_err(|err| {
            AppErrorDto::new("CHAPTER_DELETE_FAILED", "删除章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查数据库事务状态")
        })?;

        let chapter_row = tx
            .query_row(
                "SELECT chapter_index, content_path FROM chapters WHERE id = ?1 AND project_id = ?2 AND is_deleted = 0",
                params![chapter_id, project_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_DELETE_FAILED", "删除章节失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节数据是否完整")
            })?
            .ok_or_else(|| AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true))?;

        let tombstone_index = tx
            .query_row(
                "SELECT COALESCE(MAX(chapter_index), 0) + 1 FROM chapters WHERE project_id = ?1",
                params![project_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|err| {
                AppErrorDto::new("CHAPTER_DELETE_FAILED", "删除章节失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节索引状态")
            })?;
        let now = now_iso();

        tx.execute(
            "UPDATE chapters SET is_deleted = 1, chapter_index = ?1, updated_at = ?2 WHERE id = ?3 AND project_id = ?4",
            params![tombstone_index, now, chapter_id, project_id],
        )
        .map_err(|err| {
            AppErrorDto::new("CHAPTER_DELETE_FAILED", "删除章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节删除写入权限")
        })?;

        tx.execute(
            "UPDATE chapters SET chapter_index = chapter_index - 1, updated_at = ?1 WHERE project_id = ?2 AND is_deleted = 0 AND chapter_index > ?3",
            params![now, project_id, chapter_row.0],
        )
        .map_err(|err| {
            AppErrorDto::new("CHAPTER_DELETE_FAILED", "删除章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节索引重排逻辑")
        })?;

        tx.execute(
            "DELETE FROM chapter_links WHERE chapter_id = ?1",
            params![chapter_id],
        )
        .map_err(|err| {
            AppErrorDto::new("CHAPTER_DELETE_FAILED", "删除章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节关联数据清理逻辑")
        })?;

        tx.commit().map_err(|err| {
            AppErrorDto::new("CHAPTER_DELETE_FAILED", "删除章节失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查数据库事务状态")
        })?;

        if let Ok(draft_path) = draft_path_from_content(project_root_path, &chapter_row.1) {
            let _ = fs::remove_file(draft_path);
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ChapterRow {
    id: String,
    chapter_index: i64,
    title: String,
    summary: Option<String>,
    status: String,
    content_path: String,
    version: i64,
    created_at: String,
}

fn draft_path_from_content(
    project_root: &Path,
    content_path: &str,
) -> Result<PathBuf, AppErrorDto> {
    let base_name = Path::new(content_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AppErrorDto::new("CHAPTER_PATH_INVALID", "章节路径无效", false)
                .with_detail(content_path.to_string())
                .with_suggested_action("请检查章节记录中的 contentPath")
        })?;
    Ok(project_root
        .join("manuscript")
        .join("drafts")
        .join(format!("{base_name}.autosave.md")))
}

fn content_word_count(content: &str) -> i64 {
    content.chars().filter(|ch| !ch.is_whitespace()).count() as i64
}

struct ChapterMarkdownInput<'a> {
    id: &'a str,
    index: i64,
    title: &'a str,
    status: &'a str,
    summary: &'a str,
    word_count: i64,
    created_at: &'a str,
    updated_at: &'a str,
    content: &'a str,
}

fn build_chapter_markdown(input: ChapterMarkdownInput<'_>) -> String {
    let content = if input.content.trim().is_empty() {
        "正文从这里开始。".to_string()
    } else {
        input.content.trim().to_string()
    };

    vec![
        "---".to_string(),
        format!("id: {}", input.id),
        format!("index: {}", input.index),
        format!("title: {}", input.title),
        format!("status: {}", input.status),
        format!("summary: {}", input.summary),
        format!("wordCount: {}", input.word_count),
        format!("createdAt: {}", input.created_at),
        format!("updatedAt: {}", input.updated_at),
        "linkedPlotNodes: []".to_string(),
        "appearingCharacters: []".to_string(),
        "linkedWorldRules: []".to_string(),
        "---".to_string(),
        "".to_string(),
        format!("# {}", input.title),
        "".to_string(),
        content,
        "".to_string(),
    ]
    .join("\n")
}

// ── Snapshot / Version Management ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRecord {
    pub id: String,
    pub chapter_id: Option<String>,
    pub snapshot_type: String,
    pub title: Option<String>,
    pub file_path: String,
    pub note: Option<String>,
    pub created_at: String,
}

impl ChapterService {
    /// Create a manual snapshot of a chapter's current content.
    pub fn create_snapshot(
        &self,
        project_root: &str,
        chapter_id: &str,
        title: Option<&str>,
        note: Option<&str>,
    ) -> Result<SnapshotRecord, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;

        let (content_path, chapter_index) = conn
            .query_row(
                "SELECT content_path, chapter_index FROM chapters WHERE id = ?1 AND is_deleted = 0",
                params![chapter_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(|e| AppErrorDto::new("CHAPTER_QUERY_FAILED", "查询章节失败", true).with_detail(e.to_string()))?
            .ok_or_else(|| AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true))?;

        let content = read_text_if_exists(&project_root_path.join(&content_path))
            .map_err(|e| AppErrorDto::new("SNAPSHOT_FAILED", "读取章节文件失败", true).with_detail(e.to_string()))?
            .unwrap_or_default();

        let snapshot_id = Uuid::new_v4().to_string();
        let now = crate::infra::time::now_iso();
        let project_id = crate::services::project_service::get_project_id(&conn)?;

        let snapshot_dir = project_root_path
            .join("manuscript")
            .join("snapshots")
            .join(format!("ch-{:04}", chapter_index));
        fs::create_dir_all(&snapshot_dir).map_err(|e| {
            AppErrorDto::new("SNAPSHOT_FAILED", "创建快照目录失败", true).with_detail(e.to_string())
        })?;

        let safe_title = title.unwrap_or("snapshot");
        let file_name = format!("{}-{}.md", now.replace([':', '.'], "-"), safe_title);
        let snapshot_path = snapshot_dir.join(&file_name);
        write_file_atomic(&snapshot_path, &content).map_err(|e| {
            AppErrorDto::new("SNAPSHOT_FAILED", "写入快照文件失败", true).with_detail(e.to_string())
        })?;

        let relative_path =
            crate::infra::path_utils::to_posix_relative(project_root_path, &snapshot_path);

        conn.execute(
            "INSERT INTO snapshots(id, project_id, chapter_id, snapshot_type, title, file_path, note, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![snapshot_id, project_id, chapter_id, "manual", title, relative_path, note, now],
        ).map_err(|e| AppErrorDto::new("SNAPSHOT_FAILED", "保存快照记录失败", true).with_detail(e.to_string()))?;

        Ok(SnapshotRecord {
            id: snapshot_id,
            chapter_id: Some(chapter_id.to_string()),
            snapshot_type: "manual".to_string(),
            title: title.map(|s| s.to_string()),
            file_path: relative_path,
            note: note.map(|s| s.to_string()),
            created_at: now,
        })
    }

    /// List snapshots for a chapter or all snapshots.
    pub fn list_snapshots(
        &self,
        project_root: &str,
        chapter_id: Option<&str>,
    ) -> Result<Vec<SnapshotRecord>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;

        let (sql, param_str): (&str, Option<String>) = if let Some(cid) = chapter_id {
            ("SELECT id, chapter_id, snapshot_type, title, file_path, note, created_at FROM snapshots WHERE chapter_id = ?1 ORDER BY created_at DESC", Some(cid.to_string()))
        } else {
            ("SELECT id, chapter_id, snapshot_type, title, file_path, note, created_at FROM snapshots ORDER BY created_at DESC", None)
        };

        let mut stmt = conn.prepare(sql).map_err(|e| {
            AppErrorDto::new("QUERY_FAILED", "查询快照失败", true).with_detail(e.to_string())
        })?;

        let snapshots = if let Some(ref pid) = param_str {
            stmt.query_map(params![pid.as_str()], |row| {
                Ok(SnapshotRecord {
                    id: row.get(0)?,
                    chapter_id: row.get::<_, Option<String>>(1)?,
                    snapshot_type: row.get(2)?,
                    title: row.get::<_, Option<String>>(3)?,
                    file_path: row.get(4)?,
                    note: row.get::<_, Option<String>>(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询快照失败", true).with_detail(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect()
        } else {
            stmt.query_map([], |row| {
                Ok(SnapshotRecord {
                    id: row.get(0)?,
                    chapter_id: row.get::<_, Option<String>>(1)?,
                    snapshot_type: row.get(2)?,
                    title: row.get::<_, Option<String>>(3)?,
                    file_path: row.get(4)?,
                    note: row.get::<_, Option<String>>(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询快照失败", true).with_detail(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect()
        };

        Ok(snapshots)
    }

    /// Read a snapshot's content.
    pub fn read_snapshot_content(
        &self,
        project_root: &str,
        snapshot_id: &str,
    ) -> Result<String, AppErrorDto> {
        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;

        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM snapshots WHERE id = ?1",
                params![snapshot_id],
                |row| row.get(0),
            )
            .map_err(|e| AppErrorDto::new("SNAPSHOT_NOT_FOUND", "快照不存在", true).with_detail(e.to_string()))?;

        read_text_if_exists(&project_root_path.join(&file_path))
            .map_err(|e| AppErrorDto::new("SNAPSHOT_READ_FAILED", "读取快照文件失败", true).with_detail(e.to_string()))?
            .ok_or_else(|| AppErrorDto::new("SNAPSHOT_FILE_MISSING", "快照文件已不存在", false))
    }
}

// ── Volume Management ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeRecord {
    pub id: String,
    pub title: String,
    pub sort_order: i64,
    pub description: Option<String>,
    pub chapter_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVolumeInput {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Default)]
pub struct VolumeService;

impl VolumeService {
    pub fn list(&self, project_root: &str) -> Result<Vec<VolumeRecord>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = crate::services::project_service::get_project_id(&conn)?;

        let mut stmt = conn.prepare(
            "SELECT v.id, v.title, v.sort_order, v.description, COUNT(ch.id) as chapter_count, v.created_at, v.updated_at
             FROM volumes v LEFT JOIN chapters ch ON ch.volume_id = v.id AND ch.is_deleted = 0
             WHERE v.project_id = ?1 GROUP BY v.id ORDER BY v.sort_order"
        ).map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询卷失败", true).with_detail(e.to_string()))?;

        let volumes = stmt
            .query_map(params![project_id], |row| {
                Ok(VolumeRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    sort_order: row.get(2)?,
                    description: row.get::<_, Option<String>>(3)?,
                    chapter_count: row.get::<_, i64>(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询卷失败", true).with_detail(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(volumes)
    }

    pub fn create(&self, project_root: &str, input: CreateVolumeInput) -> Result<String, AppErrorDto> {
        if input.title.trim().is_empty() {
            return Err(AppErrorDto::new("VOLUME_TITLE_REQUIRED", "卷标题不能为空", true));
        }
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = crate::services::project_service::get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = crate::infra::time::now_iso();
        let next_order: i64 = conn
            .query_row("SELECT COALESCE(MAX(sort_order),0)+1 FROM volumes WHERE project_id = ?1", params![project_id], |row| row.get(0))
            .unwrap_or(1);
        conn.execute(
            "INSERT INTO volumes(id, project_id, title, sort_order, description, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, project_id, input.title.trim(), next_order, input.description, now, now],
        ).map_err(|e| AppErrorDto::new("INSERT_FAILED", "创建卷失败", true).with_detail(e.to_string()))?;
        Ok(id)
    }

    pub fn delete(&self, project_root: &str, id: &str) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        conn.execute("UPDATE chapters SET volume_id = NULL WHERE volume_id = ?1", params![id]).ok();
        conn.execute("DELETE FROM volumes WHERE id = ?1", params![id])
            .map_err(|e| AppErrorDto::new("DELETE_FAILED", "删除卷失败", true).with_detail(e.to_string()))?;
        Ok(())
    }

    pub fn assign_chapter(&self, project_root: &str, chapter_id: &str, volume_id: Option<&str>) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        conn.execute("UPDATE chapters SET volume_id = ?1 WHERE id = ?2", params![volume_id, chapter_id])
            .map_err(|e| AppErrorDto::new("UPDATE_FAILED", "分配卷失败", true).with_detail(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::thread::sleep;
    use std::time::Duration;

    use uuid::Uuid;

    use super::{ChapterInput, ChapterService};
    use crate::services::project_service::{CreateProjectInput, ProjectService};

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
    fn create_and_save_chapter_succeeds() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "章节流程".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");

        let chapter = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: Some("测试摘要".to_string()),
                    target_words: Some(2_000),
                    status: None,
                },
            )
            .expect("chapter created");
        assert_eq!(chapter.chapter_index, 1);

        let save = chapter_service
            .save_chapter_content(&project.project_root, &chapter.id, "夜潮降临。")
            .expect("save content");
        assert_eq!(save.version, 2);
        assert!(save.current_words > 0);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn autosave_and_recover_draft_succeeds() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "草稿恢复".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");
        let chapter = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: None,
                    target_words: None,
                    status: None,
                },
            )
            .expect("chapter created");

        chapter_service
            .save_chapter_content(&project.project_root, &chapter.id, "正式内容")
            .expect("save content");
        sleep(Duration::from_millis(20));
        chapter_service
            .autosave_draft(&project.project_root, &chapter.id, "更晚草稿")
            .expect("autosave draft");

        let recovered = chapter_service
            .recover_draft(&project.project_root, &chapter.id)
            .expect("recover draft");
        assert!(recovered.has_newer_draft);
        assert_eq!(recovered.draft_content.unwrap_or_default(), "更晚草稿");

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn delete_chapter_reindexes_remaining_chapters() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "章节删除".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");

        let first = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: None,
                    target_words: None,
                    status: None,
                },
            )
            .expect("first chapter created");
        let second = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第二章".to_string(),
                    summary: None,
                    target_words: None,
                    status: None,
                },
            )
            .expect("second chapter created");
        let third = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第三章".to_string(),
                    summary: None,
                    target_words: None,
                    status: None,
                },
            )
            .expect("third chapter created");

        chapter_service
            .delete_chapter(&project.project_root, &second.id)
            .expect("delete chapter");

        let chapters = chapter_service
            .list_chapters(&project.project_root)
            .expect("list chapters");
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].id, first.id);
        assert_eq!(chapters[0].chapter_index, 1);
        assert_eq!(chapters[1].id, third.id);
        assert_eq!(chapters[1].chapter_index, 2);

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn timeline_entries_are_sorted_by_chapter_index() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "时间线".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");

        let first = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: Some("开场".to_string()),
                    target_words: None,
                    status: None,
                },
            )
            .expect("first chapter created");
        let second = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第二章".to_string(),
                    summary: Some("推进".to_string()),
                    target_words: None,
                    status: None,
                },
            )
            .expect("second chapter created");

        let entries = chapter_service
            .list_timeline_entries(&project.project_root)
            .expect("list timeline entries");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].chapter_id, first.id);
        assert_eq!(entries[0].chapter_index, 1);
        assert_eq!(entries[1].chapter_id, second.id);
        assert_eq!(entries[1].chapter_index, 2);

        remove_temp_workspace(&workspace);
    }
}
