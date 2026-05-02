use crate::errors::AppErrorDto;
use crate::services::blueprint_service::BlueprintCertaintyZones;
use crate::services::context_service::{CharacterSummary, WorldRuleSummary};

type CertaintyZones = BlueprintCertaintyZones;

const MUTATION_INTENT_KEYWORDS: &[&str] = &[
    "修改", "更改", "重写", "推翻", "删除", "改动", "替换", "取消", "放弃", "不再",
];
const PROMISE_BREAK_KEYWORDS: &[&str] = &["取消", "放弃", "违背", "跳过", "不再", "不兑现"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertaintyConflictType {
    Frozen,
    Promised,
}

impl CertaintyConflictType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Frozen => "frozen_zone",
            Self::Promised => "promised_zone",
        }
    }

    pub fn error_code(self) -> &'static str {
        match self {
            Self::Frozen => "PIPELINE_FREEZE_CONFLICT",
            Self::Promised => "PIPELINE_PROMISED_CONFLICT",
        }
    }

    fn display_label(self) -> &'static str {
        match self {
            Self::Frozen => "冻结区",
            Self::Promised => "承诺区",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FreezeConflict {
    pub conflict_type: CertaintyConflictType,
    pub matched_zone: String,
    pub matched_entity_type: Option<String>,
    pub matched_entity_id: Option<String>,
    pub matched_entity_name: Option<String>,
}

fn has_any_keyword(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| text.contains(keyword))
}

fn contains_entity_reference(haystack_lower: &str, candidate_lower: &str) -> bool {
    let candidate = candidate_lower.trim();
    if candidate.is_empty() {
        return false;
    }
    let requires_word_boundary = candidate
        .chars()
        .any(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
    if !requires_word_boundary {
        return haystack_lower.contains(candidate);
    }

    let mut start = 0usize;
    while let Some(found) = haystack_lower[start..].find(candidate) {
        let matched_start = start + found;
        let matched_end = matched_start + candidate.len();
        let before = haystack_lower[..matched_start].chars().next_back();
        let after = haystack_lower[matched_end..].chars().next();
        let left_ok = before
            .map(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
            .unwrap_or(true);
        let right_ok = after
            .map(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
            .unwrap_or(true);
        if left_ok && right_ok {
            return true;
        }
        start = matched_end;
    }
    false
}

fn parse_tagged_value(entry: &str, candidate_keys: &[&str]) -> Option<String> {
    let normalized = entry.trim().replace('：', ":");
    for separator in [':', '='] {
        if let Some((raw_key, raw_value)) = normalized.split_once(separator) {
            let key = raw_key.trim().to_ascii_lowercase();
            if candidate_keys
                .iter()
                .any(|candidate| key.eq_ignore_ascii_case(candidate))
            {
                let value = raw_value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn resolve_character_match(
    entry: &str,
    characters: &[CharacterSummary],
) -> Option<(String, String, String)> {
    let explicit_name = parse_tagged_value(
        entry,
        &["character", "character_name", "role", "角色", "角色名"],
    );
    if let Some(name) = explicit_name {
        if let Some(character) = characters
            .iter()
            .find(|item| item.name.eq_ignore_ascii_case(name.as_str()))
        {
            return Some((
                "character".to_string(),
                character.id.clone(),
                character.name.clone(),
            ));
        }
    }

    let candidate = entry.trim();
    if candidate.is_empty() {
        return None;
    }
    characters
        .iter()
        .find(|item| item.name.eq_ignore_ascii_case(candidate))
        .map(|character| {
            (
                "character".to_string(),
                character.id.clone(),
                character.name.clone(),
            )
        })
}

fn resolve_world_rule_match(
    entry: &str,
    world_rules: &[WorldRuleSummary],
) -> Option<(String, String, String)> {
    let explicit_id = parse_tagged_value(
        entry,
        &[
            "world_rule_id",
            "worldruleid",
            "world-rule-id",
            "rule_id",
            "world_rule",
            "世界规则id",
            "规则id",
        ],
    )?;
    let normalized_id = explicit_id.trim().to_ascii_lowercase();
    if normalized_id.is_empty() {
        return None;
    }
    world_rules
        .iter()
        .find(|item| item.id.trim().eq_ignore_ascii_case(normalized_id.as_str()))
        .map(|rule| {
            (
                "world_rule".to_string(),
                rule.id.clone(),
                rule.title.clone(),
            )
        })
}

fn detect_zone_conflict(
    instruction_lower: &str,
    entries: &[String],
    conflict_type: CertaintyConflictType,
    characters: &[CharacterSummary],
    world_rules: &[WorldRuleSummary],
) -> Option<FreezeConflict> {
    for entry in entries {
        let normalized = entry.trim();
        if normalized.is_empty() {
            continue;
        }
        if let Some((entity_type, entity_id, entity_name)) =
            resolve_character_match(normalized, characters)
        {
            let entity_name_lower = entity_name.to_ascii_lowercase();
            let entity_id_lower = entity_id.to_ascii_lowercase();
            if contains_entity_reference(instruction_lower, entity_name_lower.as_str())
                || contains_entity_reference(instruction_lower, entity_id_lower.as_str())
            {
                return Some(FreezeConflict {
                    conflict_type,
                    matched_zone: normalized.to_string(),
                    matched_entity_type: Some(entity_type),
                    matched_entity_id: Some(entity_id),
                    matched_entity_name: Some(entity_name),
                });
            }
            continue;
        }

        if let Some((entity_type, entity_id, entity_name)) =
            resolve_world_rule_match(normalized, world_rules)
        {
            let entity_id_lower = entity_id.to_ascii_lowercase();
            if contains_entity_reference(instruction_lower, entity_id_lower.as_str()) {
                return Some(FreezeConflict {
                    conflict_type,
                    matched_zone: normalized.to_string(),
                    matched_entity_type: Some(entity_type),
                    matched_entity_id: Some(entity_id),
                    matched_entity_name: Some(entity_name),
                });
            }
            continue;
        }

        let normalized_lower = normalized.to_ascii_lowercase();
        if contains_entity_reference(instruction_lower, normalized_lower.as_str()) {
            return Some(FreezeConflict {
                conflict_type,
                matched_zone: normalized.to_string(),
                matched_entity_type: None,
                matched_entity_id: None,
                matched_entity_name: None,
            });
        }
    }
    None
}

pub fn detect_freeze_conflict(
    user_instruction: &str,
    zones: &CertaintyZones,
    characters: &[CharacterSummary],
    world_rules: &[WorldRuleSummary],
) -> Option<FreezeConflict> {
    let normalized_instruction = user_instruction.trim();
    if normalized_instruction.is_empty() || !zones.has_any() {
        return None;
    }
    let lowered = normalized_instruction.to_ascii_lowercase();
    let has_mutation_intent = has_any_keyword(lowered.as_str(), MUTATION_INTENT_KEYWORDS);
    let has_promise_break_intent = has_any_keyword(lowered.as_str(), PROMISE_BREAK_KEYWORDS);
    if !has_mutation_intent && !has_promise_break_intent {
        return None;
    }

    if has_mutation_intent {
        if let Some(conflict) = detect_zone_conflict(
            lowered.as_str(),
            &zones.frozen,
            CertaintyConflictType::Frozen,
            characters,
            world_rules,
        ) {
            return Some(conflict);
        }
    }

    if has_promise_break_intent {
        if let Some(conflict) = detect_zone_conflict(
            lowered.as_str(),
            &zones.promised,
            CertaintyConflictType::Promised,
            characters,
            world_rules,
        ) {
            return Some(conflict);
        }
    }
    None
}

pub fn freeze_conflict_error(conflict: &FreezeConflict) -> AppErrorDto {
    let mut message = format!(
        "检测到{}冲突：请求涉及「{}」，已阻断执行",
        conflict.conflict_type.display_label(),
        conflict.matched_zone
    );
    if let Some(entity_name) = conflict.matched_entity_name.as_deref() {
        message.push_str(&format!("（命中实体：{}）", entity_name));
    }

    let mut error = AppErrorDto::new(conflict.conflict_type.error_code(), &message, true);
    if let (Some(entity_type), Some(entity_id)) = (
        conflict.matched_entity_type.as_deref(),
        conflict.matched_entity_id.as_deref(),
    ) {
        error = error.with_detail(format!(
            "conflictType={}, entityType={}, entityId={}",
            conflict.conflict_type.as_str(),
            entity_type,
            entity_id
        ));
    }
    let suggested_action = match conflict.conflict_type {
        CertaintyConflictType::Frozen => {
            "请在蓝图对应步骤的确定性分区调整冻结项，或修改指令避免改写冻结事实"
        }
        CertaintyConflictType::Promised => "请保持承诺项兑现，或先在蓝图对应步骤调整承诺区后再执行",
    };
    error.with_suggested_action(suggested_action)
}

#[cfg(test)]
mod tests {
    use super::{detect_freeze_conflict, freeze_conflict_error, CertaintyConflictType};
    use crate::services::blueprint_service::BlueprintCertaintyZones;
    use crate::services::context_service::{CharacterSummary, WorldRuleSummary};

    fn sample_character(id: &str, name: &str) -> CharacterSummary {
        CharacterSummary {
            id: id.to_string(),
            name: name.to_string(),
            role_type: "主角".to_string(),
            aliases: None,
            motivation: None,
            desire: None,
            fear: None,
            flaw: None,
            arc_stage: None,
            identity_text: None,
            appearance: None,
            locked_fields: None,
            source_kind: "user_input".to_string(),
            source_ref: None,
            source_request_id: None,
        }
    }

    fn sample_world_rule(id: &str, title: &str) -> WorldRuleSummary {
        WorldRuleSummary {
            id: id.to_string(),
            title: title.to_string(),
            category: "世界规则".to_string(),
            description: "desc".to_string(),
            constraint_level: "strong".to_string(),
            source_kind: "user_input".to_string(),
            source_ref: None,
            source_request_id: None,
        }
    }

    #[test]
    fn detect_freeze_conflict_matches_character_exact_name() {
        let zones = BlueprintCertaintyZones {
            frozen: vec!["角色:林夜".to_string()],
            promised: vec![],
            exploratory: vec![],
        };
        let conflict = detect_freeze_conflict(
            "请重写林夜的人设弧线",
            &zones,
            &[sample_character("c-1", "林夜")],
            &[],
        )
        .expect("should detect character freeze conflict");
        assert_eq!(conflict.conflict_type, CertaintyConflictType::Frozen);
        assert_eq!(conflict.matched_entity_type.as_deref(), Some("character"));
        assert_eq!(conflict.matched_entity_id.as_deref(), Some("c-1"));
    }

    #[test]
    fn detect_promised_conflict_when_instruction_breaks_promise() {
        let zones = BlueprintCertaintyZones {
            frozen: vec![],
            promised: vec!["角色:林夜".to_string()],
            exploratory: vec![],
        };
        let conflict = detect_freeze_conflict(
            "本章放弃林夜与师门和解这条承诺",
            &zones,
            &[sample_character("c-1", "林夜")],
            &[],
        )
        .expect("should detect promised conflict");
        assert_eq!(conflict.conflict_type, CertaintyConflictType::Promised);
    }

    #[test]
    fn detect_freeze_conflict_matches_world_rule_id() {
        let zones = BlueprintCertaintyZones {
            frozen: vec!["world_rule_id:wr-immutable-1".to_string()],
            promised: vec![],
            exploratory: vec![],
        };
        let conflict = detect_freeze_conflict(
            "请修改 world_rule_id=wr-immutable-1 对应的机制",
            &zones,
            &[],
            &[sample_world_rule("wr-immutable-1", "灵脉不可逆")],
        )
        .expect("should detect world rule id conflict");
        assert_eq!(conflict.conflict_type, CertaintyConflictType::Frozen);
        assert_eq!(conflict.matched_entity_type.as_deref(), Some("world_rule"));
        assert_eq!(
            conflict.matched_entity_id.as_deref(),
            Some("wr-immutable-1")
        );
    }

    #[test]
    fn freeze_conflict_error_uses_conflict_specific_code() {
        let zones = BlueprintCertaintyZones {
            frozen: vec!["角色:林夜".to_string()],
            promised: vec![],
            exploratory: vec![],
        };
        let conflict = detect_freeze_conflict(
            "请修改林夜设定",
            &zones,
            &[sample_character("c-1", "林夜")],
            &[],
        )
        .expect("should detect conflict");
        let err = freeze_conflict_error(&conflict);
        assert_eq!(err.code, "PIPELINE_FREEZE_CONFLICT");
        assert!(err.message.contains("冻结区"));
    }
}
