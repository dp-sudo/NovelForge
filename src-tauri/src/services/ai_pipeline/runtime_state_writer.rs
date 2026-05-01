use crate::services::ai_pipeline_service::{PersistedRecord, RunAiTaskPipelineInput};
use crate::services::story_state_service::StoryStateInput;

pub fn should_persist_runtime_state_writes(
    canonical_task: &str,
    input: &RunAiTaskPipelineInput,
    persist_mode: &str,
    state_write_policy: &str,
) -> bool {
    match persist_mode.trim().to_ascii_lowercase().as_str() {
        "none" => return false,
        "derived_review" if !is_review_task(canonical_task) => return false,
        _ => {}
    }
    match state_write_policy.trim().to_ascii_lowercase().as_str() {
        "manual_only" => {
            is_promotion_action(input.ui_action.as_deref())
                || input
                    .automation_tier
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|tier| tier.eq_ignore_ascii_case("confirm"))
        }
        "chapter_confirmed" => {
            input
                .chapter_id
                .as_deref()
                .map(str::trim)
                .is_some_and(|chapter_id| !chapter_id.is_empty())
                || canonical_task.starts_with("chapter.")
                || is_promotion_action(input.ui_action.as_deref())
        }
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_runtime_story_state_input(
    canonical_task: &str,
    input: &RunAiTaskPipelineInput,
    normalized_output: &str,
    request_id: &str,
    persist_mode: &str,
    records: &[PersistedRecord],
    state_write_key: &str,
    active_skill_ids: &[String],
    affects_layers: &[String],
) -> Option<StoryStateInput> {
    let normalized_key = state_write_key.trim();
    if normalized_key.is_empty() {
        return None;
    }

    let (subject_type, state_kind) = normalized_key.split_once('.')?;
    let subject_type = subject_type.trim().to_ascii_lowercase();
    let state_kind = state_kind.trim().to_ascii_lowercase();
    if subject_type.is_empty() || state_kind.is_empty() {
        return None;
    }

    let subject_id = resolve_runtime_state_subject_id(&subject_type, input, records)?.to_string();
    let scope = if input
        .chapter_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|chapter_id| !chapter_id.is_empty())
    {
        "chapter".to_string()
    } else {
        "global".to_string()
    };
    let record_refs = records
        .iter()
        .map(|record| {
            serde_json::json!({
                "entityType": record.entity_type,
                "entityId": record.entity_id,
                "action": record.action,
            })
        })
        .collect::<Vec<_>>();

    Some(StoryStateInput {
        subject_type,
        subject_id,
        scope,
        state_kind,
        payload_json: serde_json::json!({
            "stateWriteKey": normalized_key,
            "taskType": canonical_task,
            "requestId": request_id,
            "uiAction": input.ui_action.as_deref().map(str::trim).filter(|value| !value.is_empty()),
            "persistMode": persist_mode.trim(),
            "automationTier": input.automation_tier.as_deref().map(str::trim).filter(|value| !value.is_empty()),
            "chapterId": input.chapter_id.as_deref().map(str::trim).filter(|value| !value.is_empty()),
            "skillIds": active_skill_ids,
            "affectsLayers": affects_layers,
            "recordRefs": record_refs,
            "outputPreview": preview_text(normalized_output, 240),
        }),
        source_chapter_id: input.chapter_id.clone(),
    })
}

fn is_review_task(canonical_task: &str) -> bool {
    canonical_task == "consistency.scan" || canonical_task.ends_with(".review")
}

fn is_promotion_action(ui_action: Option<&str>) -> bool {
    ui_action
        .map(str::trim)
        .map(|value| value.to_ascii_lowercase().contains("promote"))
        .unwrap_or(false)
}

fn resolve_runtime_state_subject_id<'a>(
    subject_type: &str,
    input: &'a RunAiTaskPipelineInput,
    records: &'a [PersistedRecord],
) -> Option<&'a str> {
    match subject_type {
        "chapter" => records
            .iter()
            .find(|record| record.entity_type == "chapter")
            .map(|record| record.entity_id.as_str())
            .or(input.chapter_id.as_deref()),
        "character" => records
            .iter()
            .find(|record| record.entity_type == "character")
            .map(|record| record.entity_id.as_str())
            .or_else(|| input.chapter_id.as_deref().map(|_| "current_character")),
        "scene" => Some("current_scene"),
        "relationship" => records
            .iter()
            .find(|record| record.entity_type == "character_relationship_batch")
            .map(|record| record.entity_id.as_str())
            .or_else(|| input.chapter_id.as_deref().map(|_| "current_relationship")),
        "window" => Some("current_window"),
        _ => Some("current_state"),
    }
}

fn preview_text(raw: &str, limit: usize) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut preview = String::new();
    for (index, ch) in trimmed.chars().enumerate() {
        if index >= limit {
            preview.push_str("...");
            break;
        }
        preview.push(ch);
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::should_persist_runtime_state_writes;
    use crate::services::ai_pipeline_service::RunAiTaskPipelineInput;

    fn build_pipeline_input(
        project_root: &str,
        task_type: &str,
        chapter_id: Option<String>,
    ) -> RunAiTaskPipelineInput {
        RunAiTaskPipelineInput {
            project_root: project_root.to_string(),
            task_type: task_type.to_string(),
            chapter_id,
            ui_action: None,
            user_instruction: "测试输入".to_string(),
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
            auto_persist: true,
            persist_mode: None,
            automation_tier: None,
            skill_selection: None,
        }
    }

    #[test]
    fn runtime_state_write_policy_respects_persist_mode_contract() {
        let mut chapter_input = build_pipeline_input(
            "C:\\tmp\\novelforge",
            "chapter.plan",
            Some("chapter-1".to_string()),
        );
        chapter_input.automation_tier = Some("confirm".to_string());
        chapter_input.ui_action = Some("book.pipeline.promote.manual".to_string());

        assert!(!should_persist_runtime_state_writes(
            "chapter.plan",
            &chapter_input,
            "none",
            "chapter_confirmed",
        ));
        assert!(!should_persist_runtime_state_writes(
            "chapter.plan",
            &chapter_input,
            "derived_review",
            "chapter_confirmed",
        ));
        assert!(should_persist_runtime_state_writes(
            "chapter.plan",
            &chapter_input,
            "formal",
            "chapter_confirmed",
        ));

        let review_input = build_pipeline_input(
            "C:\\tmp\\novelforge",
            "timeline.review",
            Some("chapter-1".to_string()),
        );
        assert!(should_persist_runtime_state_writes(
            "timeline.review",
            &review_input,
            "derived_review",
            "chapter_confirmed",
        ));
    }
}
