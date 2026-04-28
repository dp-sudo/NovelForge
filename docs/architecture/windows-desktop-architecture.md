# NovelForge Windows 桌面端技术架构文档

## 1. 文档信息
- 版本：v0.5
- 状态：S17 发布能力接入（语义检索 / Git / 授权 / 自动更新）
- 最后更新：2026-04-27
- 代码基线：`src/` + `src-tauri/src/`

## 2. 架构目标
- 本地优先：项目数据与正文文件默认本机持久化
- 主闭环优先：项目 -> 蓝图/资产 -> 章节写作 -> 检查 -> 导出
- 安全基线：API Key 不写入项目目录和日志明文
- 主链路收口：项目/蓝图/资产/章节/编辑/检查链路以 Tauri command 为唯一默认执行路径

## 3. 技术栈
- 桌面框架：Tauri 2.x
- 前端：React + TypeScript + Zustand + Vite
- 后端本地服务：Rust
- 本地数据库：SQLite（`rusqlite`）
- 网络客户端：`reqwest`（模型注册表与供应商能力探测）

## 4. 分层设计（当前实现）
### 4.1 Renderer（`src/`）
- 页面、交互、状态存储
- 通过 `src/api/*` 调用 Tauri command
- 兼容回退仅保留在非主闭环模块（如设置/导出）
- S16 新增页面：`TimelinePage`, `RelationshipsPage`

### 4.2 API 适配层（`src/api/*`）
- `tauriClient.invokeCommand()` 统一 `invoke` 与错误解析
- 主闭环 API 已去除隐式 fallback，强制透传 `projectRoot`
- `contextApi` 已接入 Rust command（`get_chapter_context`）
- S16 新增：`timelineApi`（`list_timeline_entries`）
- S17 新增：`settingsApi` 中 Git/授权/更新调用（仍保持编辑器设置 localStorage）

### 4.3 Command 层（`src-tauri/src/commands/*`）
- 参数反序列化、调用 service、返回 DTO
- 由 `src-tauri/src/lib.rs` 统一注册 invoke handler

### 4.4 Service 层（`src-tauri/src/services/*`）
- 业务规则、事务更新、导出、AI 调用、上下文收集、Prompt 构建
- `AppState` 当前包含：
  - `ProjectService`, `ChapterService`, `BlueprintService`
  - `CharacterService`, `RelationshipService`
  - `WorldService`, `GlossaryService`, `PlotService`
  - `ConsistencyService`, `ExportService`, `DashboardService`
  - `SettingsService`, `ModelRegistryService`
  - `VectorService`, `GitService`, `LicenseService`
  - `AiService`, `ContextService`, `SkillRegistry`

### 4.5 Infra 层（`src-tauri/src/infra/*`）
- `database.rs`：项目级 SQLite schema
- `app_database.rs`：应用级 SQLite schema
- `credential_manager.rs`：Windows Credential Manager API Key 存取
- `fs_utils.rs`：原子写入（temp + rename）
- `path_utils.rs`、`recent_projects.rs`、`time.rs`

## 5. 数据与存储协议
### 5.1 项目目录协议（Project Root）
- `project.json`
- `database/project.sqlite`
- `manuscript/chapters/`（章节正文）
- `manuscript/drafts/`（自动保存草稿）
- `database/vector-index.json`（本地语义检索索引）
- `.git/`（可选，Git 快照功能初始化后创建）
- `exports/`
- 以及 `blueprint/`, `assets/`, `prompts/`, `workflows/`, `logs/` 等目录

### 5.2 项目级数据库（`database/project.sqlite`）
- 核心表：`projects`, `chapters`, `blueprint_steps`, `characters`, `world_rules`, `glossary_terms`, `plot_nodes`, `chapter_links`, `consistency_issues`, `character_relationships`, `snapshots`, `volumes`
- 运行表：`ai_requests`
- 兼容性保留表：`llm_providers`, `llm_models`, `llm_task_routes`, `llm_model_registry_state`, `llm_model_refresh_logs`

### 5.3 应用级数据库（`~/.novelforge/novelforge.db`）
- `llm_providers`, `llm_models`, `llm_model_refresh_logs`, `llm_task_routes`, `llm_model_registry_state`, `app_settings`
- 用于跨项目 Provider/模型/路由配置

### 5.4 应用级本地文件
- `~/.novelforge/license.json`：授权离线缓存（掩码 + hash + 激活时间）

## 6. AI 架构（当前）
- Prompt 构建：`PromptBuilder`（章节草稿、续写、改写、去 AI 味、蓝图建议、角色生成、一致性扫描）
- 上下文收集：`ContextService`（全局 + 章节关联 + 上一章摘要 + 资产抽取候选）
- 任务路由：`AiService.generate_text()` 支持 `task_type -> llm_task_routes`
- 流式路由：`AiService.stream_generate()` 同步支持 `task_type -> llm_task_routes`，并在运行时懒加载 Provider 适配器
- 流式事件：`ai:stream-chunk:{id}`、`ai:stream-done:{id}`
- 模型管理：`ModelRegistryService` 支持刷新模型、读取刷新日志、检查/应用远端 registry（含 URL 安全约束、Schema 与签名字段校验闸）

## 7. 命令面（摘要）
- Project：`validate_project`, `create_project`, `open_project`, `list_recent_projects`, `clear_recent_projects`, `init_project_repository`, `get_project_repository_status`, `commit_project_snapshot`, `list_project_history`
- Chapter：`list_chapters`, `list_timeline_entries`, `create_chapter`, `save_chapter_content`, `autosave_draft`, `recover_draft`, `delete_chapter`
- Search：`search_project`, `search_project_semantic`, `rebuild_search_index`, `rebuild_vector_index`
- Context：`get_chapter_context`
- Blueprint / Character / World / Glossary / Plot / Consistency / Dashboard：对应 CRUD 与统计命令
- Export：`export_chapter`, `export_book`（`txt/md/docx/pdf/epub`）
- Settings：授权、自动更新、Provider、模型刷新、任务路由、远端 registry、兼容命令
- AI：`generate_ai_preview`, `stream_ai_generate`, `stream_ai_chapter_task`, `ai_scan_consistency`, `generate_blueprint_suggestion`, `ai_generate_character`, `register_ai_provider`, `test_ai_connection`, `list_skills`

## 8. 当前过渡态与风险
- 导出链路仍保留兼容 fallback，需在后续阶段收口。
- 编辑器设置仍使用 localStorage，尚未迁移到应用级配置存储。
- 向量索引会随章节规模增长，需通过分块上限控制磁盘占用。
- Git 快照依赖本机 `git` 可执行文件；缺失时会返回明确错误码。
- 自动更新依赖发布端点与签名配置，配置错误会导致更新失败。
- 若前端 DTO 与 `get_chapter_context` 返回结构漂移，编辑器上下文面板会出现渲染异常。
- `SkillRegistry` 在 `AppState::default()` 下为默认空集合；`list_skills` 当前可能返回空。
- AI 流式路径依赖 Provider 注册与路由配置，未完成配置时可能无有效内容输出。

## 9. 文档维护规则
以下变更必须同步更新本文档：
- `AppState` service 组成变化
- 新增/删除 command
- 存储协议（目录/数据库/凭据）变化
- AI 调用链或路由策略变化
