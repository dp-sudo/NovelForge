# NovelForge Windows 桌面端技术架构文档

## 1. 文档信息
- 版本：v0.7
- 状态：S18（AI Pipeline v1 + 结构化草案池 + 写作风格/项目级 AI 策略 + 技能管理）
- 最后更新：2026-04-30
- 代码基线：`src/` + `src-tauri/src/`

## 2. 架构目标
- 本地优先：项目数据与正文文件默认本机持久化。
- 主闭环优先：项目 -> 蓝图/资产 -> 章节写作 -> AI 任务 -> 人工确认 -> 检查 -> 导出。
- 安全基线：API Key 不写入项目目录与日志明文（存于 Windows Credential Manager）。
- 主链路收口：前端业务能力默认通过 Tauri command 调用，不走并行 Node 业务运行时。

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
  - 结构化确认：`apply_asset_candidate`, `apply_structured_draft`。
  - 写作风格：`save_writing_style`, `get_writing_style`。
  - 项目级 AI 策略：`save_ai_strategy_profile`, `get_ai_strategy_profile`。
  - 技能管理：`get_skill`, `get_skill_content`, `create_skill`, `update_skill`, `delete_skill`, `import_skill_file`, `reset_builtin_skill`, `refresh_skills`。

### 4.4 Service 层（`src-tauri/src/services/*`）
- `AppState` 现有服务：
  - `AiPipelineService`, `AiService`
  - `BackupService`, `ImportService`
  - `ProjectService`, `ChapterService`, `VolumeService`
  - `BlueprintService`, `CharacterService`, `RelationshipService`
  - `WorldService`, `GlossaryService`, `PlotService`, `NarrativeService`
  - `ContextService`, `ConsistencyService`, `DashboardService`, `IntegrityService`
  - `SearchService`, `VectorService`
  - `SettingsService`, `ModelRegistryService`, `GitService`, `LicenseService`
  - `SkillRegistry`

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
- AI/运行审计表：
  - `ai_requests`
  - `ai_pipeline_runs`
  - `structured_draft_batches`
  - `structured_draft_items`
- 迁移现状：
  - `project/0001_init.sql`
  - `project/0002_task_route_unique.sql`（任务类型 canonical + 去重 + 唯一索引）
  - `project/0003_pipeline_draft_pool.sql`（Pipeline run 审计 + 草案池）
  - `project/0004_ai_strategy_profile.sql`（项目级 AI 策略配置列）
- 兼容补列：
  - `database.rs::ensure_compatible_schema()` 在打开/初始化时补齐 `projects.writing_style`、`projects.ai_strategy_profile` 等历史缺列。

### 5.3 应用级数据库（`%LOCALAPPDATA%\\NovelForge\\novelforge.db`）
- 表：`llm_providers`, `llm_models`, `llm_model_refresh_logs`, `llm_task_routes`, `llm_model_registry_state`, `app_settings`
- 迁移现状：
  - `app/0001_init.sql`
  - `app/0002_skill_index.sql`
  - `app/0003_task_route_unique.sql`（canonical + 去重 + `task_type` 唯一索引）
- 编辑器设置通过 `app_settings` 的 `editor_settings` 键持久化。

### 5.4 应用级本地文件
- `%LOCALAPPDATA%\\NovelForge\\license.json`：授权离线缓存。

## 6. AI 架构（当前）
### 6.1 任务路由与 canonical
- 路由表：应用级 `llm_task_routes`。
- canonical 函数：`task_routing::canonical_task_type()`。
- 路由解析：`AiService::resolve_route()`。
- 未命中核心任务时，可回退 `custom` 路由（若已配置）。

### 6.2 Pipeline 编排（`AiPipelineService`）
- 阶段：`validate -> context -> route -> prompt -> generate -> postprocess -> persist -> done`
- 命令入口：
  - `run_ai_task_pipeline`（新协议）
  - `cancel_ai_task_pipeline`
- 事件协议：
  - 主事件：`ai:pipeline:event`
  - 事件类型：`start | progress | delta | done | error`

### 6.3 Prompt 解析策略
- 优先读取技能 Markdown 模板（`SkillRegistry`）。
- 未命中模板时回退 `PromptBuilder`。
- `projects.writing_style` 会注入到 PromptBuilder 输出（适用章节/改写/检查等任务）。

### 6.4 编辑器结构化抽取闭环
- `ContextService.collect_editor_context()` 从章节内容抽取：
  - 资产候选：`assetCandidates`
  - 结构化草案：`relationshipDrafts`, `involvementDrafts`, `sceneDrafts`
- 抽取结果先进入草案池（pending）。
- 用户在 UI 手动确认后调用 `apply_structured_draft` 落库并回写状态。

## 7. 命令面（按域摘要）
- Project：项目创建/打开/最近项目 + 写作风格保存读取 + Git 仓库与快照。
- Project：项目创建/打开/最近项目 + 写作风格/项目级 AI 策略保存读取 + Git 仓库与快照。
- Chapter：章节 CRUD、重排、自动保存/恢复、快照、卷管理。
- AI：pipeline run/cancel，模块化 AI 任务通过前端 API 薄封装统一转发到 pipeline（legacy stream 命令已移除）。
- Context：上下文聚合、资产候选采纳、结构化草案确认。
- Settings：Provider/模型/路由/registry、编辑器设置、授权、更新。
- Skills：技能列表/详情/内容读取、创建、编辑、删除、导入、重置、重载。
- Search/Integrity：关键字+语义检索、索引重建、项目完整性检查。

## 8. 当前过渡态与风险
- compatibility-only 命令（`load_provider_config`、`save_provider_config`、`register_ai_provider`、`test_ai_connection`）仅用于历史调用兼容，计划在 `2026-07-31` 后移除。
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
