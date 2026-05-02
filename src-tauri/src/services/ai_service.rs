use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock as StdRwLock};

use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::adapters::{
    anthropic::AnthropicAdapter, gemini::GeminiAdapter, llm_service::LlmService, llm_types::*,
    openai_compatible::OpenAiCompatibleAdapter,
};
use crate::domain::routing_strategy::{ProjectStage, RiskLevel, RoutingStrategyTemplate};
use crate::errors::AppErrorDto;
use crate::infra::{app_database, credential_manager};
use crate::infra::database::open_database;
use crate::infra::time::now_iso;
use crate::services::project_service::get_project_id;
use crate::services::skill_registry::{RouteOverride, SkillRegistry};
use crate::services::task_routing;

const PIPELINE_STREAM_ERROR_PREFIX: &str = "__NF_PIPELINE_ERROR__:";

#[derive(Clone)]
pub struct AiService {
    adapters: Arc<RwLock<HashMap<String, Box<dyn LlmService>>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRouteAttempt {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRouteResolution {
    pub canonical_task_type: String,
    pub provider_id: String,
    pub model_id: String,
    pub model_pool_id: Option<String>,
    pub fallback_model_pool_id: Option<String>,
    pub post_tasks: Vec<String>,
    pub attempts: Vec<TaskRouteAttempt>,
}

impl Default for AiService {
    fn default() -> Self {
        Self {
            adapters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AiService {
    fn provider_not_found_error(provider_id: &str) -> AppErrorDto {
        AppErrorDto::new(
            "PROVIDER_NOT_FOUND",
            &format!("未找到供应商 '{}'", provider_id),
            true,
        )
    }

    fn load_provider_from_db(
        conn: &rusqlite::Connection,
        provider_id: &str,
    ) -> Result<ProviderConfig, AppErrorDto> {
        app_database::load_provider(conn, provider_id)?
            .ok_or_else(|| Self::provider_not_found_error(provider_id))
    }

    fn load_provider_with_api_key(provider_id: &str) -> Result<ProviderConfig, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        let mut config = Self::load_provider_from_db(&conn, provider_id)?;
        if let Ok(Some(key)) = credential_manager::load_api_key(provider_id) {
            config.api_key = Some(key);
        }
        Ok(config)
    }

    fn build_route_resolution(
        canonical_task_type: String,
        provider_id: String,
        model_id: String,
        route: Option<TaskRoute>,
    ) -> TaskRouteResolution {
        let attempts = Self::build_attempt_chain(&provider_id, &model_id, route.as_ref())
            .into_iter()
            .map(|(provider_id, model_id)| TaskRouteAttempt {
                provider_id,
                model_id,
            })
            .collect::<Vec<_>>();
        let model_pool_id = route
            .as_ref()
            .and_then(|item| item.model_pool_id.clone())
            .filter(|value| !value.trim().is_empty());
        let fallback_model_pool_id = route
            .as_ref()
            .and_then(|item| item.fallback_model_pool_id.clone())
            .filter(|value| !value.trim().is_empty());
        let post_tasks = route
            .as_ref()
            .map(|item| item.post_tasks.clone())
            .unwrap_or_default();
        TaskRouteResolution {
            canonical_task_type,
            provider_id,
            model_id,
            model_pool_id,
            fallback_model_pool_id,
            post_tasks,
            attempts,
        }
    }

    fn is_usable_route(route: &TaskRoute) -> bool {
        let direct = !route.provider_id.trim().is_empty() && !route.model_id.trim().is_empty();
        let pooled = route
            .model_pool_id
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        direct || pooled
    }

    fn pick_task_route<'a>(routes: &'a [TaskRoute], task_type: &str) -> Option<&'a TaskRoute> {
        let canonical_task = task_routing::canonical_task_type(task_type);
        if let Some(route) = routes
            .iter()
            .find(|r| r.task_type == canonical_task.as_ref() && Self::is_usable_route(r))
        {
            return Some(route);
        }

        routes.iter().find(|r| {
            Self::is_usable_route(r)
                && task_routing::canonical_task_type(&r.task_type).as_ref()
                    == canonical_task.as_ref()
        })
    }

    fn default_model_pool_id_for_task(canonical_task: &str) -> Option<&'static str> {
        match canonical_task {
            "chapter.plan" | "blueprint.generate_step" => Some("planner"),
            "chapter.draft" | "chapter.continue" | "chapter.rewrite" | "prose.naturalize" => {
                Some("drafter")
            }
            "consistency.scan"
            | "timeline.review"
            | "relationship.review"
            | "dashboard.review"
            | "export.review" => Some("reviewer"),
            "character.create"
            | "world.create_rule"
            | "plot.create_node"
            | "glossary.create_term" => Some("extractor"),
            "narrative.create_obligation" => Some("state"),
            _ => None,
        }
    }

    fn pool_entries(pool: &crate::adapters::llm_types::ModelPoolRecord) -> Vec<(String, String)> {
        pool.entries
            .iter()
            .filter_map(|entry| {
                let provider = entry.provider_id.trim();
                let model = entry.model_id.trim();
                if provider.is_empty() || model.is_empty() {
                    None
                } else {
                    Some((provider.to_string(), model.to_string()))
                }
            })
            .collect()
    }

    fn resolve_with_model_pool_if_configured(
        conn: &rusqlite::Connection,
        canonical_task: &str,
        route: &TaskRoute,
    ) -> Result<(String, String, TaskRoute), AppErrorDto> {
        let pools = app_database::load_model_pools(conn)?;
        let explicit_pool_id = route
            .model_pool_id
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let fallback_pool_id = route
            .fallback_model_pool_id
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let pool_id = explicit_pool_id
            .or_else(|| {
                Self::default_model_pool_id_for_task(canonical_task).map(|value| value.to_string())
            })
            .filter(|value| !value.is_empty());

        let Some(pool_id) = pool_id else {
            return Ok((
                route.provider_id.clone(),
                route.model_id.clone(),
                route.clone(),
            ));
        };

        let Some(primary_pool) = pools.iter().find(|pool| pool.id == pool_id && pool.enabled)
        else {
            return Ok((
                route.provider_id.clone(),
                route.model_id.clone(),
                route.clone(),
            ));
        };
        let primary_entries = Self::pool_entries(primary_pool);
        let Some((provider_id, model_id)) = primary_entries.first().cloned() else {
            return Ok((
                route.provider_id.clone(),
                route.model_id.clone(),
                route.clone(),
            ));
        };

        let mut resolved_route = route.clone();
        resolved_route.provider_id = provider_id.clone();
        resolved_route.model_id = model_id.clone();
        resolved_route.model_pool_id = Some(primary_pool.id.clone());

        let mut fallback_target = primary_entries.get(1).cloned();
        let fallback_pool_id = fallback_pool_id.or_else(|| {
            primary_pool
                .fallback_pool_id
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });
        if fallback_target.is_none() {
            if let Some(fallback_pool_id) = fallback_pool_id.as_ref() {
                if let Some(pool) = pools
                    .iter()
                    .find(|pool| pool.id == *fallback_pool_id && pool.enabled)
                {
                    fallback_target = Self::pool_entries(pool).into_iter().next();
                    resolved_route.fallback_model_pool_id = Some(pool.id.clone());
                }
            }
        }
        if let Some((fallback_provider, fallback_model)) = fallback_target {
            resolved_route.fallback_provider_id = Some(fallback_provider);
            resolved_route.fallback_model_id = Some(fallback_model);
        }

        Ok((provider_id, model_id, resolved_route))
    }

    fn build_attempt_chain(
        provider_id: &str,
        model: &str,
        route: Option<&TaskRoute>,
    ) -> Vec<(String, String)> {
        let max_attempts = route
            .map(|r| r.max_retries.clamp(1, 8) as usize)
            .unwrap_or(1);
        let fallback = route.and_then(|r| {
            r.fallback_provider_id.as_ref().map(|fp| {
                let fallback_model = r
                    .fallback_model_id
                    .as_ref()
                    .map(|m| m.trim().to_string())
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| "default".to_string());
                (fp.clone(), fallback_model)
            })
        });

        let mut attempts = Vec::with_capacity(max_attempts);
        for idx in 0..max_attempts {
            if idx == 0 {
                attempts.push((provider_id.to_string(), model.to_string()));
            } else if let Some((ref fp, ref fm)) = fallback {
                attempts.push((fp.clone(), fm.clone()));
            } else {
                attempts.push((provider_id.to_string(), model.to_string()));
            }
        }
        attempts
    }

    fn pool_mapping(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(task_type, pool_id)| ((*task_type).to_string(), (*pool_id).to_string()))
            .collect()
    }

    fn built_in_routing_strategy_templates() -> Vec<RoutingStrategyTemplate> {
        vec![
            RoutingStrategyTemplate {
                id: "stage-draft-default".to_string(),
                name: "草稿阶段：快速起稿".to_string(),
                description: "草稿期优先速度，核心任务默认路由到 Drafter 池。".to_string(),
                project_stage: ProjectStage::Draft,
                task_risk_level: RiskLevel::Medium,
                recommended_pools: Self::pool_mapping(&[
                    ("chapter.draft", "drafter"),
                    ("chapter.continue", "drafter"),
                    ("chapter.rewrite", "drafter"),
                    ("chapter.plan", "drafter"),
                    ("prose.naturalize", "drafter"),
                    ("character.create", "drafter"),
                    ("world.create_rule", "drafter"),
                    ("plot.create_node", "drafter"),
                    ("glossary.create_term", "drafter"),
                    ("narrative.create_obligation", "drafter"),
                    ("consistency.scan", "reviewer"),
                    ("timeline.review", "reviewer"),
                    ("relationship.review", "reviewer"),
                    ("dashboard.review", "reviewer"),
                    ("export.review", "reviewer"),
                    ("blueprint.generate_step", "planner"),
                ]),
            },
            RoutingStrategyTemplate {
                id: "stage-revision-balanced".to_string(),
                name: "修订阶段：生成+审查均衡".to_string(),
                description:
                    "章节任务走 Drafter，审查任务走 Reviewer，蓝图与计划任务走 Planner。"
                        .to_string(),
                project_stage: ProjectStage::Revision,
                task_risk_level: RiskLevel::Medium,
                recommended_pools: Self::pool_mapping(&[
                    ("chapter.draft", "drafter"),
                    ("chapter.continue", "drafter"),
                    ("chapter.rewrite", "drafter"),
                    ("prose.naturalize", "reviewer"),
                    ("chapter.plan", "planner"),
                    ("blueprint.generate_step", "planner"),
                    ("character.create", "extractor"),
                    ("world.create_rule", "extractor"),
                    ("plot.create_node", "extractor"),
                    ("glossary.create_term", "extractor"),
                    ("narrative.create_obligation", "state"),
                    ("consistency.scan", "reviewer"),
                    ("timeline.review", "reviewer"),
                    ("relationship.review", "reviewer"),
                    ("dashboard.review", "reviewer"),
                    ("export.review", "reviewer"),
                ]),
            },
            RoutingStrategyTemplate {
                id: "stage-polish-high-quality".to_string(),
                name: "打磨阶段：高质量优先".to_string(),
                description: "打磨期优先质量，规划类任务走 Planner，文本与审查任务走 Reviewer。"
                    .to_string(),
                project_stage: ProjectStage::Polish,
                task_risk_level: RiskLevel::High,
                recommended_pools: Self::pool_mapping(&[
                    ("chapter.draft", "reviewer"),
                    ("chapter.continue", "reviewer"),
                    ("chapter.rewrite", "reviewer"),
                    ("chapter.plan", "planner"),
                    ("prose.naturalize", "reviewer"),
                    ("character.create", "reviewer"),
                    ("world.create_rule", "reviewer"),
                    ("plot.create_node", "planner"),
                    ("glossary.create_term", "reviewer"),
                    ("narrative.create_obligation", "planner"),
                    ("blueprint.generate_step", "planner"),
                    ("consistency.scan", "reviewer"),
                    ("timeline.review", "reviewer"),
                    ("relationship.review", "reviewer"),
                    ("dashboard.review", "reviewer"),
                    ("export.review", "reviewer"),
                ]),
            },
            RoutingStrategyTemplate {
                id: "risk-high-critical-planner".to_string(),
                name: "高风险任务：关键规划加固".to_string(),
                description:
                    "关键任务（蓝图/主线/章节计划）优先 Planner，其余任务保持平衡。"
                        .to_string(),
                project_stage: ProjectStage::Revision,
                task_risk_level: RiskLevel::High,
                recommended_pools: Self::pool_mapping(&[
                    ("blueprint.generate_step", "planner"),
                    ("chapter.plan", "planner"),
                    ("plot.create_node", "planner"),
                    ("narrative.create_obligation", "planner"),
                    ("chapter.draft", "drafter"),
                    ("chapter.continue", "drafter"),
                    ("chapter.rewrite", "drafter"),
                    ("prose.naturalize", "reviewer"),
                    ("consistency.scan", "reviewer"),
                    ("timeline.review", "reviewer"),
                    ("relationship.review", "reviewer"),
                    ("dashboard.review", "reviewer"),
                    ("export.review", "reviewer"),
                ]),
            },
        ]
    }

    fn task_risk_level(task_type: &str) -> RiskLevel {
        match task_routing::canonical_task_type(task_type).as_ref() {
            "blueprint.generate_step"
            | "chapter.plan"
            | "plot.create_node"
            | "narrative.create_obligation" => RiskLevel::High,
            "chapter.draft" | "chapter.continue" | "chapter.rewrite" | "prose.naturalize" => {
                RiskLevel::Medium
            }
            _ => RiskLevel::Low,
        }
    }

    fn infer_project_stage_from_project(project_root: &str) -> Result<ProjectStage, AppErrorDto> {
        let normalized_root = project_root.trim();
        if normalized_root.is_empty() {
            return Ok(ProjectStage::Draft);
        }
        let conn = open_database(Path::new(normalized_root)).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let raw_profile: Option<String> = conn
            .query_row("SELECT ai_strategy_profile FROM projects LIMIT 1", [], |row| {
                row.get::<_, Option<String>>(0)
            })
            .ok()
            .flatten();
        let Some(raw_profile) = raw_profile else {
            return Ok(ProjectStage::Draft);
        };
        if raw_profile.trim().is_empty() {
            return Ok(ProjectStage::Draft);
        }
        let parsed = serde_json::from_str::<serde_json::Value>(&raw_profile)
            .unwrap_or_else(|_| serde_json::json!({}));
        let mode = parsed
            .get("chapterGenerationMode")
            .and_then(|value| value.as_str())
            .or_else(|| parsed.get("chapter_generation_mode").and_then(|value| value.as_str()))
            .unwrap_or("draft_only");
        let stage = match mode {
            "plan_scene_draft" => ProjectStage::Polish,
            "plan_draft" => ProjectStage::Revision,
            _ => ProjectStage::Draft,
        };
        Ok(stage)
    }

    fn find_strategy_template(strategy_id: &str) -> Option<RoutingStrategyTemplate> {
        let normalized = strategy_id.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return None;
        }
        Self::built_in_routing_strategy_templates()
            .into_iter()
            .find(|template| template.id.eq_ignore_ascii_case(normalized.as_str()))
    }

    pub fn recommend_routing_strategy(
        project_root: &str,
        project_stage: Option<&str>,
        task_type: Option<&str>,
    ) -> Result<Vec<RoutingStrategyTemplate>, AppErrorDto> {
        let stage = project_stage
            .map(ProjectStage::from_str)
            .unwrap_or(Self::infer_project_stage_from_project(project_root)?);
        let risk = task_type
            .map(Self::task_risk_level)
            .unwrap_or(RiskLevel::Medium);
        let mut scored = Self::built_in_routing_strategy_templates()
            .into_iter()
            .map(|template| {
                let stage_score = if template.project_stage == stage { 100_i64 } else { 20_i64 };
                let risk_score = if template.task_risk_level == risk {
                    50_i64
                } else if template.task_risk_level == RiskLevel::High && risk == RiskLevel::Medium {
                    35_i64
                } else {
                    10_i64
                };
                (stage_score + risk_score, template)
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(scored.into_iter().map(|(_, template)| template).collect())
    }

    pub fn get_project_routing_strategy_id(project_root: &str) -> Result<Option<String>, AppErrorDto> {
        let normalized_root = project_root.trim();
        if normalized_root.is_empty() {
            return Ok(None);
        }
        let conn = open_database(Path::new(normalized_root)).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let value = conn.query_row("SELECT routing_strategy_id FROM projects LIMIT 1", [], |row| {
            row.get::<_, Option<String>>(0)
        });
        match value {
            Ok(item) => Ok(item.filter(|value| !value.trim().is_empty())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(
                AppErrorDto::new("DB_QUERY_FAILED", "读取项目路由策略失败", true)
                    .with_detail(err.to_string()),
            ),
        }
    }

    pub fn apply_routing_strategy_template(
        project_root: &str,
        strategy_id: &str,
    ) -> Result<Vec<TaskRoute>, AppErrorDto> {
        let normalized_root = project_root.trim();
        if normalized_root.is_empty() {
            return Err(AppErrorDto::new("PROJECT_INVALID_PATH", "项目目录不能为空", true));
        }
        let template = Self::find_strategy_template(strategy_id).ok_or_else(|| {
            AppErrorDto::new("ROUTING_STRATEGY_NOT_FOUND", "路由策略模板不存在", true)
        })?;

        let app_conn = app_database::open_or_create()?;
        let pools = app_database::load_model_pools(&app_conn)?;
        let existing_routes = app_database::load_task_routes(&app_conn)?;
        let now = now_iso();

        let mut route_map: HashMap<String, TaskRoute> = HashMap::new();
        for route in existing_routes {
            let canonical = task_routing::canonical_task_type(&route.task_type).into_owned();
            route_map.entry(canonical).or_insert(route);
        }

        for (task_type, preferred_pool_id) in &template.recommended_pools {
            let canonical_task = task_routing::canonical_task_type(task_type).into_owned();
            let Some(pool) = pools.iter().find(|pool| {
                pool.enabled
                    && (pool.id.eq_ignore_ascii_case(preferred_pool_id)
                        || pool.role.eq_ignore_ascii_case(preferred_pool_id))
            }) else {
                continue;
            };
            let Some(primary_entry) = pool
                .entries
                .iter()
                .find(|entry| !entry.provider_id.trim().is_empty() && !entry.model_id.trim().is_empty())
            else {
                continue;
            };

            let existing = route_map.get(&canonical_task).cloned();
            let mut route = existing.unwrap_or(TaskRoute {
                id: Uuid::new_v4().to_string(),
                task_type: canonical_task.clone(),
                provider_id: primary_entry.provider_id.trim().to_string(),
                model_id: primary_entry.model_id.trim().to_string(),
                fallback_provider_id: None,
                fallback_model_id: None,
                model_pool_id: None,
                fallback_model_pool_id: None,
                post_tasks: Vec::new(),
                max_retries: 1,
                created_at: Some(now.clone()),
                updated_at: Some(now.clone()),
            });
            route.task_type = canonical_task.clone();
            route.provider_id = primary_entry.provider_id.trim().to_string();
            route.model_id = primary_entry.model_id.trim().to_string();
            route.model_pool_id = Some(pool.id.clone());
            route.fallback_model_pool_id = route
                .fallback_model_pool_id
                .clone()
                .or_else(|| pool.fallback_pool_id.clone());
            route.max_retries = route.max_retries.clamp(1, 8);
            app_database::upsert_task_route(&app_conn, &route, &now)?;
            route.updated_at = Some(now.clone());
            route_map.insert(canonical_task, route);
        }

        let project_conn = open_database(Path::new(normalized_root)).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&project_conn)?;
        project_conn
            .execute(
                "UPDATE projects SET routing_strategy_id = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![template.id, now, project_id],
            )
            .map_err(|err| {
                AppErrorDto::new("DB_WRITE_FAILED", "保存项目路由策略失败", true)
                    .with_detail(err.to_string())
            })?;

        let mut routes = route_map.into_values().collect::<Vec<_>>();
        routes.sort_by(|a, b| a.task_type.cmp(&b.task_type));
        Ok(routes)
    }

    #[allow(dead_code)]
    pub fn list_model_pools() -> Result<Vec<ModelPoolRecord>, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        app_database::load_model_pools(&conn)
    }

    #[allow(dead_code)]
    pub fn create_model_pool(
        name: &str,
        pool_type: &str,
        models: Vec<ModelPoolEntry>,
    ) -> Result<ModelPoolRecord, AppErrorDto> {
        let normalized_role = pool_type.trim().to_ascii_lowercase();
        if normalized_role.is_empty() {
            return Err(AppErrorDto::new("INVALID_INPUT", "模型池类型不能为空", true));
        }
        let conn = app_database::open_or_create()?;
        let existing = app_database::load_model_pools(&conn)?
            .into_iter()
            .find(|pool| pool.role.eq_ignore_ascii_case(normalized_role.as_str()));
        let id = existing
            .as_ref()
            .map(|pool| pool.id.clone())
            .unwrap_or_else(|| normalized_role.clone());
        let record = ModelPoolRecord {
            id,
            display_name: name.trim().to_string(),
            role: normalized_role.clone(),
            enabled: true,
            entries: models,
            fallback_pool_id: existing.and_then(|pool| pool.fallback_pool_id),
            created_at: None,
            updated_at: None,
        };
        let pool_id = record.id.clone();
        let saved = Self::update_model_pool(&pool_id, record)?;
        Ok(saved)
    }

    #[allow(dead_code)]
    pub fn update_model_pool(
        pool_id: &str,
        config: ModelPoolRecord,
    ) -> Result<ModelPoolRecord, AppErrorDto> {
        let id = pool_id.trim().to_string();
        if id.is_empty() {
            return Err(AppErrorDto::new("INVALID_INPUT", "模型池ID不能为空", true));
        }
        let conn = app_database::open_or_create()?;
        let now = crate::infra::time::now_iso();
        let mut record = config;
        record.id = id.clone();
        record.role = record.role.trim().to_ascii_lowercase();
        record.display_name = record.display_name.trim().to_string();
        if record.role.is_empty() {
            return Err(AppErrorDto::new("INVALID_INPUT", "模型池类型不能为空", true));
        }
        if record.display_name.is_empty() {
            return Err(AppErrorDto::new("INVALID_INPUT", "模型池名称不能为空", true));
        }
        record.fallback_pool_id = record
            .fallback_pool_id
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty() && value != &id);
        app_database::upsert_model_pool(&conn, &record, &now)?;
        let saved = app_database::load_model_pools(&conn)?
            .into_iter()
            .find(|pool| pool.id == id)
            .ok_or_else(|| {
                AppErrorDto::new("MODEL_POOL_NOT_FOUND", "模型池保存后读取失败", false)
            })?;
        Ok(saved)
    }

    #[allow(dead_code)]
    pub fn delete_model_pool(pool_id: &str) -> Result<(), AppErrorDto> {
        let conn = app_database::open_or_create()?;
        app_database::delete_model_pool(&conn, pool_id)
    }

    /// Register a provider adapter at runtime.
    pub async fn register_provider(&self, config: ProviderConfig) {
        let id = config.id.clone();
        let is_anthropic_protocol = matches!(
            config.protocol.as_str(),
            "anthropic_messages" | "custom_anthropic_compatible"
        );
        let is_gemini_protocol = matches!(config.protocol.as_str(), "gemini_generate_content");

        let adapter: Box<dyn LlmService> = match config.vendor.as_str() {
            "anthropic" | "minimax" => Box::new(AnthropicAdapter::new(config)),
            "gemini" => Box::new(GeminiAdapter::new(config)),
            _ if is_anthropic_protocol => Box::new(AnthropicAdapter::new(config)),
            _ if is_gemini_protocol => Box::new(GeminiAdapter::new(config)),
            _ => Box::new(OpenAiCompatibleAdapter::new(config)),
        };
        self.adapters.write().await.insert(id, adapter);
    }

    /// Reload one provider adapter from app DB + credential store.
    pub async fn reload_provider(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        let config = Self::load_provider_with_api_key(provider_id)?;
        self.register_provider(config).await;
        Ok(())
    }

    /// Remove one provider adapter from runtime cache.
    pub async fn unregister_provider(&self, provider_id: &str) {
        self.adapters.write().await.remove(provider_id);
    }

    async fn ensure_provider_registered(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        if self.adapters.read().await.contains_key(provider_id) {
            return Ok(());
        }
        self.reload_provider(provider_id).await
    }

    fn resolve_request_model(
        provider_id: &str,
        requested_model: &str,
    ) -> Result<String, AppErrorDto> {
        if !requested_model.trim().is_empty() && requested_model != "default" {
            return Ok(requested_model.to_string());
        }

        let conn = app_database::open_or_create()?;
        let provider = Self::load_provider_from_db(&conn, provider_id)?;

        provider.default_model.ok_or_else(|| {
            AppErrorDto::new(
                "MODEL_NOT_CONFIGURED",
                &format!("供应商 '{}' 未配置默认模型", provider_id),
                true,
            )
        })
    }

    /// Resolve task_type to a provider_id + model_id from llm_task_routes.
    fn resolve_route(task_type: &str) -> Result<(String, String, Option<TaskRoute>), AppErrorDto> {
        let conn = app_database::open_or_create()?;
        let routes = app_database::load_task_routes(&conn)?;
        let canonical_task = task_routing::canonical_task_type(task_type).into_owned();

        if let Some(r) = Self::pick_task_route(&routes, task_type) {
            let (provider_id, model_id, resolved_route) =
                Self::resolve_with_model_pool_if_configured(&conn, &canonical_task, r)?;
            return Ok((provider_id, model_id, Some(resolved_route)));
        }

        // 未知技能/任务类型可显式回退到 "custom" 路由。
        if !task_routing::is_core_task_type(&canonical_task) {
            if let Some(custom_route) = Self::pick_task_route(&routes, "custom") {
                let mut inferred = custom_route.clone();
                inferred.task_type = canonical_task.clone();
                let (provider_id, model_id, resolved_route) =
                    Self::resolve_with_model_pool_if_configured(&conn, &canonical_task, &inferred)?;
                return Ok((provider_id, model_id, Some(resolved_route)));
            }
        }

        // 兼容旧行为：`custom` 仍可回退到第一条可用路由。
        if canonical_task == "custom" {
            if let Some(first) = routes.iter().find(|r| Self::is_usable_route(r)) {
                let (provider_id, model_id, resolved_route) =
                    Self::resolve_with_model_pool_if_configured(&conn, &canonical_task, first)?;
                return Ok((provider_id, model_id, Some(resolved_route)));
            }
        }

        Err(AppErrorDto::new(
            "TASK_ROUTE_NOT_FOUND",
            &format!(
                "任务类型 '{}' 尚未配置路由。请在「设置 > 任务路由」配置该任务，或配置 'custom' 作为兜底路由。",
                canonical_task
            ),
            true,
        ))
    }

    fn resolve_request_target(
        req: &UnifiedGenerateRequest,
    ) -> Result<(String, String, Option<TaskRoute>), AppErrorDto> {
        let provider_hint = req.provider_id.as_deref().map(str::trim).unwrap_or("");
        let should_route_by_task = provider_hint.is_empty() || provider_hint == "default";

        if should_route_by_task {
            if let Some(ref task_type) = req.task_type {
                return Self::resolve_route(task_type);
            }
            return Err(AppErrorDto::new(
                "LLM_NO_PROVIDER",
                "未指定供应商，且缺少用于路由解析的任务类型",
                true,
            ));
        }

        let provider_id = provider_hint.to_string();
        let model = Self::resolve_request_model(&provider_id, &req.model)?;
        Ok((provider_id, model, None))
    }

    /// Inspect task route resolution and computed retry/fallback chain for diagnostics.
    #[allow(dead_code)]
    pub fn inspect_task_route(task_type: &str) -> Result<TaskRouteResolution, AppErrorDto> {
        let canonical_task_type = task_routing::canonical_task_type(task_type).into_owned();
        let (provider_id, model_id, route) = Self::resolve_route(&canonical_task_type)?;
        Ok(Self::build_route_resolution(
            canonical_task_type,
            provider_id,
            model_id,
            route,
        ))
    }

    #[allow(dead_code)]
    pub fn inspect_task_route_with_skill_registry(
        task_type: &str,
        skill_registry: &SkillRegistry,
    ) -> Result<TaskRouteResolution, AppErrorDto> {
        let canonical_task_type = task_routing::canonical_task_type(task_type).into_owned();
        let selected = skill_registry.select_skills_for_task(&canonical_task_type)?;
        let (provider_id, model_id, route) = Self::resolve_request_target_with_route_override(
            &canonical_task_type,
            selected.route_override.as_ref(),
        )?;
        Ok(Self::build_route_resolution(
            canonical_task_type,
            provider_id,
            model_id,
            route,
        ))
    }

    pub fn inspect_task_route_with_override(
        task_type: &str,
        route_override: Option<&RouteOverride>,
    ) -> Result<TaskRouteResolution, AppErrorDto> {
        let canonical_task_type = task_routing::canonical_task_type(task_type).into_owned();
        let (provider_id, model_id, route) =
            Self::resolve_request_target_with_route_override(&canonical_task_type, route_override)?;
        Ok(Self::build_route_resolution(
            canonical_task_type,
            provider_id,
            model_id,
            route,
        ))
    }

    fn resolve_request_target_with_route_override(
        task_type: &str,
        route_override: Option<&RouteOverride>,
    ) -> Result<(String, String, Option<TaskRoute>), AppErrorDto> {
        if let Some(route_override) = route_override {
            let provider = route_override.provider.trim().to_string();
            let model = route_override.model.trim().to_string();
            if !provider.is_empty() {
                let resolved_model = if model.is_empty() {
                    "default".to_string()
                } else {
                    model.clone()
                };
                let route = Self::build_override_route(task_type, &provider, &resolved_model);
                return Ok((provider, resolved_model, Some(route)));
            }
            if !model.is_empty() {
                let req = UnifiedGenerateRequest {
                    model: "default".to_string(),
                    task_type: Some(task_type.to_string()),
                    ..Default::default()
                };
                let (default_provider, _default_model, _default_route) =
                    Self::resolve_request_target(&req)?;
                let route = Self::build_override_route(task_type, &default_provider, &model);
                return Ok((default_provider, model, Some(route)));
            }
        }

        let req = UnifiedGenerateRequest {
            model: "default".to_string(),
            task_type: Some(task_type.to_string()),
            ..Default::default()
        };
        Self::resolve_request_target(&req)
    }

    /// Resolve target with skill taskRoute override support.
    /// If the req.task_type matches a skill with a taskRoute override,
    /// the override's provider_id / model_id take precedence.
    fn resolve_with_skill_override(
        req: &UnifiedGenerateRequest,
        skill_registry: &SkillRegistry,
    ) -> Result<(String, String, Option<TaskRoute>), AppErrorDto> {
        if let Some(task_type) = req.task_type.as_deref() {
            let selected = skill_registry.select_skills_for_task(task_type)?;
            if let Some(route_override) = selected.route_override.as_ref() {
                let (provider, model, route) = Self::resolve_request_target_with_route_override(
                    task_type,
                    Some(route_override),
                )?;
                log::info!(
                    "[SKILL_ROUTE_OVERRIDE] task={} provider={} model={} reason={}",
                    task_type,
                    provider,
                    model,
                    route_override.reason
                );
                return Ok((provider, model, route));
            }
        }
        Self::resolve_request_target(req)
    }

    fn build_override_route(task_type: &str, provider: &str, model: &str) -> TaskRoute {
        TaskRoute {
            id: String::new(),
            task_type: task_routing::canonical_task_type(task_type).into_owned(),
            provider_id: provider.to_string(),
            model_id: model.to_string(),
            fallback_provider_id: None,
            fallback_model_id: None,
            model_pool_id: None,
            fallback_model_pool_id: None,
            post_tasks: Vec::new(),
            max_retries: 1,
            created_at: None,
            updated_at: None,
        }
    }

    fn encode_pipeline_stream_error(error: &AppErrorDto) -> String {
        format!(
            "{}{}::{}",
            PIPELINE_STREAM_ERROR_PREFIX, error.code, error.message
        )
    }

    pub(crate) fn decode_pipeline_stream_error(raw: &str) -> Option<(String, String)> {
        let encoded = raw.strip_prefix(PIPELINE_STREAM_ERROR_PREFIX)?;
        let (code, message) = encoded.split_once("::")?;
        let code = code.trim();
        if code.is_empty() {
            return None;
        }
        Some((code.to_string(), message.to_string()))
    }

    /// Start streaming generation with diagnostic error envelope for pipeline consumers.
    pub async fn stream_generate_for_pipeline(
        &self,
        req: UnifiedGenerateRequest,
        skill_registry: Option<&Arc<StdRwLock<SkillRegistry>>>,
    ) -> Result<mpsc::Receiver<StreamChunk>, AppErrorDto> {
        self.stream_generate_inner(req, skill_registry, true).await
    }

    async fn stream_generate_inner(
        &self,
        req: UnifiedGenerateRequest,
        skill_registry: Option<&Arc<StdRwLock<SkillRegistry>>>,
        with_pipeline_error_envelope: bool,
    ) -> Result<mpsc::Receiver<StreamChunk>, AppErrorDto> {
        let (provider_id, model, route) = match skill_registry {
            Some(registry) => {
                let guard = registry.read().map_err(|err| {
                    AppErrorDto::new("SKILLS_LOCK_FAILED", "技能注册表加锁失败", false)
                        .with_detail(err.to_string())
                })?;
                Self::resolve_with_skill_override(&req, &guard)?
            }
            None => Self::resolve_request_target(&req)?,
        };
        let attempts = Self::build_attempt_chain(&provider_id, &model, route.as_ref());

        let (tx, rx) = mpsc::channel(256);
        let service = self.clone();

        tokio::spawn(async move {
            let mut last_error: Option<AppErrorDto> = None;

            for (attempt_provider, attempt_model_hint) in attempts {
                let resolved_model =
                    match Self::resolve_request_model(&attempt_provider, &attempt_model_hint) {
                        Ok(model_id) => model_id,
                        Err(e) => {
                            last_error = Some(e);
                            continue;
                        }
                    };

                if let Err(e) = service.ensure_provider_registered(&attempt_provider).await {
                    last_error = Some(e);
                    continue;
                }

                let guard = service.adapters.read().await;
                if let Some(adapter) = guard.get(&attempt_provider) {
                    let mut attempt_req = req.clone();
                    attempt_req.provider_id = Some(attempt_provider.clone());
                    attempt_req.model = resolved_model;
                    match adapter.stream_text(attempt_req, tx.clone()).await {
                        Ok(()) => return,
                        Err(e) => {
                            last_error = Some(AppErrorDto::from(e));
                            continue;
                        }
                    }
                } else {
                    last_error = Some(AppErrorDto::new(
                        "LLM_ADAPTER_NOT_FOUND",
                        &format!("供应商 '{}' 未注册", attempt_provider),
                        true,
                    ));
                    continue;
                }
            }

            let terminal_error = last_error.unwrap_or_else(|| {
                AppErrorDto::new("LLM_GENERATE_FAILED", "所有供应商均调用失败", false)
            });
            let error_text = if with_pipeline_error_envelope {
                Self::encode_pipeline_stream_error(&terminal_error)
            } else {
                terminal_error.message.clone()
            };
            let _ = tx
                .send(StreamChunk {
                    content: String::new(),
                    finish_reason: None,
                    request_id: String::new(),
                    error: Some(error_text),
                    reasoning: None,
                })
                .await;
        });

        Ok(rx)
    }

    /// Test a provider's connection.
    pub async fn test_connection(&self, provider_id: &str) -> Result<(), AppErrorDto> {
        self.ensure_provider_registered(provider_id).await?;
        let guard = self.adapters.read().await;
        let adapter = guard.get(provider_id).ok_or_else(|| {
            AppErrorDto::new(
                "LLM_ADAPTER_NOT_FOUND",
                &format!("供应商 '{}' 未注册", provider_id),
                true,
            )
        })?;
        adapter.test_connection().await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::AiService;
    use crate::adapters::llm_types::{ModelPoolEntry, ModelPoolRecord, ProviderConfig, TaskRoute};
    use crate::errors::AppErrorDto;
    use crate::infra::app_database;
    use crate::services::skill_registry::{SkillManifest, SkillRegistry, SkillTaskRouteOverride};
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    fn create_test_registry(name: &str) -> SkillRegistry {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root: PathBuf =
            std::env::temp_dir().join(format!("novelforge-ai-service-test-{name}-{unique}"));
        let skills_dir = root.join("skills");
        let builtin_dir = root.join("builtin");
        std::fs::create_dir_all(&skills_dir).expect("create skills dir");
        std::fs::create_dir_all(&builtin_dir).expect("create builtin dir");
        SkillRegistry::new(skills_dir, builtin_dir)
    }

    fn build_manifest(id: &str, class: &str) -> SkillManifest {
        SkillManifest {
            id: id.to_string(),
            name: format!("{id} name"),
            description: "desc".to_string(),
            version: 1,
            source: "user".to_string(),
            category: "utility".to_string(),
            tags: Vec::new(),
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: serde_json::json!({"type":"object"}),
            requires_user_confirmation: true,
            writes_to_project: false,
            author: None,
            icon: None,
            created_at: "2026-04-30T00:00:00Z".to_string(),
            updated_at: "2026-04-30T00:00:00Z".to_string(),
            skill_class: Some(class.to_string()),
            bundle_ids: Vec::new(),
            always_on: false,
            trigger_conditions: Vec::new(),
            required_contexts: Vec::new(),
            state_writes: Vec::new(),
            automation_tier: None,
            scene_tags: Vec::new(),
            affects_layers: Vec::new(),
            task_route: None,
        }
    }

    fn setup_app_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory app db");
        crate::infra::migrator::run_app_pending(&conn).expect("run app migrations");
        conn
    }

    fn sample_provider(id: &str, default_model: &str) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            display_name: id.to_string(),
            vendor: "openai".to_string(),
            protocol: "openai_responses".to_string(),
            base_url: "https://example.invalid".to_string(),
            endpoint_path: None,
            api_key: None,
            auth_mode: "bearer".to_string(),
            auth_header_name: None,
            anthropic_version: None,
            beta_headers: None,
            custom_headers: None,
            default_model: Some(default_model.to_string()),
            timeout_ms: 120_000,
            connect_timeout_ms: 15_000,
            max_retries: 2,
            model_refresh_mode: Some("registry".to_string()),
            models_path: None,
            last_model_refresh_at: None,
        }
    }

    #[test]
    fn pipeline_stream_error_roundtrip() {
        let encoded = AiService::encode_pipeline_stream_error(&AppErrorDto::new(
            "TASK_ROUTE_NOT_FOUND",
            "route missing",
            true,
        ));
        let parsed = AiService::decode_pipeline_stream_error(&encoded).expect("decode");
        assert_eq!(parsed.0, "TASK_ROUTE_NOT_FOUND");
        assert_eq!(parsed.1, "route missing");
    }

    #[test]
    fn decode_pipeline_stream_error_rejects_plain_message() {
        assert!(AiService::decode_pipeline_stream_error("just message").is_none());
    }

    #[test]
    fn stream_generate_for_pipeline_uses_skill_route_override() {
        let registry = create_test_registry("route-override");
        let mut workflow = build_manifest("workflow.scene.render", "workflow");
        workflow.trigger_conditions = vec!["custom.scene.render".to_string()];
        workflow.task_route = Some(SkillTaskRouteOverride {
            task_type: "custom.scene.render".to_string(),
            provider_id: "provider-override".to_string(),
            model_id: "model-override".to_string(),
            reason: Some("precision scene rendering".to_string()),
        });
        registry
            .create_skill(&workflow, "workflow body")
            .expect("create workflow skill");

        let resolution =
            AiService::inspect_task_route_with_skill_registry("custom.scene.render", &registry)
                .expect("resolve route with skill override");
        assert_eq!(resolution.provider_id, "provider-override");
        assert_eq!(resolution.model_id, "model-override");
    }

    #[test]
    fn model_pool_route_mapping_prefers_pool_entry_and_uses_fallback_pool() {
        let conn = setup_app_conn();
        let now = crate::infra::time::now_iso();
        app_database::upsert_provider(&conn, &sample_provider("deepseek", "deepseek-chat"), &now)
            .expect("upsert deepseek");
        app_database::upsert_provider(&conn, &sample_provider("openai", "gpt-5.5"), &now)
            .expect("upsert openai");

        let primary_pool = ModelPoolRecord {
            id: "drafter".to_string(),
            display_name: "Drafter Pool".to_string(),
            role: "drafter".to_string(),
            enabled: true,
            entries: vec![ModelPoolEntry {
                provider_id: "deepseek".to_string(),
                model_id: "deepseek-v4-flash".to_string(),
            }],
            fallback_pool_id: Some("reviewer".to_string()),
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };
        let fallback_pool = ModelPoolRecord {
            id: "reviewer".to_string(),
            display_name: "Reviewer Pool".to_string(),
            role: "reviewer".to_string(),
            enabled: true,
            entries: vec![ModelPoolEntry {
                provider_id: "openai".to_string(),
                model_id: "gpt-5.5".to_string(),
            }],
            fallback_pool_id: None,
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };
        app_database::upsert_model_pool(&conn, &primary_pool, &now).expect("upsert primary pool");
        app_database::upsert_model_pool(&conn, &fallback_pool, &now).expect("upsert fallback pool");

        let route = TaskRoute {
            id: Uuid::new_v4().to_string(),
            task_type: "chapter.draft".to_string(),
            provider_id: "legacy-provider".to_string(),
            model_id: "legacy-model".to_string(),
            fallback_provider_id: None,
            fallback_model_id: None,
            model_pool_id: Some("drafter".to_string()),
            fallback_model_pool_id: None,
            post_tasks: Vec::new(),
            max_retries: 2,
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };

        let (provider_id, model_id, resolved_route) =
            AiService::resolve_with_model_pool_if_configured(&conn, "chapter.draft", &route)
                .expect("resolve with model pool");
        assert_eq!(provider_id, "deepseek");
        assert_eq!(model_id, "deepseek-v4-flash");
        assert_eq!(resolved_route.model_pool_id.as_deref(), Some("drafter"));
        assert_eq!(
            resolved_route.fallback_model_pool_id.as_deref(),
            Some("reviewer")
        );
        assert_eq!(
            resolved_route.fallback_provider_id.as_deref(),
            Some("openai")
        );
        assert_eq!(resolved_route.fallback_model_id.as_deref(), Some("gpt-5.5"));
    }
}
