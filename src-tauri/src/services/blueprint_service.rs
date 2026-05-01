use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintCertaintyZones {
    #[serde(default)]
    pub frozen: Vec<String>,
    #[serde(default)]
    pub promised: Vec<String>,
    #[serde(default)]
    pub exploratory: Vec<String>,
}

impl BlueprintCertaintyZones {
    pub fn has_any(&self) -> bool {
        !self.frozen.is_empty() || !self.promised.is_empty() || !self.exploratory.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintStep {
    pub id: String,
    pub project_id: String,
    pub step_key: String,
    pub title: String,
    pub content: String,
    pub content_path: String,
    pub status: String,
    pub ai_generated: bool,
    pub certainty_zones: Option<BlueprintCertaintyZones>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBlueprintStepInput {
    pub step_key: String,
    pub content: String,
    pub ai_generated: Option<bool>,
    #[serde(default)]
    pub certainty_zones: Option<BlueprintCertaintyZones>,
}

#[derive(Default)]
pub struct BlueprintService;

#[derive(Clone, Copy)]
enum CertaintyZoneKind {
    Frozen,
    Promised,
    Exploratory,
}

const CERTAINTY_ZONE_SUPPORTED_STEPS: &[&str] = &[
    "step-08-chapters",
    "step-05-world",
    "step-04-characters",
    "step-07-plot",
    // Backward-compatible aliases used by older blueprint drafts.
    "step-02-world",
    "step-03-characters",
    "step-05-plot",
];

fn normalize_certainty_step_key(step_key: &str) -> String {
    match step_key.trim().to_ascii_lowercase().as_str() {
        "step-02-world" => "step-05-world".to_string(),
        "step-03-characters" => "step-04-characters".to_string(),
        "step-05-plot" => "step-07-plot".to_string(),
        other => other.to_string(),
    }
}

pub fn supports_certainty_zones_step(step_key: &str) -> bool {
    let normalized = normalize_certainty_step_key(step_key);
    CERTAINTY_ZONE_SUPPORTED_STEPS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(normalized.as_str()))
}

fn normalize_zone_line(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('-')
        .trim_start_matches('*')
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
        .trim()
        .to_string()
}

fn normalize_zone_key(raw: &str) -> String {
    normalize_zone_line(raw).to_ascii_lowercase()
}

fn normalize_zone_entries(entries: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for entry in entries {
        let line = normalize_zone_line(entry.as_str());
        if line.is_empty() {
            continue;
        }
        if normalized
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(line.as_str()))
        {
            continue;
        }
        normalized.push(line);
        if normalized.len() >= 24 {
            break;
        }
    }
    normalized
}

fn normalize_certainty_zones(zones: BlueprintCertaintyZones) -> Option<BlueprintCertaintyZones> {
    let normalized = BlueprintCertaintyZones {
        frozen: normalize_zone_entries(zones.frozen),
        promised: normalize_zone_entries(zones.promised),
        exploratory: normalize_zone_entries(zones.exploratory),
    };
    if normalized.has_any() {
        Some(normalized)
    } else {
        None
    }
}

fn validate_certainty_zone_conflicts(
    step_key: &str,
    zones: &Option<BlueprintCertaintyZones>,
) -> Result<(), AppErrorDto> {
    if !supports_certainty_zones_step(step_key) {
        return Ok(());
    }
    let Some(zones) = zones else {
        return Ok(());
    };

    let mut ownership = std::collections::BTreeMap::<String, String>::new();
    let mut overlaps = Vec::<String>::new();
    let register = |zone_label: &str,
                    entries: &[String],
                    ownership: &mut std::collections::BTreeMap<String, String>,
                    overlaps: &mut Vec<String>| {
        for entry in entries {
            let normalized = normalize_zone_key(entry);
            if normalized.is_empty() {
                continue;
            }
            if let Some(existing_zone) = ownership.get(&normalized) {
                if existing_zone != zone_label {
                    overlaps.push(format!("{}（{} / {}）", entry, existing_zone, zone_label));
                }
                continue;
            }
            ownership.insert(normalized, zone_label.to_string());
        }
    };

    register("冻结区", &zones.frozen, &mut ownership, &mut overlaps);
    register("承诺区", &zones.promised, &mut ownership, &mut overlaps);
    register("探索区", &zones.exploratory, &mut ownership, &mut overlaps);

    if overlaps.is_empty() {
        return Ok(());
    }
    overlaps.sort();
    overlaps.dedup();
    let detail = overlaps
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join("；");
    Err(
        AppErrorDto::new(
            "BLUEPRINT_CERTAINTY_ZONES_OVERLAP",
            "确定性分区冲突：同一条目不能同时出现在多个分区",
            true,
        )
        .with_detail(detail)
        .with_suggested_action("请将冲突条目只保留在一个分区（冻结区/承诺区/探索区）后再保存"),
    )
}

fn json_value_to_zone_items(value: &Value) -> Vec<String> {
    match value {
        Value::String(raw) => raw
            .split(['\n', ';', '；'])
            .map(normalize_zone_line)
            .filter(|item| !item.is_empty())
            .collect(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(raw) => Some(normalize_zone_line(raw)),
                Value::Number(number) => Some(number.to_string()),
                Value::Bool(value) => Some(value.to_string()),
                _ => None,
            })
            .filter(|item| !item.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_certainty_zones_from_json_value(value: &Value) -> Option<BlueprintCertaintyZones> {
    let obj = value.as_object()?;
    let mut candidate = BlueprintCertaintyZones::default();
    if let Some(raw) = obj.get("frozen") {
        candidate.frozen = json_value_to_zone_items(raw);
    }
    if let Some(raw) = obj.get("promised").or_else(|| obj.get("promise")) {
        candidate.promised = json_value_to_zone_items(raw);
    }
    if let Some(raw) = obj
        .get("exploratory")
        .or_else(|| obj.get("explore"))
        .or_else(|| obj.get("exploration"))
    {
        candidate.exploratory = json_value_to_zone_items(raw);
    }

    let nested = obj
        .get("certaintyZones")
        .or_else(|| obj.get("certainty_zones"))
        .or_else(|| obj.get("certaintyZone"))
        .or_else(|| obj.get("certainty_zone"));
    if let Some(value) = nested {
        if let Some(parsed) = parse_certainty_zones_from_json_value(value) {
            return normalize_certainty_zones(parsed);
        }
    }

    normalize_certainty_zones(candidate)
}

pub fn parse_certainty_zones_json(raw: &str) -> Option<BlueprintCertaintyZones> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = serde_json::from_str::<BlueprintCertaintyZones>(trimmed).ok()?;
    normalize_certainty_zones(parsed)
}

pub fn extract_certainty_zones_from_content(content: &str) -> Option<BlueprintCertaintyZones> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(parsed) = parse_certainty_zones_from_json_value(&value) {
            return Some(parsed);
        }
    }
    extract_certainty_zones_from_text(trimmed)
}

pub fn extract_certainty_zones_from_text(content: &str) -> Option<BlueprintCertaintyZones> {
    let mut frozen = Vec::new();
    let mut promised = Vec::new();
    let mut exploratory = Vec::new();
    let mut current_zone: Option<CertaintyZoneKind> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.contains("冻结区") {
            current_zone = Some(CertaintyZoneKind::Frozen);
            continue;
        }
        if line.contains("承诺区") {
            current_zone = Some(CertaintyZoneKind::Promised);
            continue;
        }
        if line.contains("探索区") {
            current_zone = Some(CertaintyZoneKind::Exploratory);
            continue;
        }

        let normalized = normalize_zone_line(line);
        if normalized.is_empty() {
            continue;
        }
        match current_zone {
            Some(CertaintyZoneKind::Frozen) => frozen.push(normalized),
            Some(CertaintyZoneKind::Promised) => promised.push(normalized),
            Some(CertaintyZoneKind::Exploratory) => exploratory.push(normalized),
            None => {}
        }
    }

    normalize_certainty_zones(BlueprintCertaintyZones {
        frozen,
        promised,
        exploratory,
    })
}

fn stringify_certainty_zones(zones: &Option<BlueprintCertaintyZones>) -> String {
    match zones {
        Some(value) => serde_json::to_string(value).unwrap_or_default(),
        None => String::new(),
    }
}

impl BlueprintService {
    pub fn list_steps(&self, project_root: &str) -> Result<Vec<BlueprintStep>, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn.prepare("SELECT id, project_id, step_key, title, COALESCE(content,''), COALESCE(content_path,''), status, ai_generated, COALESCE(certainty_zones_json,''), completed_at, created_at, updated_at FROM blueprint_steps WHERE project_id = ?1")
            .map_err(query_steps_error)?;
        let steps = stmt
            .query_map(params![project_id], |row| {
                let certainty_raw: String = row.get(8)?;
                Ok(BlueprintStep {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    step_key: row.get(2)?,
                    title: row.get(3)?,
                    content: row.get(4)?,
                    content_path: row.get(5)?,
                    status: row.get(6)?,
                    ai_generated: row.get::<_, i32>(7)? != 0,
                    certainty_zones: parse_certainty_zones_json(&certainty_raw),
                    completed_at: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(query_steps_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(query_steps_error)?;
        Ok(steps)
    }

    pub fn save_step(
        &self,
        project_root: &str,
        input: SaveBlueprintStepInput,
    ) -> Result<BlueprintStep, AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        let ai_gen = input.ai_generated.unwrap_or(false);
        let status = if input.content.trim().is_empty() {
            "not_started"
        } else {
            "in_progress"
        };

        let existing: Option<(String, String)> = conn
            .query_row(
                "SELECT id, COALESCE(certainty_zones_json,'') FROM blueprint_steps WHERE project_id = ?1 AND step_key = ?2",
                params![project_id, input.step_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();
        let existing_certainty_zones = existing
            .as_ref()
            .and_then(|(_, raw)| parse_certainty_zones_json(raw));
        let explicit_certainty_provided = input.certainty_zones.is_some();
        let explicit_certainty_zones = input
            .certainty_zones
            .as_ref()
            .cloned()
            .and_then(normalize_certainty_zones);
        let normalized_certainty_zones = if supports_certainty_zones_step(&input.step_key) {
            if input.content.trim().is_empty() && !explicit_certainty_provided {
                None
            } else if explicit_certainty_provided {
                explicit_certainty_zones
            } else {
                extract_certainty_zones_from_content(&input.content).or(existing_certainty_zones)
            }
        } else {
            None
        };
        validate_certainty_zone_conflicts(&input.step_key, &normalized_certainty_zones)?;
        let certainty_zones_json = stringify_certainty_zones(&normalized_certainty_zones);

        if let Some((id, _)) = existing {
            conn.execute(
                "UPDATE blueprint_steps SET content = ?1, status = ?2, ai_generated = ?3, certainty_zones_json = ?4, updated_at = ?5 WHERE id = ?6",
                params![
                    input.content,
                    status,
                    ai_gen as i32,
                    certainty_zones_json,
                    now,
                    id
                ],
            )
            .map_err(update_step_error)?;
        } else {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO blueprint_steps(id, project_id, step_key, title, content, content_path, status, ai_generated, certainty_zones_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    id,
                    project_id,
                    input.step_key,
                    "",
                    input.content,
                    "",
                    status,
                    ai_gen as i32,
                    certainty_zones_json,
                    now,
                    now
                ],
            )
            .map_err(insert_step_error)?;
        }

        load_step_by_key(&conn, &project_id, &input.step_key)
    }

    pub fn mark_completed(&self, project_root: &str, step_key: &str) -> Result<(), AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        conn.execute(
            "UPDATE blueprint_steps SET status = 'completed', completed_at = ?1, updated_at = ?2 WHERE project_id = ?3 AND step_key = ?4",
            params![now, now, project_id, step_key],
        )
        .map_err(mark_completed_error)?;
        Ok(())
    }

    pub fn reset_step(&self, project_root: &str, step_key: &str) -> Result<(), AppErrorDto> {
        let conn = open_project_database(project_root)?;
        let project_id = get_project_id(&conn)?;
        let now = now_iso();
        conn.execute(
            "UPDATE blueprint_steps SET content = '', status = 'not_started', ai_generated = 0, certainty_zones_json = '', completed_at = NULL, updated_at = ?1 WHERE project_id = ?2 AND step_key = ?3",
            params![now, project_id, step_key],
        )
        .map_err(reset_step_error)?;
        Ok(())
    }
}

fn normalize_project_root(project_root: &str) -> Result<&str, AppErrorDto> {
    let normalized_root = project_root.trim();
    if normalized_root.is_empty() {
        return Err(
            AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录不能为空", true)
                .with_suggested_action("请输入有效的项目目录路径"),
        );
    }
    Ok(normalized_root)
}

fn open_project_database(project_root: &str) -> Result<Connection, AppErrorDto> {
    let normalized_root = normalize_project_root(project_root)?;
    open_database(Path::new(normalized_root)).map_err(|e| {
        AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
    })
}

fn query_steps_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("QUERY_FAILED", "查询蓝图步骤失败", true).with_detail(err.to_string())
}

fn update_step_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATE_FAILED", "更新蓝图步骤失败", true).with_detail(err.to_string())
}

fn mark_completed_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATE_FAILED", "标记完成失败", true).with_detail(err.to_string())
}

fn reset_step_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("UPDATE_FAILED", "重置蓝图步骤失败", true).with_detail(err.to_string())
}

fn insert_step_error(err: impl ToString) -> AppErrorDto {
    AppErrorDto::new("INSERT_FAILED", "创建蓝图步骤失败", true).with_detail(err.to_string())
}

fn load_step_by_key(
    conn: &Connection,
    project_id: &str,
    step_key: &str,
) -> Result<BlueprintStep, AppErrorDto> {
    conn.query_row(
        "SELECT id, project_id, step_key, title, COALESCE(content,''), COALESCE(content_path,''), status, ai_generated, COALESCE(certainty_zones_json,''), completed_at, created_at, updated_at FROM blueprint_steps WHERE project_id = ?1 AND step_key = ?2",
        params![project_id, step_key],
        |row| {
            let certainty_raw: String = row.get(8)?;
            Ok(BlueprintStep {
                id: row.get(0)?,
                project_id: row.get(1)?,
                step_key: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                content_path: row.get(5)?,
                status: row.get(6)?,
                ai_generated: row.get::<_, i32>(7)? != 0,
                certainty_zones: parse_certainty_zones_json(&certainty_raw),
                completed_at: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        },
    )
    .map_err(query_steps_error)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    use super::{
        extract_certainty_zones_from_content, BlueprintCertaintyZones, BlueprintService,
        SaveBlueprintStepInput,
    };
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let w = std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&w).expect("create temp workspace");
        w
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn blueprint_save_and_mark_complete_succeeds() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "蓝图测试".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        bs.save_step(
            &project.project_root,
            SaveBlueprintStepInput {
                step_key: "step-01-anchor".into(),
                content: "核心灵感：秩序与代价。".into(),
                ai_generated: None,
                certainty_zones: None,
            },
        )
        .expect("save step");

        let steps = bs.list_steps(&project.project_root).expect("list steps");
        let step = steps
            .iter()
            .find(|s| s.step_key == "step-01-anchor")
            .unwrap();
        assert_eq!(step.content, "核心灵感：秩序与代价。");

        bs.mark_completed(&project.project_root, "step-01-anchor")
            .expect("mark completed");
        let steps = bs.list_steps(&project.project_root).expect("list steps");
        let step = steps
            .iter()
            .find(|s| s.step_key == "step-01-anchor")
            .unwrap();
        assert_eq!(step.status, "completed");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn blueprint_methods_accept_trimmed_project_root() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "蓝图路径空白测试".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let wrapped_root = format!("  {}  ", project.project_root);
        let saved = bs
            .save_step(
                &wrapped_root,
                SaveBlueprintStepInput {
                    step_key: "step-01-anchor".into(),
                    content: "测试内容".into(),
                    ai_generated: None,
                    certainty_zones: None,
                },
            )
            .expect("save step with trimmed root");
        assert_eq!(saved.step_key, "step-01-anchor");

        let steps = bs
            .list_steps(&wrapped_root)
            .expect("list steps with trimmed root");
        assert_eq!(steps.len(), 1);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn extract_certainty_zones_from_content_reads_nested_json_dto() {
        let parsed = extract_certainty_zones_from_content(
            r#"{
  "volumeStructure": "第一卷",
  "certaintyZones": {
    "frozen": ["终局真相"],
    "promised": ["主角将直面宗门审判"],
    "exploratory": ["支线人物立场可变化"]
  }
}"#,
        )
        .expect("parse certainty zones from nested json");
        assert_eq!(parsed.frozen, vec!["终局真相".to_string()]);
        assert_eq!(parsed.promised, vec!["主角将直面宗门审判".to_string()]);
        assert_eq!(parsed.exploratory, vec!["支线人物立场可变化".to_string()]);
    }

    #[test]
    fn save_step_persists_explicit_certainty_zones() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "蓝图确定性 DTO 测试".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let saved = bs
            .save_step(
                &project.project_root,
                SaveBlueprintStepInput {
                    step_key: "step-08-chapters".into(),
                    content: "{\"volumeStructure\":\"第一卷\"}".into(),
                    ai_generated: None,
                    certainty_zones: Some(BlueprintCertaintyZones {
                        frozen: vec!["终局真相".into()],
                        promised: vec!["主角将直面宗门审判".into()],
                        exploratory: vec!["支线人物立场可变化".into()],
                    }),
                },
            )
            .expect("save certainty zones");

        let certainty = saved
            .certainty_zones
            .expect("certainty zones should be persisted");
        assert_eq!(certainty.frozen, vec!["终局真相".to_string()]);
        assert_eq!(certainty.promised, vec!["主角将直面宗门审判".to_string()]);
        assert_eq!(certainty.exploratory, vec!["支线人物立场可变化".to_string()]);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn save_step_supports_certainty_zones_for_world_and_character_steps() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "蓝图多步骤确定性测试".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let world_saved = bs
            .save_step(
                &project.project_root,
                SaveBlueprintStepInput {
                    step_key: "step-05-world".into(),
                    content: "{\"rules\":\"代价机制\"}".into(),
                    ai_generated: None,
                    certainty_zones: Some(BlueprintCertaintyZones {
                        frozen: vec!["world_rule_id:wr-immutable-1".into()],
                        promised: vec!["代价必须兑现".into()],
                        exploratory: vec![],
                    }),
                },
            )
            .expect("save world certainty zones");
        let world_zones = world_saved
            .certainty_zones
            .expect("world certainty zones should be persisted");
        assert_eq!(world_zones.frozen, vec!["world_rule_id:wr-immutable-1"]);

        let alias_saved = bs
            .save_step(
                &project.project_root,
                SaveBlueprintStepInput {
                    step_key: "step-03-characters".into(),
                    content: "{\"protagonist\":\"林夜\"}".into(),
                    ai_generated: None,
                    certainty_zones: Some(BlueprintCertaintyZones {
                        frozen: vec!["角色:林夜".into()],
                        promised: vec![],
                        exploratory: vec![],
                    }),
                },
            )
            .expect("save character certainty zones with legacy alias key");
        let alias_zones = alias_saved
            .certainty_zones
            .expect("character certainty zones should be persisted");
        assert_eq!(alias_zones.frozen, vec!["角色:林夜"]);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn save_step_falls_back_to_legacy_text_partitions_for_certainty_zones() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "蓝图确定性回退测试".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let saved = bs
            .save_step(
                &project.project_root,
                SaveBlueprintStepInput {
                    step_key: "step-08-chapters".into(),
                    content:
                        "冻结区\n- 终局真相\n承诺区\n- 主角将直面宗门审判\n探索区\n- 支线人物立场可变化"
                            .into(),
                    ai_generated: None,
                    certainty_zones: None,
                },
            )
            .expect("save certainty zones by legacy text");

        let certainty = saved
            .certainty_zones
            .expect("certainty zones should be parsed from legacy text");
        assert_eq!(certainty.frozen, vec!["终局真相".to_string()]);
        assert_eq!(certainty.promised, vec!["主角将直面宗门审判".to_string()]);
        assert_eq!(certainty.exploratory, vec!["支线人物立场可变化".to_string()]);

        remove_temp_workspace(&ws);
    }

    #[test]
    fn save_step_rejects_overlapping_certainty_zone_entries() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let bs = BlueprintService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "蓝图确定性冲突测试".into(),
                author: None,
                genre: "测试".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let err = bs
            .save_step(
                &project.project_root,
                SaveBlueprintStepInput {
                    step_key: "step-08-chapters".into(),
                    content: "{\"volumeStructure\":\"第一卷\"}".into(),
                    ai_generated: None,
                    certainty_zones: Some(BlueprintCertaintyZones {
                        frozen: vec!["终局真相".into()],
                        promised: vec!["终局真相".into()],
                        exploratory: vec![],
                    }),
                },
            )
            .expect_err("overlap should be rejected");
        assert_eq!(err.code, "BLUEPRINT_CERTAINTY_ZONES_OVERLAP");

        remove_temp_workspace(&ws);
    }

    #[test]
    fn blueprint_methods_reject_blank_project_root() {
        let bs = BlueprintService;
        let err = bs
            .list_steps("   ")
            .expect_err("blank root should be rejected");
        assert_eq!(err.code, "PROJECT_INVALID_PATH");
    }
}
