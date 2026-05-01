use crate::errors::AppErrorDto;
use crate::services::blueprint_service::BlueprintCertaintyZones;

type CertaintyZones = BlueprintCertaintyZones;

#[derive(Debug, Clone)]
pub struct FreezeConflict {
    pub matched_zone: String,
}

pub fn detect_freeze_conflict(
    user_instruction: &str,
    zones: &CertaintyZones,
) -> Option<FreezeConflict> {
    let normalized_instruction = user_instruction.trim();
    if normalized_instruction.is_empty() || zones.frozen.is_empty() {
        return None;
    }
    let lowered = normalized_instruction.to_ascii_lowercase();
    let mut has_mutation_intent = false;
    for keyword in ["修改", "更改", "重写", "推翻", "删除", "改动", "替换"] {
        if lowered.contains(keyword) {
            has_mutation_intent = true;
            break;
        }
    }
    if !has_mutation_intent {
        return None;
    }
    for frozen_item in &zones.frozen {
        let candidate = frozen_item.trim();
        if candidate.chars().count() < 2 {
            continue;
        }
        if lowered.contains(&candidate.to_ascii_lowercase()) {
            return Some(FreezeConflict {
                matched_zone: candidate.to_string(),
            });
        }
    }
    None
}

pub fn freeze_conflict_error(conflict: &FreezeConflict) -> AppErrorDto {
    AppErrorDto::new(
        "PIPELINE_FREEZE_CONFLICT",
        &format!(
            "检测到冻结区冲突：请求涉及改写冻结项「{}」，已阻断执行",
            conflict.matched_zone
        ),
        true,
    )
    .with_suggested_action("请在蓝图 > 章节路线 > 确定性分区调整冻结区，或修改指令避免改写冻结事实")
}

#[cfg(test)]
mod tests {
    use super::{detect_freeze_conflict, freeze_conflict_error, FreezeConflict};
    use crate::services::blueprint_service::BlueprintCertaintyZones;

    #[test]
    fn detect_freeze_conflict_flags_mutation_on_frozen_items() {
        let zones = BlueprintCertaintyZones {
            frozen: vec!["终局真相".to_string()],
            promised: vec![],
            exploratory: vec![],
        };
        assert!(detect_freeze_conflict("请重写终局真相的揭示方式", &zones).is_some());
        assert!(detect_freeze_conflict("补充一个新支线", &zones).is_none());
    }

    #[test]
    fn freeze_conflict_error_uses_blocking_code() {
        let err = freeze_conflict_error(&FreezeConflict {
            matched_zone: "终局真相".to_string(),
        });
        assert_eq!(err.code, "PIPELINE_FREEZE_CONFLICT");
        assert!(err.message.contains("终局真相"));
    }
}
