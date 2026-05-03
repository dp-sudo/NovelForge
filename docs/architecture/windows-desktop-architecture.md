# NovelForge Windows 桌面端技术架构文档

## 1. 文档信息
- 版本：v0.7
- 状态：生产就绪（AI Pipeline v1 + 结构化草案池 + 写作风格 + 技能管理 + 完整服务层）
- 最后更新：2026-05-03
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
- 编辑器 AI 主链路使用 `pipelineApi`（`runTaskPipeline` + `streamTaskPipelineByRequestId`）。
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
  - 技能管理：`get_skill`, `get_skill_content`, `create_skill`, `update_skill`, `delete_skill`, `import_skill_file`, `reset_builtin_skill`, `refresh_skills`。

### 4.4 Service 层（`src-tauri/src/services/*`）
- `AppState` 现有服务（完整列表）：
  - **AI 相关**: `AiPipelineService`, `AiService`
  - **项目与章节**: `ProjectService`, `ChapterService`, `VolumeService`
  - **资产管理**: `CharacterService`, `RelationshipService`, `WorldService`, `GlossaryService`, `PlotService`, `NarrativeService`, `BlueprintService`
  - **上下文与一致性**: `ContextService`, `ConsistencyService`
  - **搜索与索引**: `SearchService`, `VectorService`
  - **导入导出**: `ImportService`, `ExportService`, `BackupService`
  - **版本控制**: `GitService`
  - **设置与配置**: `SettingsService`, `ModelRegistryService`, `LicenseService`
  - **统计与完整性**: `DashboardService`, `IntegrityService`
  - **技能管理**: `SkillRegistry` (Arc<RwLock<SkillRegistry>>)

### 4.5 Infra 层（`src-tauri/src/infra/*`）
- `migrator.rs` + `migrations/*`：项目库/应用库迁移管理。
- `database.rs`：项目库初始化与兼容补列（含 `projects.writing_style`）。
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
- 兼容补列：
  - `database.rs::ensure_compatible_schema()` 在打开/初始化时补齐 `projects.writing_style` 等历史缺列。

### 5.3 应用级数据库（`~/.novelforge/novelforge.db`）
- 表：`llm_providers`, `llm_models`, `llm_model_refresh_logs`, `llm_task_routes`, `llm_model_registry_state`, `app_settings`
- 迁移现状：
  - `app/0001_init.sql`
  - `app/0002_skill_index.sql`
  - `app/0003_task_route_unique.sql`（canonical + 去重 + `task_type` 唯一索引）
- 编辑器设置通过 `app_settings` 的 `editor_settings` 键持久化。

### 5.4 应用级本地文件
- `~/.novelforge/license.json`：授权离线缓存。

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
### 7.1 Project 命令
- `validate_project`: 项目名称校验与规范化
- `create_project`: 创建新项目（目录、数据库、project.json）
- `open_project`: 打开现有项目
- `list_recent_projects`: 列出最近打开的项目
- `clear_recent_projects`: 清空最近项目列表
- `save_writing_style`: 保存项目写作风格配置
- `get_writing_style`: 获取项目写作风格配置
- `init_project_repository`: 初始化 Git 仓库
- `get_project_repository_status`: 获取 Git 仓库状态
- `commit_project_snapshot`: 提交 Git 快照
- `list_project_history`: 列出 Git 提交历史

### 7.2 Chapter 命令
- `list_chapters`: 列出所有章节
- `list_timeline_entries`: 列出时间线条目
- `reorder_chapters`: 重新排序章节
- `create_chapter`: 创建新章节
- `save_chapter_content`: 保存章节正文
- `autosave_draft`: 自动保存草稿
- `recover_draft`: 恢复草稿
- `read_chapter_content`: 读取章节正文
- `delete_chapter`: 删除章节（软删除）
- `create_snapshot`: 创建章节快照
- `list_snapshots`: 列出快照
- `read_snapshot_content`: 读取快照内容
- `list_volumes`: 列出卷
- `create_volume`: 创建卷
- `delete_volume`: 删除卷
- `assign_chapter_volume`: 分配章节到卷

### 7.3 AI Pipeline 命令
- `run_ai_task_pipeline`: 运行 AI 任务管道（返回 requestId）
- `cancel_ai_task_pipeline`: 取消 AI 任务
- 事件协议：`ai:pipeline:event`（start | progress | delta | done | error）

### 7.4 AI 功能任务命令
- `generate_blueprint_suggestion`: 生成蓝图建议
- `ai_generate_character`: 生成角色卡
- `ai_generate_world_rule`: 生成世界规则
- `ai_generate_plot_node`: 生成情节节点
- `ai_generate_glossary_term`: 生成名词表术语
- `ai_generate_narrative_obligation`: 生成叙事义务
- `ai_scan_consistency`: 一致性扫描

### 7.5 Context 命令
- `get_chapter_context`: 获取章节上下文（包含资产候选和结构化草案）
- `apply_asset_candidate`: 采纳资产候选入库
- `apply_structured_draft`: 确认结构化草案入库
- `update_review_queue_item_status`: 更新审查队列项状态
- `list_review_work_items`: 列出审查工作项

### 7.6 Blueprint / Character / World / Glossary / Plot / Narrative 命令
- **Blueprint**: `list_blueprint_steps`, `save_blueprint_step`, `mark_blueprint_completed`, `reset_blueprint_step`
- **Character**: `list_characters`, `create_character`, `update_character`, `delete_character`
- **Relationship**: `list_character_relationships`, `create_character_relationship`, `delete_character_relationship`
- **World**: `list_world_rules`, `create_world_rule`, `delete_world_rule`
- **Glossary**: `list_glossary_terms`, `create_glossary_term`
- **Plot**: `list_plot_nodes`, `create_plot_node`, `reorder_plot_nodes`
- **Narrative**: `list_narrative_obligations`, `create_narrative_obligation`, `update_obligation_status`, `delete_narrative_obligation`

### 7.7 Consistency 命令
- `scan_chapter_consistency`: 扫描章节一致性
- `scan_full_consistency`: 扫描全书一致性
- `list_consistency_issues`: 列出一致性问题
- `update_issue_status`: 更新问题状态

### 7.8 Search / Integrity 命令
- `search_project`: 关键字 + 语义搜索合并
- `search_project_semantic`: 纯语义搜索
- `rebuild_search_index`: 重建关键字索引
- `rebuild_vector_index`: 重建向量索引
- `check_project_integrity`: 项目完整性检查

### 7.9 Export 命令
- `export_chapter`: 导出单章节（txt | md | docx | pdf | epub）
- `export_book`: 导出全书（txt | md | docx | pdf | epub）

### 7.10 Import / Backup 命令
- `import_chapter_files`: 导入章节文件
- `create_backup`: 创建备份
- `list_backups`: 列出备份
- `restore_backup`: 恢复备份

### 7.11 Settings 命令
- **Provider**: `list_providers`, `save_provider`, `load_provider`, `delete_provider`, `test_provider_connection`
- **Model**: `refresh_provider_models`, `get_provider_models`, `get_refresh_logs`
- **Route**: `list_task_routes`, `save_task_route`, `delete_task_route`
- **Registry**: `check_remote_registry`, `apply_registry_update`
- **Editor**: `load_editor_settings`, `save_editor_settings`
- **License**: `get_license_status`, `activate_license`
- **Update**: `check_app_update`, `install_app_update`
- **Compatibility-only** (deprecated): `load_provider_config`, `save_provider_config`, `register_ai_provider`, `test_ai_connection`

### 7.12 Skills 命令
- `list_skills`: 列出所有技能
- `get_skill`: 获取技能元数据
- `get_skill_content`: 获取技能内容
- `create_skill`: 创建自定义技能
- `update_skill`: 更新技能
- `delete_skill`: 删除技能
- `import_skill_file`: 导入技能文件
- `reset_builtin_skill`: 重置内置技能
- `refresh_skills`: 刷新技能列表

### 7.13 Dashboard 命令
- `get_dashboard_stats`: 获取仪表盘统计数据

## 8. 当前过渡态与风险
- compatibility-only 命令（`load_provider_config`、`save_provider_config`、`register_ai_provider`、`test_ai_connection`）仍保留用于历史调用兼容，不作为新接入路径。
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
