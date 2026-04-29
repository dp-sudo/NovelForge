use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub aliases: String,
    pub role_type: String,
    pub age: Option<String>,
    pub gender: Option<String>,
    pub identity_text: Option<String>,
    pub appearance: Option<String>,
    pub motivation: Option<String>,
    pub desire: Option<String>,
    pub fear: Option<String>,
    pub flaw: Option<String>,
    pub arc_stage: Option<String>,
    pub locked_fields: String,
    pub notes: Option<String>,
    pub is_deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCharacterInput {
    pub name: String,
    pub aliases: Option<Vec<String>>,
    pub role_type: String,
    pub age: Option<String>,
    pub gender: Option<String>,
    pub identity_text: Option<String>,
    pub appearance: Option<String>,
    pub motivation: Option<String>,
    pub desire: Option<String>,
    pub fear: Option<String>,
    pub flaw: Option<String>,
    pub arc_stage: Option<String>,
    pub locked_fields: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCharacterInput {
    pub id: String,
    pub name: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub role_type: Option<String>,
    pub age: Option<String>,
    pub gender: Option<String>,
    pub identity_text: Option<String>,
    pub appearance: Option<String>,
    pub motivation: Option<String>,
    pub desire: Option<String>,
    pub fear: Option<String>,
    pub flaw: Option<String>,
    pub arc_stage: Option<String>,
    pub locked_fields: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Default)]
pub struct CharacterService;

impl CharacterService {
    pub fn list(&self, project_root: &str) -> Result<Vec<CharacterRecord>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id, project_id, name, COALESCE(aliases,'[]'), role_type, age, gender, identity_text, appearance, motivation, desire, fear, flaw, arc_stage, COALESCE(locked_fields,'[]'), notes, is_deleted, created_at, updated_at FROM characters WHERE project_id = ?1 AND is_deleted = 0")
            .map_err(|e| AppErrorDto::new("QUERY_FAILED", "查询角色失败", true).with_detail(e.to_string()))?;
        let chars = stmt
            .query_map(params![project_id], |row| {
                Ok(CharacterRecord {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    aliases: row.get(3)?,
                    role_type: row.get(4)?,
                    age: row.get(5)?,
                    gender: row.get(6)?,
                    identity_text: row.get(7)?,
                    appearance: row.get(8)?,
                    motivation: row.get(9)?,
                    desire: row.get(10)?,
                    fear: row.get(11)?,
                    flaw: row.get(12)?,
                    arc_stage: row.get(13)?,
                    locked_fields: row.get(14)?,
                    notes: row.get(15)?,
                    is_deleted: row.get::<_, i32>(16)? != 0,
                    created_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色失败", true).with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色失败", true).with_detail(e.to_string())
            })?;
        Ok(chars)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreateCharacterInput,
    ) -> Result<String, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let aliases = serde_json::to_string(&input.aliases.unwrap_or_default()).unwrap_or_default();
        let locked =
            serde_json::to_string(&input.locked_fields.unwrap_or_default()).unwrap_or_default();
        conn.execute(
            "INSERT INTO characters(id, project_id, name, aliases, role_type, age, gender, identity_text, appearance, motivation, desire, fear, flaw, arc_stage, locked_fields, notes, is_deleted, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,0,?17,?18)",
            params![id, project_id, input.name, aliases, input.role_type, input.age, input.gender, input.identity_text, input.appearance, input.motivation, input.desire, input.fear, input.flaw, input.arc_stage, locked, input.notes, now, now],
        )
        .map_err(|e| AppErrorDto::new("INSERT_FAILED", "创建角色失败", true).with_detail(e.to_string()))?;
        Ok(id)
    }

    pub fn update(
        &self,
        project_root: &str,
        input: UpdateCharacterInput,
    ) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let now = now_iso();
        if let Some(name) = &input.name {
            conn.execute(
                "UPDATE characters SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(role) = &input.role_type {
            conn.execute(
                "UPDATE characters SET role_type = ?1, updated_at = ?2 WHERE id = ?3",
                params![role, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(age) = &input.age {
            conn.execute(
                "UPDATE characters SET age = ?1, updated_at = ?2 WHERE id = ?3",
                params![age, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(aliases) = &input.aliases {
            let aliases_json = serde_json::to_string(aliases).map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
            conn.execute(
                "UPDATE characters SET aliases = ?1, updated_at = ?2 WHERE id = ?3",
                params![aliases_json, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(gender) = &input.gender {
            conn.execute(
                "UPDATE characters SET gender = ?1, updated_at = ?2 WHERE id = ?3",
                params![gender, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(identity_text) = &input.identity_text {
            conn.execute(
                "UPDATE characters SET identity_text = ?1, updated_at = ?2 WHERE id = ?3",
                params![identity_text, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(appearance) = &input.appearance {
            conn.execute(
                "UPDATE characters SET appearance = ?1, updated_at = ?2 WHERE id = ?3",
                params![appearance, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(motivation) = &input.motivation {
            conn.execute(
                "UPDATE characters SET motivation = ?1, updated_at = ?2 WHERE id = ?3",
                params![motivation, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(desire) = &input.desire {
            conn.execute(
                "UPDATE characters SET desire = ?1, updated_at = ?2 WHERE id = ?3",
                params![desire, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(fear) = &input.fear {
            conn.execute(
                "UPDATE characters SET fear = ?1, updated_at = ?2 WHERE id = ?3",
                params![fear, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(flaw) = &input.flaw {
            conn.execute(
                "UPDATE characters SET flaw = ?1, updated_at = ?2 WHERE id = ?3",
                params![flaw, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(arc_stage) = &input.arc_stage {
            conn.execute(
                "UPDATE characters SET arc_stage = ?1, updated_at = ?2 WHERE id = ?3",
                params![arc_stage, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(locked_fields) = &input.locked_fields {
            let locked_json = serde_json::to_string(locked_fields).map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
            conn.execute(
                "UPDATE characters SET locked_fields = ?1, updated_at = ?2 WHERE id = ?3",
                params![locked_json, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        if let Some(notes) = &input.notes {
            conn.execute(
                "UPDATE characters SET notes = ?1, updated_at = ?2 WHERE id = ?3",
                params![notes, now, input.id],
            )
            .map_err(|e| {
                AppErrorDto::new("UPDATE_FAILED", "更新角色失败", true).with_detail(e.to_string())
            })?;
        }
        Ok(())
    }

    pub fn soft_delete(&self, project_root: &str, id: &str) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let now = now_iso();
        conn.execute(
            "UPDATE characters SET is_deleted = 1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| {
            AppErrorDto::new("DELETE_FAILED", "删除角色失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    use super::{CharacterService, CreateCharacterInput};
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
    fn character_create_and_list_succeeds() {
        let ws = create_temp_workspace();
        let ps = ProjectService;
        let cs = CharacterService;
        let project = ps
            .create_project(CreateProjectInput {
                name: "角色测试".into(),
                author: None,
                genre: "玄幻".into(),
                target_words: None,
                save_directory: ws.to_string_lossy().into(),
            })
            .expect("project created");

        let id = cs
            .create(
                &project.project_root,
                CreateCharacterInput {
                    name: "沈烬".into(),
                    role_type: "主角".into(),
                    aliases: Some(vec!["阿烬".into()]),
                    motivation: Some("查清真相".into()),
                    ..Default::default()
                },
            )
            .expect("create character");
        assert!(!id.is_empty());

        let chars = cs.list(&project.project_root).expect("list characters");
        assert_eq!(chars.len(), 1);
        assert_eq!(chars[0].name, "沈烬");

        remove_temp_workspace(&ws);
    }
}

impl Default for CreateCharacterInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            role_type: String::new(),
            aliases: None,
            age: None,
            gender: None,
            identity_text: None,
            appearance: None,
            motivation: None,
            desire: None,
            fear: None,
            flaw: None,
            arc_stage: None,
            locked_fields: None,
            notes: None,
        }
    }
}

// ── Character Relationships ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterRelationship {
    pub id: String,
    pub source_character_id: String,
    pub target_character_id: String,
    pub relationship_type: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRelationshipInput {
    pub source_character_id: String,
    pub target_character_id: String,
    pub relationship_type: String,
    pub description: Option<String>,
}

#[derive(Default)]
pub struct RelationshipService;

impl RelationshipService {
    pub fn list(
        &self,
        project_root: &str,
        character_id: Option<&str>,
    ) -> Result<Vec<CharacterRelationship>, AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;

        let sql = if character_id.is_some() {
            "SELECT id, source_character_id, target_character_id, relationship_type, description, created_at, updated_at FROM character_relationships WHERE source_character_id = ?1 OR target_character_id = ?1"
        } else {
            "SELECT id, source_character_id, target_character_id, relationship_type, description, created_at, updated_at FROM character_relationships"
        };

        let mut stmt = conn.prepare(sql).map_err(|e| {
            AppErrorDto::new("QUERY_FAILED", "查询角色关系失败", true).with_detail(e.to_string())
        })?;

        let rows = if let Some(cid) = character_id {
            stmt.query_map(params![cid], |row| {
                Ok(CharacterRelationship {
                    id: row.get(0)?,
                    source_character_id: row.get(1)?,
                    target_character_id: row.get(2)?,
                    relationship_type: row.get(3)?,
                    description: row.get::<_, Option<String>>(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色关系失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色关系失败", true)
                    .with_detail(e.to_string())
            })?
        } else {
            stmt.query_map([], |row| {
                Ok(CharacterRelationship {
                    id: row.get(0)?,
                    source_character_id: row.get(1)?,
                    target_character_id: row.get(2)?,
                    relationship_type: row.get(3)?,
                    description: row.get::<_, Option<String>>(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色关系失败", true)
                    .with_detail(e.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                AppErrorDto::new("QUERY_FAILED", "查询角色关系失败", true)
                    .with_detail(e.to_string())
            })?
        };

        Ok(rows)
    }

    pub fn create(
        &self,
        project_root: &str,
        input: CreateRelationshipInput,
    ) -> Result<String, AppErrorDto> {
        if input.source_character_id == input.target_character_id {
            return Err(AppErrorDto::new(
                "INVALID_RELATIONSHIP",
                "角色不能与自己建立关系",
                true,
            ));
        }
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        let project_id = get_project_id(&conn)?;
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        conn.execute(
            "INSERT INTO character_relationships(id, project_id, source_character_id, target_character_id, relationship_type, description, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, project_id, input.source_character_id, input.target_character_id, input.relationship_type, input.description, now, now],
        ).map_err(|e| AppErrorDto::new("INSERT_FAILED", "创建角色关系失败", true).with_detail(e.to_string()))?;
        Ok(id)
    }

    pub fn delete(&self, project_root: &str, id: &str) -> Result<(), AppErrorDto> {
        let conn = open_database(Path::new(project_root)).map_err(|e| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false).with_detail(e.to_string())
        })?;
        conn.execute(
            "DELETE FROM character_relationships WHERE id = ?1",
            params![id],
        )
        .map_err(|e| {
            AppErrorDto::new("DELETE_FAILED", "删除角色关系失败", true).with_detail(e.to_string())
        })?;
        Ok(())
    }
}
