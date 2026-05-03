use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;
use std::path::Path;

// --- Data types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryStateSnapshot {
    pub id: String,
    pub project_id: String,
    pub chapter_id: String,
    pub snapshot_type: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub character_states: Vec<CharacterStateEntry>,
    pub plot_states: Vec<PlotStateEntry>,
    pub world_states: Vec<WorldStateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterStateEntry {
    pub id: String,
    pub snapshot_id: String,
    pub character_id: String,
    pub location: Option<String>,
    pub emotional_state: Option<String>,
    pub arc_progress: Option<String>,
    pub knowledge_gained: Option<String>,
    pub relationships_changed: Option<String>,
    pub status_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotStateEntry {
    pub id: String,
    pub snapshot_id: String,
    pub plot_node_id: Option<String>,
    pub progress_status: String,
    pub tension_level: Option<i32>,
    pub open_threads: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldStateEntry {
    pub id: String,
    pub snapshot_id: String,
    pub world_rule_id: Option<String>,
    pub state_description: String,
    pub changed_in_chapter: bool,
}

// --- Input types ---

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSnapshotInput {
    pub chapter_id: String,
    pub snapshot_type: Option<String>,
    pub notes: Option<String>,
    pub character_states: Vec<CreateCharacterStateInput>,
    pub plot_states: Vec<CreatePlotStateInput>,
    pub world_states: Vec<CreateWorldStateInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCharacterStateInput {
    pub character_id: String,
    pub location: Option<String>,
    pub emotional_state: Option<String>,
    pub arc_progress: Option<String>,
    pub knowledge_gained: Option<String>,
    pub relationships_changed: Option<String>,
    pub status_notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlotStateInput {
    pub plot_node_id: Option<String>,
    pub progress_status: String,
    pub tension_level: Option<i32>,
    pub open_threads: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorldStateInput {
    pub world_rule_id: Option<String>,
    pub state_description: String,
    pub changed_in_chapter: Option<bool>,
}

/// Compact summary for prompt injection (no full snapshot weight).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateSnapshotSummary {
    pub snapshot_id: String,
    pub chapter_id: String,
    pub snapshot_type: String,
    pub character_count: usize,
    pub plot_count: usize,
    pub world_count: usize,
}

// --- Service ---

#[derive(Default, Clone)]
pub struct StateTrackerService;

impl StateTrackerService {
    /// Create a full state snapshot for a chapter.
    pub fn create_snapshot(
        &self,
        project_root: &str,
        input: CreateSnapshotInput,
    ) -> Result<String, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let snapshot_id = Uuid::new_v4().to_string();
        let now = now_iso();
        let snapshot_type = input.snapshot_type.unwrap_or_else(|| "post_chapter".into());

        conn.execute(
            "INSERT INTO story_state_snapshots(id, project_id, chapter_id, snapshot_type, notes, created_at) \
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![snapshot_id, project_id, input.chapter_id, snapshot_type, input.notes, now],
        )
        .map_err(|e| {
            AppErrorDto::new("INSERT_FAILED", "创建状态快照失败", true).with_detail(e.to_string())
        })?;

        for cs in &input.character_states {
            let entry_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO character_state_entries(id, snapshot_id, character_id, location, \
                 emotional_state, arc_progress, knowledge_gained, relationships_changed, status_notes) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    entry_id,
                    snapshot_id,
                    cs.character_id,
                    cs.location,
                    cs.emotional_state,
                    cs.arc_progress,
                    cs.knowledge_gained,
                    cs.relationships_changed,
                    cs.status_notes
                ],
            )
            .map_err(|e| {
                AppErrorDto::new("INSERT_FAILED", "写入角色状态失败", true)
                    .with_detail(e.to_string())
            })?;
        }

        for ps in &input.plot_states {
            let entry_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO plot_state_entries(id, snapshot_id, plot_node_id, progress_status, \
                 tension_level, open_threads) VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    entry_id,
                    snapshot_id,
                    ps.plot_node_id,
                    ps.progress_status,
                    ps.tension_level,
                    ps.open_threads
                ],
            )
            .map_err(|e| {
                AppErrorDto::new("INSERT_FAILED", "写入情节状态失败", true)
                    .with_detail(e.to_string())
            })?;
        }

        for ws in &input.world_states {
            let entry_id = Uuid::new_v4().to_string();
            let changed = ws.changed_in_chapter.unwrap_or(false);
            conn.execute(
                "INSERT INTO world_state_entries(id, snapshot_id, world_rule_id, \
                 state_description, changed_in_chapter) VALUES (?1,?2,?3,?4,?5)",
                params![
                    entry_id,
                    snapshot_id,
                    ws.world_rule_id,
                    ws.state_description,
                    changed as i32
                ],
            )
            .map_err(|e| {
                AppErrorDto::new("INSERT_FAILED", "写入世界状态失败", true)
                    .with_detail(e.to_string())
            })?;
        }

        Ok(snapshot_id)
    }

    /// Get the latest snapshot for a given chapter.
    pub fn get_latest_snapshot(
        &self,
        project_root: &str,
        chapter_id: &str,
    ) -> Result<Option<StoryStateSnapshot>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;

        let snapshot_row: Option<(String, String, String, Option<String>, String)> = conn
            .query_row(
                "SELECT id, snapshot_type, chapter_id, notes, created_at \
                 FROM story_state_snapshots \
                 WHERE project_id = ?1 AND chapter_id = ?2 \
                 ORDER BY created_at DESC LIMIT 1",
                params![project_id, chapter_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询状态快照失败", true)
                    .with_detail(e.to_string())
            })?;

        let (snapshot_id, snapshot_type, ch_id, notes, created_at): (
            String,
            String,
            String,
            Option<String>,
            String,
        ) = match snapshot_row {
            Some(row) => row,
            None => return Ok(None),
        };

        let character_states = self.load_character_states(&conn, &snapshot_id)?;
        let plot_states = self.load_plot_states(&conn, &snapshot_id)?;
        let world_states = self.load_world_states(&conn, &snapshot_id)?;

        Ok(Some(StoryStateSnapshot {
            id: snapshot_id,
            project_id,
            chapter_id: ch_id,
            snapshot_type,
            notes,
            created_at,
            character_states,
            plot_states,
            world_states,
        }))
    }

    /// List all snapshots for a project (summary only, no child entries).
    pub fn list_snapshots(
        &self,
        project_root: &str,
    ) -> Result<Vec<StateSnapshotSummary>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;

        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.chapter_id, s.snapshot_type, \
                 (SELECT COUNT(*) FROM character_state_entries WHERE snapshot_id = s.id), \
                 (SELECT COUNT(*) FROM plot_state_entries WHERE snapshot_id = s.id), \
                 (SELECT COUNT(*) FROM world_state_entries WHERE snapshot_id = s.id) \
                 FROM story_state_snapshots s WHERE s.project_id = ?1 \
                 ORDER BY s.created_at DESC",
            )
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询快照列表失败", true)
                    .with_detail(e.to_string())
            })?;

        let summaries = stmt
            .query_map(params![project_id], |row| {
                Ok(StateSnapshotSummary {
                    snapshot_id: row.get(0)?,
                    chapter_id: row.get(1)?,
                    snapshot_type: row.get(2)?,
                    character_count: row.get::<_, i64>(3)? as usize,
                    plot_count: row.get::<_, i64>(4)? as usize,
                    world_count: row.get::<_, i64>(5)? as usize,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询快照列表失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询快照列表失败", true)
                    .with_detail(e.to_string())
            })?;

        Ok(summaries)
    }

    /// Delete a snapshot and all its child entries.
    pub fn delete_snapshot(
        &self,
        project_root: &str,
        snapshot_id: &str,
    ) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        // Delete children first
        for table in &[
            "character_state_entries",
            "plot_state_entries",
            "world_state_entries",
        ] {
            conn.execute(
                &format!("DELETE FROM {} WHERE snapshot_id = ?1", table),
                params![snapshot_id],
            )
            .map_err(|e| {
                AppErrorDto::new("DELETE_FAILED", "删除状态条目失败", true)
                    .with_detail(e.to_string())
            })?;
        }
        conn.execute(
            "DELETE FROM story_state_snapshots WHERE id = ?1",
            params![snapshot_id],
        )
        .map_err(|e| {
            AppErrorDto::new("DELETE_FAILED", "删除状态快照失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }

    /// Format the latest state snapshot for the previous chapter as prompt text.
    /// Used by PromptBuilder to inject continuity context.
    pub fn collect_state_for_prompt(
        &self,
        project_root: &str,
        chapter_id: &str,
    ) -> Result<String, AppErrorDto> {
        // Find the previous chapter to get its state snapshot
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;

        // Get current chapter's index
        let current_index: Option<i64> = conn
            .query_row(
                "SELECT sort_order FROM chapters WHERE id = ?1 AND project_id = ?2",
                params![chapter_id, project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询章节失败", true).with_detail(e.to_string())
            })?;

        let current_index = match current_index {
            Some(idx) => idx,
            None => return Ok(String::new()),
        };

        // Find previous chapter
        let prev_chapter_id: Option<String> = conn
            .query_row(
                "SELECT id FROM chapters \
                 WHERE project_id = ?1 AND sort_order < ?2 AND deleted = 0 \
                 ORDER BY sort_order DESC LIMIT 1",
                params![project_id, current_index],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询前一章节失败", true)
                    .with_detail(e.to_string())
            })?;

        let prev_id = match prev_chapter_id {
            Some(id) => id,
            None => return Ok(String::new()),
        };

        // Get the latest snapshot for the previous chapter
        let snapshot = self.get_latest_snapshot(project_root, &prev_id)?;
        match snapshot {
            None => Ok(String::new()),
            Some(snap) => Ok(self.format_snapshot_for_prompt(&snap)),
        }
    }

    // --- Private helpers ---

    fn load_character_states(
        &self,
        conn: &rusqlite::Connection,
        snapshot_id: &str,
    ) -> Result<Vec<CharacterStateEntry>, AppErrorDto> {
        let mut stmt = conn
            .prepare(
                "SELECT id, snapshot_id, character_id, location, emotional_state, \
                 arc_progress, knowledge_gained, relationships_changed, status_notes \
                 FROM character_state_entries WHERE snapshot_id = ?1",
            )
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色状态失败", true)
                    .with_detail(e.to_string())
            })?;
        let rows = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(CharacterStateEntry {
                    id: row.get(0)?,
                    snapshot_id: row.get(1)?,
                    character_id: row.get(2)?,
                    location: row.get(3)?,
                    emotional_state: row.get(4)?,
                    arc_progress: row.get(5)?,
                    knowledge_gained: row.get(6)?,
                    relationships_changed: row.get(7)?,
                    status_notes: row.get(8)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色状态失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色状态失败", true)
                    .with_detail(e.to_string())
            })?;
        Ok(rows)
    }

    fn load_plot_states(
        &self,
        conn: &rusqlite::Connection,
        snapshot_id: &str,
    ) -> Result<Vec<PlotStateEntry>, AppErrorDto> {
        let mut stmt = conn
            .prepare(
                "SELECT id, snapshot_id, plot_node_id, progress_status, tension_level, open_threads \
                 FROM plot_state_entries WHERE snapshot_id = ?1",
            )
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询情节状态失败", true)
                    .with_detail(e.to_string())
            })?;
        let rows = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(PlotStateEntry {
                    id: row.get(0)?,
                    snapshot_id: row.get(1)?,
                    plot_node_id: row.get(2)?,
                    progress_status: row.get(3)?,
                    tension_level: row.get(4)?,
                    open_threads: row.get(5)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询情节状态失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询情节状态失败", true)
                    .with_detail(e.to_string())
            })?;
        Ok(rows)
    }

    fn load_world_states(
        &self,
        conn: &rusqlite::Connection,
        snapshot_id: &str,
    ) -> Result<Vec<WorldStateEntry>, AppErrorDto> {
        let mut stmt = conn
            .prepare(
                "SELECT id, snapshot_id, world_rule_id, state_description, changed_in_chapter \
                 FROM world_state_entries WHERE snapshot_id = ?1",
            )
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询世界状态失败", true)
                    .with_detail(e.to_string())
            })?;
        let rows = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(WorldStateEntry {
                    id: row.get(0)?,
                    snapshot_id: row.get(1)?,
                    world_rule_id: row.get(2)?,
                    state_description: row.get(3)?,
                    changed_in_chapter: row.get::<_, i32>(4)? != 0,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询世界状态失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询世界状态失败", true)
                    .with_detail(e.to_string())
            })?;
        Ok(rows)
    }

    fn format_snapshot_for_prompt(&self, snapshot: &StoryStateSnapshot) -> String {
        let mut lines = vec!["# 前一章故事状态快照".to_string()];

        if !snapshot.character_states.is_empty() {
            lines.push("## 角色状态".to_string());
            for cs in &snapshot.character_states {
                let mut parts = vec![format!("- 角色ID: {}", cs.character_id)];
                if let Some(ref loc) = cs.location {
                    parts.push(format!("  位置: {}", loc));
                }
                if let Some(ref emo) = cs.emotional_state {
                    parts.push(format!("  情绪: {}", emo));
                }
                if let Some(ref arc) = cs.arc_progress {
                    parts.push(format!("  成长弧: {}", arc));
                }
                if let Some(ref kg) = cs.knowledge_gained {
                    parts.push(format!("  新获信息: {}", kg));
                }
                if let Some(ref rc) = cs.relationships_changed {
                    parts.push(format!("  关系变化: {}", rc));
                }
                lines.push(parts.join("\n"));
            }
        }

        if !snapshot.plot_states.is_empty() {
            lines.push("## 情节进度".to_string());
            for ps in &snapshot.plot_states {
                let mut desc = format!("- 状态: {}", ps.progress_status);
                if let Some(tl) = ps.tension_level {
                    desc.push_str(&format!(" 紧张度: {}/10", tl));
                }
                if let Some(ref ot) = ps.open_threads {
                    desc.push_str(&format!(" 未解决线索: {}", ot));
                }
                lines.push(desc);
            }
        }

        if !snapshot.world_states.is_empty() {
            let changed: Vec<_> = snapshot
                .world_states
                .iter()
                .filter(|ws| ws.changed_in_chapter)
                .collect();
            if !changed.is_empty() {
                lines.push("## 世界状态变化".to_string());
                for ws in changed {
                    lines.push(format!("- {}", ws.state_description));
                }
            }
        }

        if lines.len() <= 1 {
            return String::new();
        }
        lines.push(String::new());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::project_service::{CreateProjectInput, ProjectService};
    use std::fs;
    use std::path::PathBuf;

    fn create_temp_workspace() -> PathBuf {
        let w = std::env::temp_dir().join(format!("novelforge-state-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&w).expect("create temp workspace");
        w
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn create_test_project(ws: &PathBuf) -> String {
        let ps = ProjectService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "状态追踪测试".into(),
                author: None,
                genre: "科幻".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");
        project.project_root
    }

    fn create_test_chapter(project_root: &str) -> String {
        use crate::services::chapter_service::{ChapterInput, ChapterService};
        let cs = ChapterService::default();
        cs.create_chapter(
            project_root,
            ChapterInput {
                title: "测试章节".to_string(),
                summary: None,
                target_words: None,
                status: None,
            },
        )
        .expect("chapter created")
        .id
    }

    #[test]
    fn create_and_get_snapshot() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let chapter_id = create_test_chapter(&project_root);
        let svc = StateTrackerService;

        let snapshot_id = svc
            .create_snapshot(
                &project_root,
                CreateSnapshotInput {
                    chapter_id: chapter_id.clone(),
                    snapshot_type: None,
                    notes: Some("测试快照".into()),
                    character_states: vec![],
                    plot_states: vec![CreatePlotStateInput {
                        plot_node_id: None,
                        progress_status: "in_progress".into(),
                        tension_level: Some(5),
                        open_threads: Some("谁是幕后黑手".into()),
                    }],
                    world_states: vec![],
                },
            )
            .expect("create snapshot");

        let snap = svc
            .get_latest_snapshot(&project_root, &chapter_id)
            .expect("get snapshot")
            .expect("snapshot exists");

        assert_eq!(snap.id, snapshot_id);
        assert_eq!(snap.plot_states.len(), 1);
        assert_eq!(snap.plot_states[0].progress_status, "in_progress");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn delete_snapshot_cleans_children() {
        let ws = create_temp_workspace();
        let project_root = create_test_project(&ws);
        let chapter_id = create_test_chapter(&project_root);
        let svc = StateTrackerService;

        let snapshot_id = svc
            .create_snapshot(
                &project_root,
                CreateSnapshotInput {
                    chapter_id: chapter_id.clone(),
                    snapshot_type: None,
                    notes: None,
                    character_states: vec![],
                    plot_states: vec![CreatePlotStateInput {
                        plot_node_id: None,
                        progress_status: "resolved".into(),
                        tension_level: None,
                        open_threads: None,
                    }],
                    world_states: vec![],
                },
            )
            .expect("create");

        svc.delete_snapshot(&project_root, &snapshot_id)
            .expect("delete");

        let snap = svc
            .get_latest_snapshot(&project_root, &chapter_id)
            .expect("get");
        assert!(snap.is_none());

        remove_temp_workspace(&ws);
    }
}
