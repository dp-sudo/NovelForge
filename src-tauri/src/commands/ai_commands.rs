use std::sync::{Arc, Mutex};

use tauri::{Emitter, Listener, State};
use uuid::Uuid;

use crate::adapters::{
    llm_types::{ContentBlock, Message, StreamChunk, UnifiedGenerateRequest},
    ProviderConfig,
};
use crate::errors::AppErrorDto;
use crate::services::ai_pipeline_service::RunAiTaskPipelineInput;
use crate::services::ai_service::GeneratePreviewInput;
use crate::services::context_service::{CollectedContext, ContextService};
use crate::services::prompt_builder::PromptBuilder;
use crate::services::task_routing;
use crate::state::AppState;

// ── Legacy preview command ──

#[tauri::command]
pub async fn generate_ai_preview(
    project_root: String,
    input: GeneratePreviewInput,
    state: State<'_, AppState>,
) -> Result<crate::services::ai_service::AiPreviewResult, AppErrorDto> {
    crate::infra::logger::log_ai_call("preview", "default", &input.task_type, None);

    // Attempt real preview using context + prompt builder
    let chapter_id = match &input.chapter_id {
        Some(cid) if !cid.is_empty() => cid.clone(),
        _ => {
            crate::infra::logger::log_service(
                "ai_commands",
                "generate_ai_preview",
                "no chapter_id, using mock",
            );
            return Ok(legacy_mock_preview(&input));
        }
    };

    let context = match state
        .context_service
        .collect_chapter_context(&project_root, &chapter_id)
    {
        Ok(ctx) => ctx,
        Err(_) => {
            crate::infra::logger::log_service(
                "ai_commands",
                "generate_ai_preview",
                "context collection failed, fallback to mock",
            );
            return Ok(legacy_mock_preview(&input));
        }
    };

    let prompt = resolve_or_build_prompt(
        &state,
        &context,
        &input.task_type,
        input.selected_text.as_deref().unwrap_or(""),
        &input.user_instruction,
    )?;

    let request_id = Uuid::new_v4().to_string();
    let used_context = vec![
        "project".to_string(),
        "blueprint".to_string(),
        "chapter_links.characters".to_string(),
        "chapter_links.world_rules".to_string(),
        "chapter_links.plot_nodes".to_string(),
    ];

    // Try non-streaming generation
    let req = UnifiedGenerateRequest {
        model: "default".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock {
                block_type: "text".to_string(),
                text: Some("请根据上述要求生成内容。".to_string()),
            }],
        }],
        system_prompt: Some(prompt),
        stream: false,
        task_type: Some(input.task_type.clone()),
        ..Default::default()
    };

    match state.ai_service.generate_text(req, None).await {
        Ok(resp) => {
            let preview = resp
                .choices
                .first()
                .and_then(|c| c.message.content.first())
                .and_then(|c| c.text.clone())
                .unwrap_or_default();
            Ok(crate::services::ai_service::AiPreviewResult {
                request_id,
                preview,
                used_context,
                risks: vec![],
            })
        }
        Err(_) => Ok(legacy_mock_preview(&input)),
    }
}

fn legacy_mock_preview(
    input: &GeneratePreviewInput,
) -> crate::services::ai_service::AiPreviewResult {
    let request_id = Uuid::new_v4().to_string();
    let preview = match input.task_type.as_str() {
        "scan_consistency" => {
            r#"{"issues":[{"issueType":"prose_style","severity":"low","sourceText":"命运的齿轮开始转动","explanation":"存在典型套话表达","suggestedFix":"改为具体动作或感官描写","relatedAsset":"chapter"}]}"#.to_string()
        }
        _ => format!(
            "【AI预览草稿】\n{}\n\n他推开门，雨声骤然压进屋里。",
            input.user_instruction
        ),
    };
    crate::services::ai_service::AiPreviewResult {
        request_id,
        preview,
        used_context: vec!["project.json".to_string(), "当前章节信息".to_string()],
        risks: vec!["请检查生成内容是否符合预期".to_string()],
    }
}

// ── Chapter-aware streaming command ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterTaskInput {
    pub project_root: String,
    pub chapter_id: String,
    pub task_type: String,
    pub user_instruction: String,
    pub selected_text: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PipelineEventBridgePayload {
    pub request_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub delta: Option<String>,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[tauri::command]
pub async fn run_ai_task_pipeline(
    app_handle: tauri::AppHandle,
    input: RunAiTaskPipelineInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    let request_id = Uuid::new_v4().to_string();
    crate::infra::logger::log_user_action(
        "pipeline.start",
        &format!(
            "requestId={}, taskType={}, chapterId={}",
            request_id,
            input.task_type,
            input
                .chapter_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .unwrap_or("n/a")
        ),
    );
    spawn_pipeline_run(&app_handle, &state, request_id.clone(), input, false, None);
    Ok(request_id)
}

#[tauri::command]
pub async fn cancel_ai_task_pipeline(
    request_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    crate::infra::logger::log_user_action(
        "pipeline.cancel",
        &format!("requestId={}", request_id),
    );
    state
        .ai_pipeline_service
        .cancel_ai_task_pipeline(&request_id)
}

#[tauri::command]
pub async fn stream_ai_chapter_task(
    app_handle: tauri::AppHandle,
    input: ChapterTaskInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("streaming", "default", &input.task_type, None);
    let request_id = Uuid::new_v4().to_string();
    let pipeline_input = RunAiTaskPipelineInput {
        project_root: input.project_root,
        task_type: input.task_type,
        chapter_id: Some(input.chapter_id),
        ui_action: Some("stream_ai_chapter_task".to_string()),
        user_instruction: input.user_instruction,
        selected_text: input.selected_text,
        chapter_content: None,
        blueprint_step_key: None,
        blueprint_step_title: None,
    };
    let listener_id =
        register_legacy_stream_bridge(&app_handle, &request_id, "ai:pipeline:event".to_string());
    spawn_pipeline_run(
        &app_handle,
        &state,
        request_id.clone(),
        pipeline_input,
        true,
        Some(listener_id),
    );
    Ok(request_id)
}

fn register_legacy_stream_bridge(
    app_handle: &tauri::AppHandle,
    request_id: &str,
    pipeline_event_name: String,
) -> tauri::EventId {
    let req_id = request_id.to_string();
    let chunk_event = format!("ai:stream-chunk:{}", req_id);
    let done_event = format!("ai:stream-done:{}", req_id);
    let app = app_handle.clone();
    let slot: Arc<Mutex<Option<tauri::EventId>>> = Arc::new(Mutex::new(None));
    let slot_for_cb = Arc::clone(&slot);

    let listener_id = app_handle.listen(pipeline_event_name, move |event| {
        let payload =
            match serde_json::from_str::<PipelineEventBridgePayload>(event.payload().as_ref()) {
                Ok(payload) => payload,
                Err(_) => return,
            };
        if payload.request_id != req_id {
            return;
        }

        match payload.event_type.as_str() {
            "delta" => {
                let _ = app.emit(
                    &chunk_event,
                    StreamChunk {
                        content: payload.delta.unwrap_or_default(),
                        finish_reason: None,
                        request_id: req_id.clone(),
                        error: None,
                        reasoning: None,
                    },
                );
            }
            "error" => {
                let error_message = payload
                    .message
                    .or(payload.error_code)
                    .unwrap_or_else(|| "AI 生成异常".to_string());
                let _ = app.emit(
                    &chunk_event,
                    StreamChunk {
                        content: String::new(),
                        finish_reason: Some("error".to_string()),
                        request_id: req_id.clone(),
                        error: Some(error_message),
                        reasoning: None,
                    },
                );
                let _ = app.emit(&done_event, "DONE");
                detach_legacy_listener(&app, &slot_for_cb);
            }
            "done" => {
                let _ = app.emit(
                    &chunk_event,
                    StreamChunk {
                        content: String::new(),
                        finish_reason: Some("stop".to_string()),
                        request_id: req_id.clone(),
                        error: None,
                        reasoning: None,
                    },
                );
                let _ = app.emit(&done_event, "DONE");
                detach_legacy_listener(&app, &slot_for_cb);
            }
            _ => {}
        }
    });

    if let Ok(mut guard) = slot.lock() {
        *guard = Some(listener_id);
    }
    listener_id
}

fn detach_legacy_listener(
    app_handle: &tauri::AppHandle,
    slot: &Arc<Mutex<Option<tauri::EventId>>>,
) {
    if let Ok(mut guard) = slot.lock() {
        if let Some(id) = guard.take() {
            app_handle.unlisten(id);
        }
    }
}

fn spawn_pipeline_run(
    app_handle: &tauri::AppHandle,
    state: &State<'_, AppState>,
    request_id: String,
    input: RunAiTaskPipelineInput,
    with_legacy_bridge: bool,
    legacy_listener_id: Option<tauri::EventId>,
) {
    let app = app_handle.clone();
    let pipeline_service = state.ai_pipeline_service.clone();
    let ai_service = state.ai_service.clone();
    let skill_registry = state.skill_registry.clone();
    tokio::spawn(async move {
        let context_service = ContextService;
        let run_result = pipeline_service
            .run_ai_task_pipeline_with_request_id(
                &app,
                &ai_service,
                &context_service,
                &skill_registry,
                request_id.clone(),
                input,
            )
            .await;
        match run_result {
            Ok(result) => {
                crate::infra::logger::log_user_action(
                    "pipeline.done",
                    &format!(
                        "requestId={}, status={}, taskType={}",
                        request_id, result.status, result.task_type
                    ),
                );
            }
            Err(err) => {
                if err.code == "PIPELINE_CANCELLED" {
                    crate::infra::logger::log_user_action(
                        "pipeline.cancelled",
                        &format!("requestId={}, reason=cancelled_by_client", request_id),
                    );
                } else {
                    crate::infra::logger::log_command_error(
                        "run_ai_task_pipeline",
                        &format!(
                            "requestId={}, code={}, message={}",
                            request_id, err.code, err.message
                        ),
                    );
                }
            }
        }
        if with_legacy_bridge {
            if let Some(listener_id) = legacy_listener_id {
                app.unlisten(listener_id);
            }
        }
    });
}

// ── Legacy streaming command ──

#[tauri::command]
pub async fn stream_ai_generate(
    app_handle: tauri::AppHandle,
    req: UnifiedGenerateRequest,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    let request_id = Uuid::new_v4().to_string();
    let event_prefix = format!("ai:stream-chunk:{}", request_id);
    let done_event = format!("ai:stream-done:{}", request_id);

    let mut rx = state.ai_service.stream_generate(req, None).await?;

    tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            let _ = app_handle.emit(&event_prefix, &chunk);
        }
        let _ = app_handle.emit(&done_event, "DONE");
    });

    Ok(request_id)
}

// ── Register provider command ──

#[tauri::command]
pub async fn register_ai_provider(
    config: ProviderConfig,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.ai_service.register_provider(config).await;
    Ok(())
}

// ── Test connection command ──

#[tauri::command]
pub async fn test_ai_connection(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.ai_service.test_connection(&provider_id).await
}

// ── Blueprint AI suggestion (non-streaming) ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintSuggestionInput {
    pub project_root: String,
    pub step_key: String,
    pub step_title: String,
    pub user_instruction: String,
}

#[tauri::command]
pub async fn generate_blueprint_suggestion(
    input: BlueprintSuggestionInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call(
        "blueprint",
        "default",
        &format!("blueprint.{}", input.step_key),
        None,
    );

    // Collect global context
    let context = state
        .context_service
        .collect_global_context_only(&input.project_root)?;

    // Build prompt
    let prompt = PromptBuilder::build_blueprint_step(
        &context,
        &input.step_key,
        &input.step_title,
        &input.user_instruction,
    );

    // Log the AI request
    let _ = state.ai_service.log_ai_request(
        &input.project_root,
        "blueprint.generate_step",
        None,
        None,
        &prompt,
        "running",
    );

    // Generate
    let req = UnifiedGenerateRequest {
        model: "default".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock {
                block_type: "text".to_string(),
                text: Some("请根据上述要求生成内容。".to_string()),
            }],
        }],
        system_prompt: Some(prompt),
        stream: false,
        task_type: Some("blueprint.generate_step".to_string()),
        ..Default::default()
    };

    match state.ai_service.generate_text(req, None).await {
        Ok(resp) => {
            let text = resp
                .choices
                .first()
                .and_then(|c| c.message.content.first())
                .and_then(|c| c.text.clone())
                .unwrap_or_default();
            Ok(text)
        }
        Err(e) => Err(e),
    }
}

// ── AI character creation (non-streaming) ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCharacterInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_character(
    app_handle: tauri::AppHandle,
    input: AiCharacterInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("character", "default", "character.create", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "character.create".to_string(),
            chapter_id: None,
            ui_action: Some("ai_generate_character".to_string()),
            user_instruction: input.user_description,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
        },
    )
    .await
}

// ── AI world rule creation (non-streaming) ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWorldRuleInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_world_rule(
    app_handle: tauri::AppHandle,
    input: AiWorldRuleInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("world", "default", "world.create_rule", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "world.create_rule".to_string(),
            chapter_id: None,
            ui_action: Some("ai_generate_world_rule".to_string()),
            user_instruction: input.user_description,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
        },
    )
    .await
}

// ── AI plot node creation (non-streaming) ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPlotNodeInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_plot_node(
    app_handle: tauri::AppHandle,
    input: AiPlotNodeInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("plot", "default", "plot.create_node", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "plot.create_node".to_string(),
            chapter_id: None,
            ui_action: Some("ai_generate_plot_node".to_string()),
            user_instruction: input.user_description,
            selected_text: None,
            chapter_content: None,
            blueprint_step_key: None,
            blueprint_step_title: None,
        },
    )
    .await
}

// ── AI consistency scan ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConsistencyInput {
    pub project_root: String,
    pub chapter_id: String,
    pub chapter_content: String,
}

#[tauri::command]
pub async fn ai_scan_consistency(
    app_handle: tauri::AppHandle,
    input: AiConsistencyInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("consistency", "default", "consistency.scan", None);
    run_pipeline_text_result(
        &app_handle,
        &state,
        RunAiTaskPipelineInput {
            project_root: input.project_root,
            task_type: "consistency.scan".to_string(),
            chapter_id: Some(input.chapter_id),
            ui_action: Some("ai_scan_consistency".to_string()),
            user_instruction: String::new(),
            selected_text: None,
            chapter_content: Some(input.chapter_content),
            blueprint_step_key: None,
            blueprint_step_title: None,
        },
    )
    .await
}

async fn run_pipeline_text_result(
    app_handle: &tauri::AppHandle,
    state: &State<'_, AppState>,
    input: RunAiTaskPipelineInput,
) -> Result<String, AppErrorDto> {
    let context_service = ContextService;
    let result = state
        .ai_pipeline_service
        .run_ai_task_pipeline(
            app_handle,
            &state.ai_service,
            &context_service,
            &state.skill_registry,
            input,
        )
        .await?;
    Ok(result.output_text.unwrap_or_default())
}

/// Resolve a prompt from Skill Markdown, falling back to PromptBuilder.
fn resolve_or_build_prompt(
    state: &State<'_, AppState>,
    context: &CollectedContext,
    task_type: &str,
    selected_text: &str,
    user_instruction: &str,
) -> Result<String, AppErrorDto> {
    let skill_id = task_routing::canonical_task_type(task_type).into_owned();

    // Try loading from Skill Markdown
    if let Ok(guard) = state.skill_registry.read() {
        if let Ok(Some(content)) = guard.read_skill_content(&skill_id) {
            let context_str = context_to_string(context, selected_text, user_instruction);
            let rendered = content
                .replace("{projectContext}", &context_str)
                .replace("{userInstruction}", user_instruction)
                .replace("{selectedText}", selected_text);
            return Ok(rendered);
        }
    }

    // Fallback to PromptBuilder
    let prompt = match skill_id.as_str() {
        "chapter.continue" => {
            PromptBuilder::build_continue(context, selected_text, user_instruction)
        }
        "chapter.rewrite" => PromptBuilder::build_rewrite(context, selected_text, user_instruction),
        "prose.naturalize" => PromptBuilder::build_naturalize(context, selected_text),
        "chapter.plan" => PromptBuilder::build_chapter_plan(context, user_instruction),
        _ => PromptBuilder::build_chapter_draft(context, user_instruction),
    };
    Ok(prompt)
}

/// Render collected context into a string for Skill Markdown placeholders.
fn context_to_string(
    context: &CollectedContext,
    _selected_text: &str,
    _user_instruction: &str,
) -> String {
    let global = &context.global_context;
    let related = &context.related_context;
    let mut parts = vec![];

    parts.push(format!("作品名称：{}", global.project_name));
    parts.push(format!("题材：{}", global.genre));
    if let Some(ref pov) = global.narrative_pov {
        parts.push(format!("叙事视角：{}", pov));
    }

    for step in &global.blueprint_summary {
        if step.status == "completed" {
            if let Some(ref content) = step.content {
                let preview: String = content.chars().take(200).collect();
                parts.push(format!("[蓝图] {}: {}", step.title, preview));
            }
        }
    }

    if let Some(ref ch) = related.chapter {
        parts.push(format!("章节：{}", ch.title));
        if !ch.summary.is_empty() {
            parts.push(format!("摘要：{}", ch.summary));
        }
    }

    for node in &related.plot_nodes {
        parts.push(format!("剧情节点：{}", node.title));
    }
    for ch in &related.characters {
        parts.push(format!("角色：{}", ch.name));
    }
    for rule in &related.world_rules {
        let preview: String = rule.description.chars().take(120).collect();
        parts.push(format!("世界规则：{}", preview));
    }

    parts.join("\n")
}
