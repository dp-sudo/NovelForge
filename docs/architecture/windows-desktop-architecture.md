# NovelForge Windows 桌面端技术架构文档

## 1. 文档信息
- 版本：v0.12
- 状态：S24（阶段四：反馈生命周期闭环、池级路由策略推荐、场景分类离线回归）
- 最后更新：2026-05-02
- 代码基线：`src/` + `src-tauri/src/`

## 2. 架构目标
- 本地优先：项目数据与正文文件默认本机持久化。
- 主闭环优先：项目 -> 蓝图/资产 -> 章节写作 -> AI 任务 -> 人工确认 -> 检查 -> 导出。
- 安全基线：API Key 不写入项目目录与日志明文（存于 Windows Credential Manager）。
- 主链路收口：前端业务能力默认通过 Tauri command 调用，不走并行 Node 业务运行时。
- 五层对象落地：`Story Constitution` / `Canon Registry` / `State Ledger` / `Execution Workspace` / `Review Trail`。

### 2.1 AI 生产系统对象（运行期）
- `Story Constitution`：`blueprint_steps` + `projects.ai_strategy_profile`（目标、约束、默认策略）。
- `Canon Registry`：正式资产表（`characters/world_rules/plot_nodes/glossary_terms/narrative_obligations/chapters`）。
- `State Ledger`：`story_state`（章节写后状态增量，如窗口进度、结构化确认状态、技能运行时状态写入）。
- `Execution Workspace`：`ai_pipeline_runs`、`structured_draft_*`、`ai_requests` 与编辑器执行态。
- `Review Trail`：`entity_provenance` + `user_review_actions`（approved/rejected/edited 理由与明细）。

## 3. 技术栈
- 桌面框架：Tauri 2.x
- 前端：React + TypeScript + Zustand + Vite
- 后端本地服务：Rust
- 本地数据库：SQLite（`rusqlite`）
- 网络客户端：`reqwest`（模型探活、远端 registry 检查）

## 4. 分层设计（当前实现）
### 4.1 Renderer（`src/`）
- 页面、交互、状态存储。
- 通过 `src/api/*` 调用 Tauri command。
- 编辑器 AI 主链路使用 `pipelineApi`（`runTaskPipeline` + `streamTaskPipeline`）。
- 编辑器执行层已拆分为：
  - `usePipelineStream`（流式驱动、错误映射、取消语义）。
  - `EditorContextPanel`（上下文标签、候选采纳、结构化草案确认）。
- 编辑器右侧上下文面板支持：
  - `assetCandidates` 候选采纳。
  - `relationshipDrafts` / `involvementDrafts` / `sceneDrafts` 人工确认入库。

### 4.2 API 适配层（`src/api/*`）
- `tauriClient.invokeCommand()` 统一调用与错误对象转换。
- 关键模块：`projectApi`, `chapterApi`, `contextApi`, `pipelineApi`, `settingsApi`, `skillsApi`。
- 当前业务 API 为 invoke-only（未使用 DevEngine fallback 主路径）。

### 4.3 Command 层（`src-tauri/src/commands/*`）
- 参数反序列化、调用 service、返回 DTO。
- 由 `src-tauri/src/lib.rs` 统一注册 invoke handler。
- 重点新增面：
  - AI Pipeline：`run_ai_task_pipeline`, `cancel_ai_task_pipeline`。
  - 结构化确认：`apply_asset_candidate`, `apply_structured_draft`, `reject_structured_draft`, `get_review_trail`。
  - 回报查询与流转：`get_feedback_events`、`acknowledge_feedback_event`、`resolve_feedback_event`、`ignore_feedback_event`。
  - 模型池管理：`list_model_pools`, `create_model_pool`, `update_model_pool`, `delete_model_pool`。
  - 路由策略推荐：`recommend_routing_strategy`、`apply_routing_strategy_template`、`get_project_routing_strategy`。
  - 晋升策略：`list_promotion_policies`, `save_promotion_policy`。
  - 写作风格：`save_writing_style`, `get_writing_style`。
  - 项目级 AI 策略：`save_ai_strategy_profile`, `get_ai_strategy_profile`。
  - 技能管理：`get_skill`, `get_skill_content`, `create_skill`, `update_skill`, `delete_skill`, `import_skill_file`, `reset_builtin_skill`, `refresh_skills`。

### 4.4 Service 层（`src-tauri/src/services/*`）
- `AppState` 托管服务：
  - `AiPipelineService`, `AiService`
  - `BackupService`, `ImportService`, `ExportService`
  - `ProjectService`, `ChapterService`, `VolumeService`
  - `BlueprintService`, `CharacterService`, `RelationshipService`
  - `WorldService`, `GlossaryService`, `PlotService`, `NarrativeService`
  - `ContextService`, `ConsistencyService`, `DashboardService`, `IntegrityService`
  - `SearchService`, `VectorService`
  - `SettingsService`, `ModelRegistryService`, `GitService`, `LicenseService`
  - `SkillRegistry`（Arc<RwLock<SkillRegistry>>）
- 独立服务（按需实例化）：
  - `StoryStateService`（状态账本管理，在 `ChapterService`、`ContextService`、`AiPipelineService` 中直接实例化使用）
  - `PromotionService`（统一晋升策略校验与晋升执行入口）
  - `ReviewTrailService`（用户审查动作查询/写入）
  - `FeedbackService`（回报规则检测、事件状态机与闭环备注回写）

### 4.5 Infra 层（`src-tauri/src/infra/*`）
- `migrator.rs` + `migrations/*`：项目库/应用库迁移管理。
- `database.rs`：项目库初始化与兼容补列（含 `projects.writing_style`、`projects.ai_strategy_profile`）。
- `app_database.rs`：应用级 Provider/模型/路由/编辑器配置存储。
- `credential_manager.rs`：API Key 与系统凭据管理。
- `fs_utils.rs`：原子写入（temp + rename）。
- `recent_projects.rs`, `path_utils.rs`, `time.rs` 等基础设施。

## 5. 数据与存储协议
### 5.1 项目目录协议（Project Root）
- `project.json`
- `database/project.sqlite`
- `manuscript/chapters/`（正文）
- `manuscript/drafts/`（自动保存）
- `manuscript/snapshots/`（章节快照）
- `database/vector-index.json`（语义索引）
- `exports/`
- `backups/`
- 可选 `.git/`（Git 快照能力初始化后创建）

### 5.2 项目级数据库（`database/project.sqlite`）
- 核心表：
  - `projects`, `chapters`, `blueprint_steps`, `characters`, `world_rules`, `glossary_terms`, `plot_nodes`, `chapter_links`
  - `character_relationships`, `consistency_issues`, `narrative_obligations`, `snapshots`, `volumes`
- AI/运行审计与闭环表：
  - `ai_requests`
  - `ai_pipeline_runs`
  - `structured_draft_batches`
  - `structured_draft_items`
  - `entity_provenance`
  - `user_review_actions`
  - `story_state`
  - `feedback_events`
- 迁移现状：
  - `project/0001_init.sql`
  - `project/0002_task_route_unique.sql`（任务类型 canonical + 去重 + 唯一索引）
  - `project/0003_pipeline_draft_pool.sql`（Pipeline run 审计 + 草案池）
  - `project/0004_ai_strategy_profile.sql`（项目级 AI 策略配置列）
  - `project/0005_entity_provenance.sql`（正式资产来源轨迹）
  - `project/0006_story_state.sql`（状态账本）
  - `project/0007_blueprint_certainty_zones.sql`（蓝图确定性分区显式字段 `certainty_zones_json`）
  - `project/0008_pipeline_run_meta.sql`（Pipeline run 元数据 `meta_json`）
  - `project/0009_user_review_actions.sql`（审查轨迹事件表）
  - `project/0010_feedback_events.sql`（回报事件 + `ai_pipeline_runs.post_task_results`）
  - `project/0011_feedback_event_lifecycle.sql`（回报生命周期字段：`resolved_at/resolved_by/resolution_note`）
  - `project/0012_project_routing_strategy.sql`（项目级路由策略 ID：`projects.routing_strategy_id`）
- 兼容补列：
  - `database.rs::ensure_compatible_schema()` 在打开/初始化时补齐 `projects.writing_style`、`projects.ai_strategy_profile` 等历史缺列。

### 5.3 应用级数据库（`%LOCALAPPDATA%\\NovelForge\\novelforge.db`）
- 表：`llm_providers`, `llm_models`, `llm_model_refresh_logs`, `llm_model_pools`, `llm_task_routes`, `llm_model_registry_state`, `app_settings`, `promotion_policies`, `feedback_rules`
- 迁移现状：
  - `app/0001_init.sql`
  - `app/0002_skill_index.sql`
  - `app/0003_task_route_unique.sql`（canonical + 去重 + `task_type` 唯一索引）
  - `app/0004_model_pools.sql`（模型池表 + 路由池字段）
  - `app/0005_promotion_policies.sql`（统一晋升策略）
  - `app/0006_feedback_rules.sql`（回报规则 + `llm_task_routes.post_tasks_json`）
- 编辑器设置通过 `app_settings` 的 `editor_settings` 键持久化。
- 应用级 Provider/模型/路由运行期真相源在 `novelforge.db`；项目级 AI 策略真相源在 `project.sqlite`。

### 5.4 应用级本地文件
- `%LOCALAPPDATA%\\NovelForge\\license.json`：授权离线缓存。

## 6. AI 架构（当前）
### 6.1 任务路由与 canonical
- 路由表：应用级 `llm_task_routes`。
- canonical 函数：`task_routing::canonical_task_type()`。
- 路由解析：`AiService::resolve_route()`。
- 未命中核心任务时，可回退 `custom` 路由（若已配置）。
- 新增池级兼容链路：`task route -> model pool -> provider/model`。
  - 池表：`llm_model_pools`（`planner/drafter/reviewer/extractor/state`）。
  - 任务路由可选字段：`model_pool_id`、`fallback_model_pool_id`。
  - 兼容模式：无池配置时回退原 `provider/model/fallback` 路由。

### 6.2 Pipeline 编排（`AiPipelineService`）
- 阶段：`validate -> context -> route -> prompt -> generate -> postprocess -> persist -> done`
- 命令入口：
  - `run_ai_task_pipeline`（新协议）
  - `cancel_ai_task_pipeline`
- 前端策略入口：
  - 新调用显式传 `persistMode/automationTier`。
  - `autoPersist` 仅作 legacy bridge 推导；触发时记录 `PIPELINE.LEGACY_POLICY_BRIDGE` 诊断事件。
- 事件协议：
  - 主事件：`ai:pipeline:event`
  - 事件类型：`start | progress | delta | done | error`

### 6.3 Prompt 解析策略
- 优先读取技能 Markdown 模板（`SkillRegistry`）。
- 未命中模板时回退 `PromptBuilder`。
- 章节任务在 `prompt` 前会先编译 `ContinuityPack`（Constitution/Canon/Lexicon Policy/State/Promise/Window/Recent）。
- `projects.writing_style` 与已选技能栈（workflow/capability/extractor/policy/review）会共同注入提示词。

### 6.4 运行期技能消费与路由覆盖
- `orchestrator` 在 `route` 阶段构建运行时技能选择上下文，并统一执行 `select_skills_for_task_with_context`。
- 运行时技能选择上下文包含：`alwaysOnPolicySkills`、`defaultCapabilityBundles`、请求级 `skillSelection` 覆盖、当前 `automationTier`、可用上下文键、推断出的场景标签。
- `orchestrator` 先解析本次请求最终 provider/model，再把显式路由写入生成请求，避免 `route` 与 `generate` 阶段重复按旧逻辑二次选技能。
- 若技能声明 `affectsLayers`，`orchestrator` 会按聚合层焦点裁剪 `ContinuityPack`（constitution/lexicon 护栏层固定保留）。
- 章节任务新增场景后置链：
  - `SceneClassifier` 分类 `dialogue/action/exposition/introspection/combat`。
  - `PostTaskExecutor` 合并默认后置任务与路由 `post_tasks`（`review_continuity/extract_state/extract_assets`）。
  - 后置结果写入 `ai_pipeline_runs.post_task_results` 供审计回放。
- `ai:pipeline:event.meta` 回传所选技能数量、技能 IDs、stateWrites、affectsLayers、激活 bundle、推断 scene tag 与 route override 元信息，便于审计。
- `ai_pipeline_runs.meta_json` 会记录路由决策（包含 `modelPoolId` 与 fallback 池信息），用于回放与审计。

### 6.5 写后回写与来源轨迹
- `save_chapter_content` 写正文后调用 `StoryStateService.record_window_progress` 回写窗口状态。
- `task_handlers.persist_task_output` 在正式资产写入后统一记录 `entity_provenance`。
- 若已激活技能声明 `stateWrites`，`task_handlers` 会按项目级 `stateWritePolicy` 追加运行时 `story_state` 记录。
- 手动 CRUD（character/world/plot/glossary/narrative）创建时写入 `source_kind = "user_input"`。

### 6.6 编辑器结构化抽取闭环
- `ContextService.collect_editor_context()` 从章节内容抽取：
  - 资产候选：`assetCandidates`
  - 结构化草案：`relationshipDrafts`, `involvementDrafts`, `sceneDrafts`
- 抽取结果先进入草案池（pending）。
- 用户在 UI 手动确认后调用 `apply_structured_draft` 落库并回写状态；可调用 `reject_structured_draft` 记录否决理由。
- 审查动作（approved/rejected/edited）写入 `user_review_actions`，编辑器右侧面板支持按章节查看审查轨迹。

## 7. 命令面（按域摘要）
- Project：项目创建/打开/最近项目 + 写作风格/项目级 AI 策略保存读取 + Git 仓库与快照。
- Chapter：章节 CRUD、重排、自动保存/恢复、快照、卷管理。
- AI：pipeline run/cancel，模块化 AI 任务通过前端 API 薄封装统一转发到 pipeline（legacy stream 命令已移除）。
- Context：上下文聚合、资产候选采纳、结构化草案确认/否决、审查轨迹查询。
- Dashboard：统计总览 + 回报事件生命周期面板（open/acknowledged/resolved/ignored + 操作闭环）。
- Settings：Provider/模型/模型池/路由/registry、编辑器设置、授权、更新。
- Settings 路由页支持“推荐策略 -> 应用模板 -> 手动覆盖”的池级协同链路。
- Skills：技能列表/详情/内容读取、创建、编辑、删除、导入、重置、重载。
- Search/Integrity：关键字+语义检索、索引重建、项目完整性检查。

## 8. 当前过渡态与风险
- compatibility-only 命令（`load_provider_config`、`save_provider_config`、`register_ai_provider`、`test_ai_connection`）仅用于历史调用兼容，计划在 `2026-07-31` 后移除。
- compatibility-only 命令调用会同时记录 deprecated 统计与 `compatibility_bridge.used` 行为日志，便于收敛与下线审计。
- Pipeline 对模型可用性、API Key、路由配置依赖强，未配置时会在 `route/generate` 阶段显式失败。
- 结构化抽取为启发式规则，存在误报；当前策略是“先草案、再人工确认入库”。
- 向量索引与草案池都会随项目规模增长，需持续关注 DB 体积与索引策略。
- 自动更新仍依赖发布端点与签名配置。

## 9. 文档维护规则
以下变更必须同步更新本文档：
- `AppState` 服务组成变化。
- Tauri command 注册增删。
- 迁移文件新增/删除、索引策略变化、兼容补列策略变化。
- AI Pipeline 阶段、事件协议、错误码语义变化。
- 结构化草案池字段与“人工确认入库”行为变化。
