use serde::Serialize;
use serde_json::json;

use crate::services::ai_pipeline::scene_classifier::SceneClassifier;
use crate::services::context_service::ContextService;
use crate::services::story_state_service::{StoryStateInput, StoryStateService};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostTaskResult {
    pub task_type: String,
    pub status: String,
    pub summary: String,
    pub error: Option<String>,
    pub meta: Option<serde_json::Value>,
}

#[derive(Default, Clone)]
pub struct PostTaskExecutor;

fn infer_emotion(text: &str) -> Option<&'static str> {
    if text.contains("愤怒") || text.contains("怒火") || text.contains("震怒") {
        return Some("anger");
    }
    if text.contains("悲伤") || text.contains("哀伤") || text.contains("落寞") {
        return Some("sadness");
    }
    if text.contains("紧张") || text.contains("惶恐") || text.contains("不安") {
        return Some("anxiety");
    }
    if text.contains("喜悦") || text.contains("欣喜") || text.contains("宽慰") {
        return Some("joy");
    }
    None
}

fn infer_relationship_temperature(text: &str) -> Option<&'static str> {
    if text.contains("信任")
        || text.contains("并肩")
        || text.contains("拥抱")
        || text.contains("和解")
    {
        return Some("warm");
    }
    if text.contains("敌视")
        || text.contains("争吵")
        || text.contains("背叛")
        || text.contains("对立")
    {
        return Some("cold");
    }
    None
}

fn infer_injury_state(text: &str) -> Option<&'static str> {
    if text.contains("重伤") || text.contains("骨折") {
        return Some("severe");
    }
    if text.contains("受伤") || text.contains("流血") {
        return Some("light");
    }
    None
}

fn infer_stamina_state(text: &str) -> Option<&'static str> {
    if text.contains("力竭") || text.contains("虚脱") {
        return Some("critical");
    }
    if text.contains("疲惫") || text.contains("乏力") {
        return Some("low");
    }
    None
}

impl PostTaskExecutor {
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        project_root: &str,
        chapter_id: Option<&str>,
        scene_type: &str,
        route_post_tasks: &[String],
        normalized_output: &str,
        context_service: &ContextService,
    ) -> Vec<PostTaskResult> {
        let mut merged = SceneClassifier::default_post_tasks(scene_type);
        for task in route_post_tasks {
            let normalized = task.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                continue;
            }
            if !merged
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(normalized.as_str()))
            {
                merged.push(normalized);
            }
        }

        let mut results = Vec::new();
        for task in merged {
            let result = match task.as_str() {
                "review_continuity" => self.review_continuity(project_root, chapter_id, context_service),
                "extract_assets" => self.extract_assets(project_root, chapter_id, context_service),
                "extract_state" => self.extract_state(project_root, chapter_id, scene_type, normalized_output),
                _ => PostTaskResult {
                    task_type: task.clone(),
                    status: "skipped".to_string(),
                    summary: "unsupported post task".to_string(),
                    error: None,
                    meta: None,
                },
            };
            results.push(result);
        }
        results
    }

    fn review_continuity(
        &self,
        project_root: &str,
        chapter_id: Option<&str>,
        context_service: &ContextService,
    ) -> PostTaskResult {
        let Some(chapter_id) = chapter_id.map(str::trim).filter(|value| !value.is_empty()) else {
            return PostTaskResult {
                task_type: "review_continuity".to_string(),
                status: "skipped".to_string(),
                summary: "chapter_id missing".to_string(),
                error: None,
                meta: None,
            };
        };

        match context_service.get_recent_continuity(project_root, Some(chapter_id)) {
            Ok(lines) => PostTaskResult {
                task_type: "review_continuity".to_string(),
                status: "succeeded".to_string(),
                summary: format!("reviewed {} continuity entries", lines.len()),
                error: None,
                meta: Some(json!({
                    "entryCount": lines.len(),
                    "sample": lines.into_iter().take(3).collect::<Vec<_>>(),
                })),
            },
            Err(err) => PostTaskResult {
                task_type: "review_continuity".to_string(),
                status: "failed".to_string(),
                summary: "continuity review failed".to_string(),
                error: Some(format!("{}: {}", err.code, err.message)),
                meta: None,
            },
        }
    }

    fn extract_assets(
        &self,
        project_root: &str,
        chapter_id: Option<&str>,
        context_service: &ContextService,
    ) -> PostTaskResult {
        let Some(chapter_id) = chapter_id.map(str::trim).filter(|value| !value.is_empty()) else {
            return PostTaskResult {
                task_type: "extract_assets".to_string(),
                status: "skipped".to_string(),
                summary: "chapter_id missing".to_string(),
                error: None,
                meta: None,
            };
        };

        match context_service.extract_and_persist_structured_drafts(project_root, chapter_id) {
            Ok(()) => PostTaskResult {
                task_type: "extract_assets".to_string(),
                status: "succeeded".to_string(),
                summary: "structured drafts refreshed".to_string(),
                error: None,
                meta: Some(json!({ "chapterId": chapter_id })),
            },
            Err(err) => PostTaskResult {
                task_type: "extract_assets".to_string(),
                status: "failed".to_string(),
                summary: "asset extraction failed".to_string(),
                error: Some(format!("{}: {}", err.code, err.message)),
                meta: None,
            },
        }
    }

    fn extract_state(
        &self,
        project_root: &str,
        chapter_id: Option<&str>,
        scene_type: &str,
        normalized_output: &str,
    ) -> PostTaskResult {
        let Some(chapter_id) = chapter_id.map(str::trim).filter(|value| !value.is_empty()) else {
            return PostTaskResult {
                task_type: "extract_state".to_string(),
                status: "skipped".to_string(),
                summary: "chapter_id missing".to_string(),
                error: None,
                meta: None,
            };
        };

        let emotion = infer_emotion(normalized_output);
        let relationship_temperature = infer_relationship_temperature(normalized_output);
        let injury_state = infer_injury_state(normalized_output);
        let stamina_state = infer_stamina_state(normalized_output);

        let payload = json!({
            "sceneType": scene_type,
            "emotion": emotion,
            "relationshipTemperature": relationship_temperature,
            "injuryState": injury_state,
            "staminaState": stamina_state,
        });

        let save_result = StoryStateService.upsert_state(
            project_root,
            StoryStateInput {
                subject_type: "scene".to_string(),
                subject_id: chapter_id.to_string(),
                scope: "chapter".to_string(),
                state_kind: "post_extract_state".to_string(),
                payload_json: payload.clone(),
                source_chapter_id: Some(chapter_id.to_string()),
            },
        );
        match save_result {
            Ok(_) => PostTaskResult {
                task_type: "extract_state".to_string(),
                status: "succeeded".to_string(),
                summary: "state snapshot extracted".to_string(),
                error: None,
                meta: Some(payload),
            },
            Err(err) => PostTaskResult {
                task_type: "extract_state".to_string(),
                status: "failed".to_string(),
                summary: "state extraction failed".to_string(),
                error: Some(format!("{}: {}", err.code, err.message)),
                meta: Some(payload),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PostTaskExecutor;
    use crate::services::context_service::ContextService;

    #[test]
    fn execute_merges_scene_defaults_with_route_post_tasks() {
        let executor = PostTaskExecutor;
        let context_service = ContextService;
        let results = executor.execute(
            "unused-project-root",
            None,
            "combat",
            &[
                "extract_assets".to_string(),
                "review_continuity".to_string(),
                "unknown_task".to_string(),
            ],
            "战斗中主角受伤流血",
            &context_service,
        );

        let task_types = results
            .iter()
            .map(|item| item.task_type.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            task_types,
            vec![
                "review_continuity",
                "extract_state",
                "extract_assets",
                "unknown_task",
            ]
        );
    }

    #[test]
    fn execute_records_failure_results_without_panicking() {
        let executor = PostTaskExecutor;
        let context_service = ContextService;
        let results = executor.execute(
            "   ",
            Some("chapter-1"),
            "combat",
            &[],
            "战斗中主角受伤流血",
            &context_service,
        );

        assert!(!results.is_empty());
        assert!(
            results.iter().any(|item| item.status == "failed"),
            "expected at least one failed post-task result"
        );
    }
}
