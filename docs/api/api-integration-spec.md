# NovelForge API 集成文档（Frontend <-> Tauri <-> Rust）

## 1. 文档信息
- 版本：v0.5
- 状态：S17 发布能力已接入（主闭环 + Beta 发布能力默认走 Tauri command）
- 最后更新：2026-04-27
- 代码基线：`src/api/*`、`src-tauri/src/commands/*`

## 2. 集成原则（当前）
- 主路径：前端通过 `invokeCommand` 调用 Rust command
- 主闭环页面（项目/蓝图/资产/章节/编辑/检查/统计）不再依赖隐式 DevEngine fallback
- 错误基线：前端与 Rust 统一 `AppErrorDto` 结构
- 文档描述以“现有可验证代码”为准，不以计划为准

## 3. 前端 API 入口
- 统一调用：`src/api/tauriClient.ts`
- 业务模块：
  - `projectApi`, `chapterApi`, `blueprintApi`, `characterApi`, `worldApi`, `glossaryApi`, `plotApi`
  - `consistencyApi`, `statsApi`, `settingsApi`, `aiApi`, `exportApi`, `contextApi`, `timelineApi`
- fallback 引擎：`src/api/dev-engine.ts`

## 4. Command 契约（按模块）
### 4.1 Project
1. `validate_project`
- input: `ValidateProjectInput { name, forceError? }`
- output: `ValidateProjectOutput { normalizedName, message }`
2. `create_project`
- input: `CreateProjectInput`
- output: `ProjectOpenResult`
3. `open_project`
- input: `{ projectRoot }`
- output: `ProjectOpenResult`
4. `list_recent_projects`
- input: 无
- output: `RecentProjectItem[]`
5. `init_project_repository`
- input: `{ projectRoot }`
- output: `GitRepositoryStatus { initialized, branch, hasChanges }`
6. `get_project_repository_status`
- input: `{ projectRoot }`
- output: `GitRepositoryStatus`
7. `commit_project_snapshot`
- input: `{ projectRoot, message? }`
- output: `GitSnapshotResult { noChanges, commit? }`
8. `list_project_history`
- input: `{ projectRoot, limit? }`
- output: `GitCommitRecord[]`

### 4.2 Chapter
1. `list_chapters`
- input: `{ projectRoot }`
- output: `ChapterRecord[]`
2. `create_chapter`
- input: `{ projectRoot, input: ChapterInput }`
- output: `ChapterRecord`
3. `save_chapter_content`
- input: `{ projectRoot, chapterId, content }`
- output: `SaveChapterOutput`
4. `autosave_draft`
- input: `{ projectRoot, chapterId, content }`
- output: `string`（草稿路径）
5. `recover_draft`
- input: `{ projectRoot, chapterId }`
- output: `RecoverDraftResult`
6. `delete_chapter`
- input: `{ projectRoot, input: { id } }`
- output: `void`
7. `list_timeline_entries`
- input: `{ projectRoot }`
- output: `TimelineEntryRecord[]`（章节顺序、卷信息、更新时间）

### 4.3 Blueprint
1. `list_blueprint_steps`
- input: `projectRoot: string`
- output: `BlueprintStep[]`
2. `save_blueprint_step`
- input: `projectRoot: string`, `input: SaveBlueprintStepInput`
- output: `BlueprintStep`
3. `mark_blueprint_completed`
- input: `projectRoot: string`, `stepKey: string`
- output: `void`
4. `reset_blueprint_step`
- input: `projectRoot: string`, `stepKey: string`
- output: `void`

### 4.4 Character / Relationship
- `list_characters(projectRoot)` -> `CharacterRecord[]`
- `create_character(projectRoot, input)` -> `string`
- `update_character(projectRoot, input)` -> `void`
- `delete_character(projectRoot, id)` -> `void`
- `list_character_relationships(projectRoot, characterId?)` -> `CharacterRelationship[]`
- `create_character_relationship(projectRoot, input)` -> `string`
- `delete_character_relationship(projectRoot, id)` -> `void`

### 4.5 World / Glossary / Plot
- World：`list_world_rules`, `create_world_rule`, `delete_world_rule`
- Glossary：`list_glossary_terms`, `create_glossary_term`
- Plot：`list_plot_nodes`, `create_plot_node`, `reorder_plot_nodes(projectRoot, orderedIds)`

### 4.6 Search / Consistency / Dashboard
- `search_project(projectRoot, query, limit?)` -> `SearchResult[]`（已叠加语义召回）
- `search_project_semantic(projectRoot, query, limit?)` -> `VectorSearchResult[]`
- `rebuild_search_index(projectRoot)` -> `number`（FTS + 向量索引总条数）
- `rebuild_vector_index(projectRoot)` -> `number`
- `scan_chapter_consistency(projectRoot, input: { chapterId })` -> `ConsistencyIssue[]`
- `list_consistency_issues(projectRoot)` -> `ConsistencyIssue[]`
- `update_issue_status(projectRoot, issueId, status)` -> `void`
- `get_dashboard_stats(projectRoot)` -> `DashboardStats`

### 4.7 Export
- `export_chapter(input: { projectRoot, chapterId, format, outputPath, options? })`
- `export_book(input: { projectRoot, format, outputPath, options? })`
- `format` 当前支持：`txt | md | docx | pdf | epub`
- 相对输出路径会按 `projectRoot` 解析，避免导出到未知工作目录
- output: `{ outputPath }`

### 4.8 Settings + Model Registry + Task Route
- 授权：`get_license_status`, `activate_license(licenseKey)`
- 更新：`check_app_update`, `install_app_update`
- Provider：`list_providers`, `save_provider(config, apiKey?)`, `load_provider`, `delete_provider`, `test_provider_connection`
- 模型：`refresh_provider_models`, `get_provider_models`, `get_refresh_logs`
- 路由：`list_task_routes`, `save_task_route`, `delete_task_route`
- 远端 registry：`check_remote_registry(url)`, `apply_registry_update(url)`
- 兼容：`load_provider_config`, `save_provider_config`
- 行为补充（S14）：
  - `save_provider` / `save_provider_config` 保存后会触发运行时 Provider 重载，不需要重启应用。
  - `test_provider_connection` 已改为真实探活（按不同适配器返回可区分失败原因）。
  - `check_remote_registry` / `apply_registry_update` 增加 URL 安全约束（HTTPS 或 loopback HTTP）、Schema 校验与签名字段校验；校验失败时不会覆盖本地模型数据。

### 4.9 AI
- `generate_ai_preview(projectRoot, input)` -> `AiPreviewResult`
- `stream_ai_generate(req)` -> `requestId` + 事件流
- `stream_ai_chapter_task(input)` -> `requestId` + 事件流
- `ai_scan_consistency(input)` -> `string`
- `generate_blueprint_suggestion(input)` -> `string`
- `ai_generate_character(input)` -> `string`
- `register_ai_provider(config)` -> `void`
- `test_ai_connection(providerId)` -> `void`
- `list_skills()` -> `SkillManifest[]`

### 4.10 Context
- `get_chapter_context(projectRoot, chapterId)` -> `ChapterContext`
- `ChapterContext` 已新增 `assetCandidates` 字段（章节文本资产抽取候选）

## 5. 标准错误结构
```ts
interface AppErrorDto {
  code: string;
  message: string;
  detail?: string;
  recoverable: boolean;
  suggestedAction?: string;
}
```

## 6. 前端 API 现状分层
### 6.1 主闭环纯 invoke（无隐式 fallback）
- `projectApi`
- `chapterApi`
- `blueprintApi`
- `characterApi`
- `worldApi`
- `glossaryApi`
- `plotApi`
- `consistencyApi`
- `statsApi`
- `contextApi`
- `timelineApi`
- `aiApi`（章节流式主链路 `stream_ai_chapter_task`）
- `tauriClient`

### 6.2 仍保留 fallback（非 S13 收口范围）
- `settingsApi`（仅编辑器设置仍走 DevEngine localStorage）
- `exportApi`

## 7. 当前已知对齐差异（重要）
1. 已修复项（2026-04-27）
- 主闭环页面已统一显式透传 `projectRoot`，移除 `projectRoot?` 隐式降级调用。
- 已新增并接入：
  - `get_chapter_context` command（编辑器上下文面板走 Rust）
  - `delete_chapter` command（章节软删除 + 索引重排）
- 已收口主链路 API 的 silent fallback（`project/chapter/blueprint/character/world/glossary/plot/consistency/stats/context/ai`）。
- S16 Beta 新增：
  - 导出支持 `docx/pdf/epub`（保留 `txt/md`）
  - 时间线 API：`list_timeline_entries`
  - 编辑器上下文新增 `assetCandidates` 候选字段（用于资产抽取预览）
- S17 Beta 新增：
  - 语义检索命令：`search_project_semantic`、`rebuild_vector_index`
  - Git 发布能力命令：`init_project_repository`、`commit_project_snapshot`、`list_project_history`
  - 授权命令：`get_license_status`、`activate_license`
  - 自动更新命令：`check_app_update`、`install_app_update`

2. 兼容性保留（当前策略）
- `settingsApi` 的 Provider/路由/registry 调用已切换为纯 Tauri；仅编辑器设置仍走 localStorage。
- `exportApi` 仍保留兼容 fallback。

## 8. 验收建议
- 接口收敛优先级：
  1. 继续收敛 `exportApi` fallback，并评估编辑器设置迁移到 Tauri
  2. 保持新增 command 的 DTO 与页面渲染字段一致
- 回归检查最小链路：
  - 新建项目 -> 章节创建/保存/恢复 -> AI 生成预览 -> 一致性检查 -> 导出
  - Beta 补充：章节时间线加载 -> 关系图加载 -> DOCX/PDF/EPUB 导出
  - 发布链路补充：语义检索召回 -> Git 快照提交 -> 授权激活 -> 检查更新

## 9. 文档维护规则
以下变化必须同步更新本文档：
- 命令新增/删除
- 输入输出 DTO 变化
- fallback 策略变化
- 已知对齐差异修复或新增
