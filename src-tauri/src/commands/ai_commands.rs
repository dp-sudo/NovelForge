use tauri::{Emitter, State};
use uuid::Uuid;

use crate::adapters::{
    llm_types::{ContentBlock, Message, StreamChunk, UnifiedGenerateRequest},
    ProviderConfig,
};
use crate::errors::AppErrorDto;
use crate::services::ai_service::{AiService, GeneratePreviewInput};
use crate::services::context_service::ContextService;
use crate::services::prompt_builder::PromptBuilder;
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
            crate::infra::logger::log_service("ai_commands", "generate_ai_preview", "no chapter_id, using mock");
            return Ok(legacy_mock_preview(&input));
        }
    };

    let context = match state
        .context_service
        .collect_chapter_context(&project_root, &chapter_id)
    {
        Ok(ctx) => ctx,
        Err(_) => {
            crate::infra::logger::log_service("ai_commands", "generate_ai_preview", "context collection failed, fallback to mock");
            return Ok(legacy_mock_preview(&input));
        }
    };

    let prompt = match input.task_type.as_str() {
        "chapter_continue" | "continue_chapter" => PromptBuilder::build_continue(
            &context,
            input.selected_text.as_deref().unwrap_or(""),
            &input.user_instruction,
        ),
        "chapter_rewrite" | "rewrite_selection" => PromptBuilder::build_rewrite(
            &context,
            input.selected_text.as_deref().unwrap_or(""),
            &input.user_instruction,
        ),
        _ => PromptBuilder::build_chapter_draft(&context, &input.user_instruction),
    };

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

    match state.ai_service.generate_text(req).await {
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

#[tauri::command]
pub async fn stream_ai_chapter_task(
    app_handle: tauri::AppHandle,
    input: ChapterTaskInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("streaming", "default", &input.task_type, None);

    // 1. Collect context
    let context = state
        .context_service
        .collect_chapter_context(&input.project_root, &input.chapter_id)?;

    // 2. Build prompt based on task type
    let prompt = match input.task_type.as_str() {
        "chapter_continue" | "continue_chapter" => PromptBuilder::build_continue(
            &context,
            input.selected_text.as_deref().unwrap_or(""),
            &input.user_instruction,
        ),
        "chapter_rewrite" | "rewrite_selection" => PromptBuilder::build_rewrite(
            &context,
            input.selected_text.as_deref().unwrap_or(""),
            &input.user_instruction,
        ),
        "prose_naturalize" | "deai_text" => {
            PromptBuilder::build_naturalize(&context, input.selected_text.as_deref().unwrap_or(""))
        }
        "chapter_plan" | "plan_chapter" => {
            PromptBuilder::build_chapter_plan(&context, &input.user_instruction)
        }
        _ => PromptBuilder::build_chapter_draft(&context, &input.user_instruction),
    };

    // 3. Build unified request with task routing
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
        stream: true,
        task_type: Some(input.task_type.clone()),
        ..Default::default()
    };

    // 4. Start streaming
    let request_id = Uuid::new_v4().to_string();
    let mut rx = state.ai_service.stream_generate(req).await?;

    // 5. Log AI request
    let preview = input.user_instruction.chars().take(120).collect::<String>();
    let _ = state.ai_service.log_ai_request(
        &input.project_root,
        &input.task_type,
        None,
        None,
        &preview,
        "running",
    );

    // 6. Spawn event emitter
    let app = app_handle.clone();
    let event_prefix = format!("ai:stream-chunk:{}", request_id);
    let done_event = format!("ai:stream-done:{}", request_id);
    let proj_root = input.project_root.clone();
    let req_id = request_id.clone();

    tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            let _ = app.emit(&event_prefix, &chunk);
        }
        let _ = app.emit(&done_event, "DONE");
    });

    Ok(request_id)
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

    let mut rx = state.ai_service.stream_generate(req).await?;

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

// ── List built-in skills ──

#[tauri::command]
pub async fn list_skills(
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::skill_registry::SkillManifest>, AppErrorDto> {
    Ok(state.skill_registry.list_skills().to_vec())
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
    crate::infra::logger::log_ai_call("blueprint", "default", &format!("blueprint.{}", input.step_key), None);

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

    match state.ai_service.generate_text(req).await {
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
    input: AiCharacterInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("character", "default", "character.create", None);

    let context = state
        .context_service
        .collect_global_context_only(&input.project_root)?;

    let prompt = PromptBuilder::build_character_create(&context, &input.user_description);

    let _ = state.ai_service.log_ai_request(
        &input.project_root,
        "character.create",
        None,
        None,
        &prompt,
        "running",
    );

    let req = UnifiedGenerateRequest {
        model: "default".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock {
                block_type: "text".to_string(),
                text: Some("请根据用户设想生成角色卡 JSON。".to_string()),
            }],
        }],
        system_prompt: Some(prompt),
        stream: false,
        task_type: Some("character.create".to_string()),
        ..Default::default()
    };

    match state.ai_service.generate_text(req).await {
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

// ── AI world rule creation (non-streaming) ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWorldRuleInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_world_rule(
    input: AiWorldRuleInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("world", "default", "world.create_rule", None);

    let context = state
        .context_service
        .collect_global_context_only(&input.project_root)?;

    let prompt = PromptBuilder::build_world_create_rule(&context, &input.user_description);

    let _ = state.ai_service.log_ai_request(
        &input.project_root,
        "world.create_rule",
        None,
        None,
        &prompt,
        "running",
    );

    let req = UnifiedGenerateRequest {
        model: "default".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock {
                block_type: "text".to_string(),
                text: Some("请根据用户设想生成世界设定 JSON。".to_string()),
            }],
        }],
        system_prompt: Some(prompt),
        stream: false,
        task_type: Some("world.create_rule".to_string()),
        ..Default::default()
    };

    match state.ai_service.generate_text(req).await {
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

// ── AI plot node creation (non-streaming) ──

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPlotNodeInput {
    pub project_root: String,
    pub user_description: String,
}

#[tauri::command]
pub async fn ai_generate_plot_node(
    input: AiPlotNodeInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("plot", "default", "plot.create_node", None);

    let context = state
        .context_service
        .collect_global_context_only(&input.project_root)?;

    let prompt = PromptBuilder::build_plot_create_node(&context, &input.user_description);

    let _ = state.ai_service.log_ai_request(
        &input.project_root,
        "plot.create_node",
        None,
        None,
        &prompt,
        "running",
    );

    let req = UnifiedGenerateRequest {
        model: "default".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock {
                block_type: "text".to_string(),
                text: Some("请根据用户设想生成剧情节点 JSON。".to_string()),
            }],
        }],
        system_prompt: Some(prompt),
        stream: false,
        task_type: Some("plot.create_node".to_string()),
        ..Default::default()
    };

    match state.ai_service.generate_text(req).await {
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
    input: AiConsistencyInput,
    state: State<'_, AppState>,
) -> Result<String, AppErrorDto> {
    crate::infra::logger::log_ai_call("consistency", "default", "consistency.scan", None);

    let context = state
        .context_service
        .collect_chapter_context(&input.project_root, &input.chapter_id)?;

    let prompt = PromptBuilder::build_consistency_scan(&context, &input.chapter_content);

    let _ = state.ai_service.log_ai_request(
        &input.project_root,
        "consistency.scan",
        None,
        None,
        &prompt,
        "running",
    );

    let req = UnifiedGenerateRequest {
        model: "default".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock {
                block_type: "text".to_string(),
                text: Some("请检查章节一致性并输出 JSON。".to_string()),
            }],
        }],
        system_prompt: Some(prompt),
        stream: false,
        task_type: Some("consistency.scan".to_string()),
        ..Default::default()
    };

    match state.ai_service.generate_text(req).await {
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
