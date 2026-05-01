# NovelForge API 集成文档（Frontend <-> Tauri <-> Rust）

## 1. 文档信息
- 版本：v1.0
- 状态：S21（全面文档维护：API 契约验证 + 命令注册完整性确认）
- 最后更新：2026-05-01
- 代码基线：`src/api/*`、`src-tauri/src/commands/*`

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

### 3.1 前端函数清单（按文件）
- `tauriClient.ts`：`invokeCommand`、`registerUnloadCleanup`、`logUI`
- `projectApi.ts`：`validateProjectName`、`createProject`、`openProject`、`listRecentProjects`、`clearRecentProjects`
- `chapterApi.ts`：`importChapterFiles`、`createBackup`、`listBackups`、`restoreBackup`、`searchProject`、`searchProjectSemantic`、`rebuildSearchIndex`、`rebuildVectorIndex`、`checkProjectIntegrity`、`listChapters`、`reorderChapters`、`createChapter`、`saveChapterContent`、`autosaveDraft`、`recoverDraft`、`readChapterContent`、`deleteChapter`、`createSnapshot`、`listSnapshots`、`readSnapshotContent`、`listVolumes`、`createVolume`、`deleteVolume`、`assignChapterVolume`
- `timelineApi.ts`：`listTimelineEntries`
- `blueprintApi.ts`：`listBlueprintSteps`、`saveBlueprintStep`、`markBlueprintCompleted`、`resetBlueprintStep`、`getWindowPlanningData`、`generateBlueprintSuggestion`
- `bookPipelineApi.ts`：`resolveChapterPlanChapterSelection`、`selectPromotionStage`、`buildBookStages`、`buildPromotionStages`、`streamBookGenerationPipeline`
- `characterApi.ts`：`listCharacters`、`createCharacter`、`updateCharacter`、`deleteCharacter`、`aiGenerateCharacter`、`listCharacterRelationships`、`createCharacterRelationship`、`deleteCharacterRelationship`、`getRelationshipGraphData`
- `worldApi.ts`：`listWorldRules`、`createWorldRule`、`deleteWorldRule`、`aiGenerateWorldRule`
- `glossaryApi.ts`：`listGlossaryTerms`、`createGlossaryTerm`、`aiGenerateGlossaryTerm`
- `plotApi.ts`：`listPlotNodes`、`createPlotNode`、`reorderPlotNodes`、`aiGeneratePlotNode`
- `narrativeApi.ts`：`listNarrativeObligations`、`createNarrativeObligation`、`updateObligationStatus`、`deleteNarrativeObligation`、`aiGenerateNarrativeObligation`
- `consistencyApi.ts`：`scanChapterConsistency`、`scanFullConsistency`、`listConsistencyIssues`、`updateIssueStatus`、`aiScanConsistency`
- `statsApi.ts`：`getDashboardStats`
- `contextApi.ts`：`getChapterContext`、`materializeChapterStructuredDrafts`、`applyAssetCandidate`、`applyStructuredDraft`、`summarizeStateDeltaForFeedback`、`getSummaryFeedback`
- `pipelineApi.ts`：`runTaskPipeline`、`cancelTaskPipeline`、`streamTaskPipeline`
- `moduleAiApi.ts`：`runModuleAiTask`
- `exportApi.ts`：`exportChapter`、`exportBook`
- `settingsApi.ts`：`listProviders`、`saveProvider`、`deleteProvider`、`testProviderConnection`、`refreshProviderModels`、`getProviderModels`、`getRefreshLogs`、`listTaskRoutes`、`saveTaskRoute`、`deleteTaskRoute`、`checkRemoteRegistry`、`applyRegistryUpdate`、`loadEditorSettings`、`saveEditorSettings`、`saveWritingStyle`、`getWritingStyle`、`saveAiStrategyProfile`、`getAiStrategyProfile`、`saveProjectAiStrategy`、`getProjectAiStrategy`、`initProjectRepository`、`getProjectRepositoryStatus`、`commitProjectSnapshot`、`listProjectHistory`、`getLicenseStatus`、`activateLicense`、`checkAppUpdate`、`installAppUpdate`、`getDeprecatedCommandUsageReport`
- `skillsApi.ts`：`listSkills`、`getSkill`、`getSkillContent`、`createSkill`、`updateSkill`、`deleteSkill`、`importSkillFile`、`resetBuiltinSkill`、`refreshSkills`

### 3.2 前端关键类型/接口清单（按文件）
- `projectApi.ts`：`ValidateProjectInput`、`ValidateProjectOutput`、`ProjectOpenResult`、`RecentProjectItem`
- `chapterApi.ts`：`ImportFileEntry`、`ImportedChapter`、`ImportResult`、`BackupResult`、`RestoreResult`、`SearchResult`、`IntegrityIssue`、`IntegritySummary`、`IntegrityReport`、`ChapterRecord`、`SaveChapterOutput`、`RecoverDraftResult`、`SnapshotRecord`、`VolumeRecord`
- `timelineApi.ts`：`TimelineEntry`
- `blueprintApi.ts`：`BlueprintStepRow`、`BlueprintSuggestionInput`、`WindowPlanningData`
- `bookPipelineApi.ts`：`BookStageKey`、`BookStage`、`RunBookGenerationInput`、`ChapterPlanChapterCandidate`、`ChapterPlanSelectionStrategy`、`ResolveChapterPlanChapterInput`、`ResolveChapterPlanChapterResult`、`BookPipelineEvent`
- `characterApi.ts`：`CharacterRow`、`CharacterRelationship`、`CreateRelationshipInput`、`RelationshipGraphData`
- `worldApi.ts`：`WorldRow`
- `glossaryApi.ts`：`GlossaryRow`
- `plotApi.ts`：`PlotRow`
- `narrativeApi.ts`：`NarrativeObligation`、`CreateNarrativeObligationInput`
- `consistencyApi.ts`：`ConsistencyIssueRow`、`AiConsistencyInput`
- `statsApi.ts`：`DashboardRecentChapter`、`DashboardStats`
- `contextApi.ts`：`ChapterContext`、`ApplyAssetCandidateInput`、`ApplyAssetCandidateResult`、`ApplyStructuredDraftInput`、`ApplyStructuredDraftResult`、`SummaryFeedbackData`
- `pipelineApi.ts`：`AiPipelinePhase`、`AiPipelineEventType`、`PersistMode`、`AutomationTier`、`RuntimeSkillSelectionInput`、`RunTaskPipelineInput`、`AiPipelineEvent`、`TaskPipelineStreamOptions`
- `moduleAiApi.ts`：`RunModuleAiTaskInput`
- `exportApi.ts`：`ExportOutput`、`ExportFormat`
- `settingsApi.ts`：`EditorSettingsData`、`GitRepositoryStatus`、`GitCommitRecord`、`GitSnapshotResult`、`LicenseStatus`、`AppUpdateInfo`、`DeprecatedCommandUsageEntry`
- `skillsApi.ts`：`SkillManifest`、`CreateSkillInput`、`SkillManifestPatch`、`UpdateSkillInput`

### 3.3 前后端对接矩阵（带注释）
- `projectApi.ts`：
  - `validateProjectName -> validate_project`（项目名规则校验）
  - `createProject -> create_project`（创建项目目录与数据库）
  - `openProject -> open_project`（打开并校验项目）
  - `listRecentProjects -> list_recent_projects`（读取最近项目）
  - `clearRecentProjects -> clear_recent_projects`（清空最近项目）
- `chapterApi.ts`（章节/快照/卷/备份/检索/完整性）：
  - `listChapters -> list_chapters`、`reorderChapters -> reorder_chapters`、`createChapter -> create_chapter`、`saveChapterContent -> save_chapter_content`、`autosaveDraft -> autosave_draft`、`recoverDraft -> recover_draft`、`readChapterContent -> read_chapter_content`、`deleteChapter -> delete_chapter`
  - `createSnapshot -> create_snapshot`、`listSnapshots -> list_snapshots`、`readSnapshotContent -> read_snapshot_content`
  - `listVolumes -> list_volumes`、`createVolume -> create_volume`、`deleteVolume -> delete_volume`、`assignChapterVolume -> assign_chapter_volume`
  - `importChapterFiles -> import_chapter_files`
  - `createBackup -> create_backup`、`listBackups -> list_backups`、`restoreBackup -> restore_backup`
  - `searchProject -> search_project`、`searchProjectSemantic -> search_project_semantic`、`rebuildSearchIndex -> rebuild_search_index`、`rebuildVectorIndex -> rebuild_vector_index`、`checkProjectIntegrity -> check_project_integrity`
- `blueprintApi.ts`：
  - `listBlueprintSteps -> list_blueprint_steps`、`saveBlueprintStep -> save_blueprint_step`、`markBlueprintCompleted -> mark_blueprint_completed`、`resetBlueprintStep -> reset_blueprint_step`
  - `getWindowPlanningData`（前端聚合函数：调用 `listBlueprintSteps` + `listChapters` 生成窗口规划数据）
  - `generateBlueprintSuggestion`（包装链路：`runModuleAiTask -> runTaskPipeline -> run_ai_task_pipeline`）
- `characterApi.ts`：
  - `listCharacters -> list_characters`、`createCharacter -> create_character`、`updateCharacter -> update_character`、`deleteCharacter -> delete_character`
  - `listCharacterRelationships -> list_character_relationships`、`createCharacterRelationship -> create_character_relationship`、`deleteCharacterRelationship -> delete_character_relationship`
  - `aiGenerateCharacter`（包装链路：`runTaskPipeline -> run_ai_task_pipeline`）
  - `getRelationshipGraphData`（前端聚合函数：组合角色与关系数据）
- `worldApi.ts`：`listWorldRules -> list_world_rules`、`createWorldRule -> create_world_rule`、`deleteWorldRule -> delete_world_rule`、`aiGenerateWorldRule`（包装链路）
- `glossaryApi.ts`：`listGlossaryTerms -> list_glossary_terms`、`createGlossaryTerm -> create_glossary_term`、`aiGenerateGlossaryTerm`（包装链路）
- `plotApi.ts`：`listPlotNodes -> list_plot_nodes`、`createPlotNode -> create_plot_node`、`reorderPlotNodes -> reorder_plot_nodes`、`aiGeneratePlotNode`（包装链路）
- `narrativeApi.ts`：`listNarrativeObligations -> list_narrative_obligations`、`createNarrativeObligation -> create_narrative_obligation`、`updateObligationStatus -> update_obligation_status`、`deleteNarrativeObligation -> delete_narrative_obligation`、`aiGenerateNarrativeObligation`（包装链路）
- `consistencyApi.ts`：`scanChapterConsistency -> scan_chapter_consistency`、`scanFullConsistency -> scan_full_consistency`、`listConsistencyIssues -> list_consistency_issues`、`updateIssueStatus -> update_issue_status`、`aiScanConsistency`（包装链路）
- `statsApi.ts`：`getDashboardStats -> get_dashboard_stats`
- `timelineApi.ts`：`listTimelineEntries -> list_timeline_entries`
- `contextApi.ts`：
  - `getChapterContext -> get_chapter_context`
  - `materializeChapterStructuredDrafts -> materialize_chapter_structured_drafts`
  - `applyAssetCandidate -> apply_asset_candidate`
  - `applyStructuredDraft -> apply_structured_draft`
  - `summarizeStateDeltaForFeedback`、`getSummaryFeedback`（前端本地反馈组装，不触发新增 command）
- `exportApi.ts`：`exportChapter -> export_chapter`、`exportBook -> export_book`
- `settingsApi.ts`：
  - Provider/模型：`listProviders -> list_providers`、`saveProvider -> save_provider`、`deleteProvider -> delete_provider`、`testProviderConnection -> test_provider_connection`、`refreshProviderModels -> refresh_provider_models`、`getProviderModels -> get_provider_models`、`getRefreshLogs -> get_refresh_logs`
  - 路由/注册表：`listTaskRoutes -> list_task_routes`、`saveTaskRoute -> save_task_route`、`deleteTaskRoute -> delete_task_route`、`checkRemoteRegistry -> check_remote_registry`、`applyRegistryUpdate -> apply_registry_update`
  - 编辑器/策略：`loadEditorSettings -> load_editor_settings`、`saveEditorSettings -> save_editor_settings`、`saveWritingStyle -> save_writing_style`、`getWritingStyle -> get_writing_style`、`saveAiStrategyProfile -> save_ai_strategy_profile`、`getAiStrategyProfile -> get_ai_strategy_profile`、`saveProjectAiStrategy -> save_ai_strategy_profile`、`getProjectAiStrategy -> get_ai_strategy_profile`
  - Git/授权/更新：`initProjectRepository -> init_project_repository`、`getProjectRepositoryStatus -> get_project_repository_status`、`commitProjectSnapshot -> commit_project_snapshot`、`listProjectHistory -> list_project_history`、`getLicenseStatus -> get_license_status`、`activateLicense -> activate_license`、`checkAppUpdate -> check_app_update`、`installAppUpdate -> install_app_update`
  - 兼容桥审计：`getDeprecatedCommandUsageReport -> get_deprecated_command_usage_report`
- `skillsApi.ts`：`listSkills -> list_skills`、`getSkill -> get_skill`、`getSkillContent -> get_skill_content`、`createSkill -> create_skill`、`updateSkill -> update_skill`、`deleteSkill -> delete_skill`、`importSkillFile -> import_skill_file`、`resetBuiltinSkill -> reset_builtin_skill`、`refreshSkills -> refresh_skills`
- `pipelineApi.ts`：
  - `runTaskPipeline -> run_ai_task_pipeline`
  - `cancelTaskPipeline -> cancel_ai_task_pipeline`
  - `streamTaskPipeline`（监听事件 `ai:pipeline:event`，内部使用 `runTaskPipeline/cancelTaskPipeline`）
  - `registerUnloadCleanup`（页面卸载时触发命令级取消，内部调用 `cancel_ai_task_pipeline`）
- `moduleAiApi.ts`：
  - `runModuleAiTask`（模块化统一入口，包装到 `runTaskPipeline`，再进入 `run_ai_task_pipeline`）

## 4. Command 契约（按模块）
### 4.1 Project
- `validate_project(input: { name, forceError? }) -> { normalizedName, message }`
- `create_project(input: CreateProjectInput) -> ProjectOpenResult`
- `open_project(input: { projectRoot }) -> ProjectOpenResult`
- `list_recent_projects() -> RecentProjectItem[]`
- `clear_recent_projects() -> void`
- `save_writing_style(input: { projectRoot, writingStyle }) -> void`
- `get_writing_style(input: { projectRoot }) -> WritingStyle`
- `save_ai_strategy_profile(input: { projectRoot, profile }) -> void`
- `get_ai_strategy_profile(input: { projectRoot }) -> AiStrategyProfile`
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
  - `BlueprintStep` 新增 `certaintyZones?: { frozen: string[]; promised: string[]; exploratory: string[] }`（当前用于 `step-08-chapters`）。
  - `save_blueprint_step.input` 新增可选 `certaintyZones`，作为确定性分区显式 DTO（优先于旧文本分区解析）。
  - 分区校验规则：同一条目不可跨分区重叠；冲突时返回 `BLUEPRINT_CERTAINTY_ZONES_OVERLAP`。
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
  - `scan_full_consistency(projectRoot) -> ConsistencyIssue[]`
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
- 项目级 AI 策略：
  - `save_ai_strategy_profile(input: { projectRoot, profile }) -> void`
  - `get_ai_strategy_profile(input: { projectRoot }) -> AiStrategyProfile`
- 兼容命令：
  - 问题4修复：`load_provider_config`, `save_provider_config`（compatibility-only，已标注 deprecated）
- 授权：
  - `get_license_status() -> LicenseStatus`
  - `activate_license(licenseKey) -> LicenseStatus`
- 更新：
  - `check_app_update() -> AppUpdateInfo`
  - `install_app_update() -> AppUpdateInfo`
- 兼容桥审计：
  - `get_deprecated_command_usage_report() -> DeprecatedCommandUsageEntry[]`
- 行为约束：
  - `save_provider` 后自动 reload 运行时 adapter。
  - 问题4修复：默认任务路由只在 app DB 初始化阶段补齐（单一入口）。
  - 问题4修复：`list_task_routes` 为纯读接口，会 canonical 化并做去重视图。
  - compatibility-only 命令调用会记录 deprecated usage，并打 `compatibility_bridge.used` 行为日志。

### 4.6 Skills
- `list_skills() -> SkillManifest[]`
- `get_skill(id) -> SkillManifest`
- `get_skill_content(id) -> string`
- `create_skill(input) -> SkillManifest`
- `update_skill(input: { id, body?, manifest? }) -> SkillManifest`
  - `manifest` 为可选 patch，支持更新 `skillClass/bundleIds/alwaysOn/triggerConditions/requiredContexts/stateWrites/automationTier/sceneTags/affectsLayers` 等元数据
- `delete_skill(id) -> void`
- `import_skill_file(filePath) -> SkillManifest`
- `reset_builtin_skill(id) -> SkillManifest`
- `refresh_skills() -> SkillManifest[]`

### 4.7 AI / Pipeline / Context
- 问题3修复：legacy AI 命令 `generate_ai_preview`、`stream_ai_generate`、`stream_ai_chapter_task` 已从当前命令面移除。
- Pipeline：
  - `run_ai_task_pipeline(input) -> requestId`
  - `cancel_ai_task_pipeline(requestId) -> void`
  - 前端流式入口：`streamTaskPipeline(input, options)`（监听 `ai:pipeline:event`）。
  - `run_ai_task_pipeline.input` 持久化字段：
    - `autoPersist?: boolean`（兼容桥，保留）
    - `persistMode?: "none" | "formal" | "derived_review"`（显式持久化语义）
    - `automationTier?: "auto" | "supervised" | "confirm"`（显式自动化档位）
    - `skillSelection?: { explicitSkillIds, activeBundleIds, sceneTags, availableContexts, disableInferredSceneTags }`（请求级技能编排覆盖）
  - 兼容规则：
    - 当 `persistMode` 存在时，以 `persistMode` 语义为准（覆盖 `autoPersist`）。
    - 当仅有 `autoPersist: true` 时，前端按任务类型推断 `persistMode`，默认 `automationTier = "supervised"`。
    - 当仅走 `autoPersist` 推导路径时，前端会记录 `PIPELINE.LEGACY_POLICY_BRIDGE` 诊断日志。
  - 运行时行为：
    - `prompt` 前会编译 `ContinuityPack`，并注入技能选择结果。
    - 技能选择不再只看 `taskType`；还会同时应用项目级 `alwaysOnPolicySkills/defaultCapabilityBundles`，以及技能元数据 `sceneTags/requiredContexts/automationTier`。
    - 若技能声明 `affectsLayers`，`orchestrator` 会按聚合后的 layer focus 对 `ContinuityPack` 进行裁剪（保留 constitution/lexicon 护栏层）。
    - 若技能命中 `route_override`，仅覆盖本次请求的 provider/model，不修改项目配置。
    - 若激活技能声明 `stateWrites`，后端会按项目级 `stateWritePolicy` 追加 `story_state` 记录，并写入 `skillIds/affectsLayers` 运行态元数据。
    - 若检测到用户指令改写冻结区条目，后端返回 `PIPELINE_FREEZE_CONFLICT` 并阻断执行。
- AI 功能任务（前端薄封装，统一走 pipeline）：
  - `generateBlueprintSuggestion` -> `runModuleAiTask(taskType="blueprint.generate_step")`
  - `aiGenerateCharacter` -> `runModuleAiTask(taskType="character.create")`
  - `aiGenerateWorldRule` -> `runModuleAiTask(taskType="world.create_rule")`
  - `aiGeneratePlotNode` -> `runModuleAiTask(taskType="plot.create_node")`
  - `aiGenerateGlossaryTerm` -> `runModuleAiTask(taskType="glossary.create_term")`
  - `aiGenerateNarrativeObligation` -> `runModuleAiTask(taskType="narrative.create_obligation")`
  - `aiScanConsistency` -> `runModuleAiTask(taskType="consistency.scan")`
  - compatibility-only：`register_ai_provider(config)`、`test_ai_connection(providerId)`（deprecated）
- Context：
  - `get_chapter_context(projectRoot, chapterId) -> ChapterContext`
  - `materialize_chapter_structured_drafts(projectRoot, chapterId) -> ChapterContext`
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
- 问题2修复：模块化 AI 命令（`ai_generate_*`、`ai_scan_consistency`、`generate_blueprint_suggestion`）已从 Rust command 面移除。
- 问题4修复：compatibility-only 命令仅用于历史兼容，不作为官方接入路径。
- 结构化抽取结果默认仅入草案池，需显式确认命令才落核心资产表。
- 项目级 AI 策略运行期真相源为 `project.sqlite.projects.ai_strategy_profile`（不走 `project.json` 日常双写）。

## 8. 最小回归链路
1. 新建项目 -> 新建章节 -> 保存正文 -> 恢复草稿。
2. 编辑器 9 按钮任一任务可触发 pipeline 并收到 done/error。
3. `get_chapter_context` 返回 `assetCandidates` 与三类 `*Drafts`。
4. `apply_asset_candidate` 与 `apply_structured_draft` 可成功入库。
5. 任务路由页面保存后，`list_task_routes` 回显 canonical 结果。
6. `save_ai_strategy_profile/get_ai_strategy_profile` 可在 `project.sqlite.projects.ai_strategy_profile` 往返读写。

## 9. 文档维护规则
以下变化必须同步更新本文档：
- command 新增/删除。
- 输入输出 DTO 结构变化。
- pipeline 事件字段或阶段语义变化。
- 路由 canonical 规则变化。

## 10. Compatibility 命令收敛计划（问题4修复）
1. `2026-04-29` 起：`load_provider_config`、`save_provider_config`、`register_ai_provider`、`test_ai_connection` 标记为 compatibility-only，并在后端日志输出 deprecated 警告与 `compatibility_bridge.used` 审计事件。
2. 新功能约束：页面/模块新增调用必须通过 `src/api/settingsApi.ts` 与 `src/api/pipelineApi.ts` 官方调用面。
3. 计划移除日期：`2026-07-31`（若无外部兼容阻塞，届时从 `src-tauri/src/lib.rs` `invoke_handler` 下线）。
