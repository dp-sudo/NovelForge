# NovelForge API 集成文档（Frontend <-> Tauri <-> Rust）

## 1. 文档信息
- 版本：v0.8
- 状态：生产就绪（AI Pipeline v1 + 结构化草案池闭环 + 完整命令体系）
- 最后更新：2026-05-03
- 代码基线：`src/api/*`、`src-tauri/src/commands/*`、`src-tauri/src/lib.rs`

## 2. 集成原则（当前）
- 前端统一通过 `invokeCommand` 调用 Rust command。
- API 描述以 `src-tauri/src/lib.rs` 注册命令为准。
- 任务路由统一使用 canonical task type（前后端同一映射）。
- 错误基线：统一 `AppErrorDto`。

## 2.1 唯一官方调用面（问题4修复）
- 官方前端调用面：`src/api/*`。
- 约束：页面层仅通过 `src/api/*` 调用 Tauri command。
- 标记为 compatibility-only 的命令（如 `load_provider_config`、`save_provider_config`、`register_ai_provider`、`test_ai_connection`）不作为新功能接入入口。

## 3. 前端 API 入口
- 统一调用：`src/api/tauriClient.ts`
- 业务模块：
  - `projectApi`, `chapterApi`, `blueprintApi`, `characterApi`, `worldApi`, `glossaryApi`, `plotApi`, `narrativeApi`
  - `consistencyApi`, `statsApi`, `settingsApi`, `skillsApi`, `contextApi`, `pipelineApi`, `moduleAiApi`, `exportApi`, `timelineApi`

## 4. Command 契约（按模块）
### 4.1 Project
- `validate_project(input: { name, forceError? }) -> { normalizedName, message }`
- `create_project(input: CreateProjectInput) -> ProjectOpenResult`
- `open_project(input: { projectRoot }) -> ProjectOpenResult`
- `list_recent_projects() -> RecentProjectItem[]`
- `clear_recent_projects() -> void`
- `save_writing_style(input: { projectRoot, writingStyle }) -> void`
- `get_writing_style(input: { projectRoot }) -> WritingStyle`
- `init_project_repository(projectRoot) -> GitRepositoryStatus`
- `get_project_repository_status(projectRoot) -> GitRepositoryStatus`
- `commit_project_snapshot(input: { projectRoot, message? }) -> GitSnapshotResult`
- `list_project_history(projectRoot, limit?) -> GitCommitRecord[]`

### 4.2 Chapter / Timeline / Snapshot / Volume / Import / Backup
- `list_chapters(projectRoot) -> ChapterRecord[]`
- `reorder_chapters(projectRoot, orderedIds: string[]) -> void`
- `create_chapter(input: { projectRoot, input: ChapterInput }) -> ChapterRecord`
- `save_chapter_content(input: { projectRoot, chapterId, content }) -> SaveChapterOutput`
- `autosave_draft(input: { projectRoot, chapterId, content }) -> string`
- `recover_draft(input: { projectRoot, chapterId }) -> RecoverDraftResult`
- 问题1修复：`read_chapter_content(projectRoot, chapterId) -> string`（切章与进入编辑器先加载正式正文）
- `delete_chapter(projectRoot, input: { id }) -> void`
- `list_timeline_entries(projectRoot) -> TimelineEntryRecord[]`
- `create_snapshot(projectRoot, chapterId, title?, note?) -> SnapshotRecord`
- `list_snapshots(projectRoot, chapterId?) -> SnapshotRecord[]`
- `read_snapshot_content(projectRoot, snapshotId) -> string`
- `list_volumes(projectRoot) -> VolumeRecord[]`
- `create_volume(projectRoot, input: { title, description? }) -> string`
- `delete_volume(projectRoot, id) -> void`
- `assign_chapter_volume(projectRoot, chapterId, volumeId?) -> void`
- `import_chapter_files(input: { projectRoot, files }) -> ImportResult`
- `create_backup(projectRoot) -> BackupResult`
- `list_backups(projectRoot) -> BackupResult[]`
- `restore_backup(projectRoot, backupPath) -> RestoreResult`

### 4.3 Blueprint / Character / World / Glossary / Plot / Narrative
- Blueprint：
  - `list_blueprint_steps(projectRoot) -> BlueprintStep[]`
  - `save_blueprint_step(projectRoot, input) -> BlueprintStep`
  - `mark_blueprint_completed(projectRoot, stepKey) -> void`
  - `reset_blueprint_step(projectRoot, stepKey) -> void`
- Character + Relationship：
  - `list_characters`, `create_character`, `update_character`, `delete_character`
  - `list_character_relationships`, `create_character_relationship`, `delete_character_relationship`
- World / Glossary / Plot：
  - `list_world_rules`, `create_world_rule`, `delete_world_rule`
  - `list_glossary_terms`, `create_glossary_term`
  - `list_plot_nodes`, `create_plot_node`, `reorder_plot_nodes`
- Narrative：
  - `list_narrative_obligations`
  - `create_narrative_obligation`
  - `update_obligation_status`
  - `delete_narrative_obligation`

### 4.4 Search / Integrity / Consistency / Dashboard / Export
- Search：
  - `search_project(projectRoot, query, limit?) -> SearchResult[]`（关键字 + 语义召回合并）
  - `search_project_semantic(projectRoot, query, limit?) -> VectorSearchResult[]`
  - `rebuild_search_index(projectRoot) -> number`
  - `rebuild_vector_index(projectRoot) -> number`
- Integrity：
  - `check_project_integrity(projectRoot) -> IntegrityReport`
- Consistency：
  - `scan_chapter_consistency(projectRoot, input: { chapterId }) -> ConsistencyIssue[]`
  - `list_consistency_issues(projectRoot) -> ConsistencyIssue[]`
  - `update_issue_status(projectRoot, issueId, status) -> void`
- Dashboard：
  - `get_dashboard_stats(projectRoot) -> DashboardStats`
- Export：
  - `export_chapter(input: { projectRoot, chapterId, format, outputPath, options? }) -> { outputPath }`
  - `export_book(input: { projectRoot, format, outputPath, options? }) -> { outputPath }`
  - `format`: `txt | md | docx | pdf | epub`

### 4.5 Settings / Model Registry / Task Routes
- Provider：
  - `list_providers`
  - `save_provider(config, apiKey?)`
  - `load_provider(providerId)`
  - `delete_provider(providerId)`
  - `test_provider_connection(providerId)`
- 模型：
  - `refresh_provider_models(providerId)`
  - `get_provider_models(providerId)`
  - `get_refresh_logs(providerId)`
- 路由：
  - `list_task_routes()`
  - `save_task_route(route)`
  - `delete_task_route(routeId)`
- Registry：
  - `check_remote_registry(url)`
  - `apply_registry_update(url)`
- 编辑器设置：
  - `load_editor_settings() -> EditorSettings`
  - `save_editor_settings(settings) -> void`
- 兼容命令：
  - 问题4修复：`load_provider_config`, `save_provider_config`（compatibility-only，已标注 deprecated）
- 授权：
  - `get_license_status() -> LicenseStatus`
  - `activate_license(licenseKey) -> LicenseStatus`
- 更新：
  - `check_app_update() -> AppUpdateInfo`
  - `install_app_update() -> AppUpdateInfo`
- 行为约束：
  - `save_provider` 后自动 reload 运行时 adapter。
  - 当 provider 有默认模型时，会为缺失任务自动补齐默认任务路由。
  - `list_task_routes` 会 canonical 化并做去重视图。

### 4.6 Skills
- `list_skills() -> SkillManifest[]`
- `get_skill(id) -> SkillManifest`
- `get_skill_content(id) -> string`
- `create_skill(input) -> SkillManifest`
- `update_skill(id, body) -> SkillManifest`
- `delete_skill(id) -> void`
- `import_skill_file(filePath) -> SkillManifest`
- `reset_builtin_skill(id) -> SkillManifest`
- `refresh_skills() -> SkillManifest[]`

### 4.7 AI / Pipeline / Context
- 问题3修复：legacy AI 命令 `generate_ai_preview`、`stream_ai_generate`、`stream_ai_chapter_task` 已从当前命令面移除。
- Pipeline：
  - `run_ai_task_pipeline(input) -> requestId`
  - `cancel_ai_task_pipeline(requestId) -> void`
- AI 功能任务：
  - `generate_blueprint_suggestion(input) -> string`
  - `ai_generate_character(input) -> string`
  - `ai_generate_world_rule(input) -> string`
  - `ai_generate_plot_node(input) -> string`
  - `ai_scan_consistency(input) -> string`
  - 问题4修复：`register_ai_provider(config) -> void`（compatibility-only，deprecated）
  - 问题4修复：`test_ai_connection(providerId) -> void`（compatibility-only，deprecated）
- Context：
  - `get_chapter_context(projectRoot, chapterId) -> ChapterContext`
  - `apply_asset_candidate(projectRoot, chapterId, input) -> ApplyAssetCandidateResult`
  - `apply_structured_draft(projectRoot, chapterId, input) -> ApplyStructuredDraftResult`

## 5. Pipeline 事件协议（`ai:pipeline:event`）
事件载荷（camelCase）：
- `requestId: string`
- `phase: "validate" | "context" | "route" | "prompt" | "generate" | "postprocess" | "persist" | "done" | string`
- `type: "start" | "progress" | "delta" | "done" | "error"`
- `delta?: string`
- `errorCode?: string`
- `message?: string`
- `recoverable?: boolean`
- `meta?: Record<string, unknown> | null`

## 6. 标准错误结构
```ts
interface AppErrorDto {
  code: string;
  message: string;
  detail?: string;
  recoverable: boolean;
  suggestedAction?: string;
}
```

## 7. 当前接口状态
- `src/api/*` 当前业务调用均为 invoke-only。
- 任务路由采用 canonical task type，前后端映射一致。
- 问题3修复：编辑器 AI 主路径已切换到 pipeline 事件流，legacy AI 命令不再开放。
- 问题4修复：compatibility-only 命令仅用于历史兼容，不作为官方接入路径。
- 结构化抽取结果默认仅入草案池，需显式确认命令才落核心资产表。

## 8. 最小回归链路
1. 新建项目 -> 新建章节 -> 保存正文 -> 恢复草稿。
2. 编辑器 9 按钮任一任务可触发 pipeline 并收到 done/error。
3. `get_chapter_context` 返回 `assetCandidates` 与三类 `*Drafts`。
4. `apply_asset_candidate` 与 `apply_structured_draft` 可成功入库。
5. 任务路由页面保存后，`list_task_routes` 回显 canonical 结果。

## 9. 文档维护规则
以下变化必须同步更新本文档：
- command 新增/删除。
- 输入输出 DTO 结构变化。
- pipeline 事件字段或阶段语义变化。
- 路由 canonical 规则变化。

## 10. Compatibility 命令收敛计划（问题4修复）
1. `2026-04-29` 起：`load_provider_config`、`save_provider_config`、`register_ai_provider`、`test_ai_connection` 标记为 compatibility-only，并在后端日志输出 deprecated 警告。
2. 新功能约束：页面/模块新增调用必须通过 `src/api/settingsApi.ts` 与 `src/api/pipelineApi.ts` 官方调用面。
3. 后续移除条件：当代码库内与外部适配层不再出现上述命令调用后，从 `src-tauri/src/lib.rs` `invoke_handler` 中移除。
