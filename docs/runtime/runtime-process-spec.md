# NovelForge 运行流程文档（Main / Renderer / API / Service）

## 1. 文档信息
- 版本：v1.2
- 状态：S24（阶段四：反馈事件生命周期 + 路由推荐 + 场景分类离线评估）
- 最后更新：2026-05-02

## 2. 运行时角色
### 2.1 Main（Tauri + Rust）
- 启动应用并注册 `invoke_handler`。
- 托管 `AppState`。
- 执行本地文件系统、SQLite、Provider/模型、AI Pipeline、技能注册表逻辑。

### 2.2 Renderer（React）
- 页面渲染与用户交互。
- 通过 `src/api/*` 调用 `invokeCommand`。
- 编辑器 AI 主链路走 `pipelineApi`，并监听 `ai:pipeline:event`。

### 2.3 API 适配层（`src/api/*`）
- `tauriClient.invokeCommand()` 统一调用与错误解析。
- 关键链路：
  - `projectApi` / `chapterApi` / `contextApi`
  - `pipelineApi`（run/cancel/stream）
  - `settingsApi`（Provider/路由/编辑器设置/写作风格/项目级 AI 策略/Git/授权/更新）
  - `skillsApi`（技能管理）

### 2.4 兼容层
- 问题3修复：legacy AI 命令 `generate_ai_preview`、`stream_ai_generate`、`stream_ai_chapter_task` 已从当前代码命令面移除。
- 问题4修复：仅保留少量 compatibility-only 命令（如 `load_provider_config`、`save_provider_config`、`register_ai_provider`、`test_ai_connection`），用于历史调用兼容。
- compatibility-only 命令在后端会记录两类审计：deprecated usage 计数 + `compatibility_bridge.used` 行为日志。

## 3. 启动流程
1. `main.rs` 调用 `app_lib::run()`。
2. `lib.rs` 注册 command、初始化 `AppState` 与技能注册表。
3. 前端进入 `project-center`；打开项目后进入 `AppShell`。

## 4. 主流程时序（当前实现）
### 4.1 项目创建/打开
1. `ProjectCenterPage` 调用 `create_project/open_project/list_recent_projects`。
2. `ProjectService` 创建目录、初始化数据库、写 `project.json`、写 `projects` 记录。
3. 首次打开项目会触发自动备份尝试（best-effort）。
4. 最近项目列表会清理失效路径并回写缓存。

### 4.2 章节写作与保存
1. `create_chapter` 创建章节。
2. 编辑器内容变化后进入 `unsaved`。
3. 5 秒防抖触发 `autosave_draft`。
4. 用户保存触发 `save_chapter_content`，并刷新上下文。
5. 切换章节时先 `recover_draft`，若草稿更新则弹窗确认。
6. 删除章节触发 `delete_chapter`（软删除 + 重排索引）。

### 4.3 编辑器 AI（Pipeline 主链路）
1. 点击 9 个固定按钮之一或输入自定义指令。
2. `EditorPage` 先做前置校验（章节/选区/指令/正文要求）。
3. 调用 `run_ai_task_pipeline` 得到 `requestId`（输入支持 `autoPersist` + `persistMode` + `automationTier` + 可选 `skillSelection` 请求级覆盖）。
   - 新调用约束：优先显式传 `persistMode/automationTier`；`autoPersist` 仅保留兼容推导路径，命中时前端记录 `PIPELINE.LEGACY_POLICY_BRIDGE` 诊断事件。
4. 通过 `streamTaskPipeline` 监听 `ai:pipeline:event`。
5. 事件按阶段推进：
   - `validate`
   - `context`
   - `route`
   - `prompt`
   - `generate`（delta 文本流）
   - `postprocess`
   - `persist`
   - `done`
6. 若报错，前端按 `errorCode + phase` 映射建议动作并展示。
   - 命中冻结区改写冲突时，后端返回 `PIPELINE_FREEZE_CONFLICT` 并阻断执行（不再仅做持久化降级）。
7. 用户可触发 `cancel_ai_task_pipeline` 取消进行中的任务。
8. 章节任务在 `route` 阶段会解析并记录池级决策（`modelPoolId` / `fallbackModelPoolId` / resolved provider-model）。
9. 章节任务在 `done` 前会执行“场景判别 + 后置任务链”，并将结果写入 `ai_pipeline_runs.post_task_results` 与 `ai_pipeline_runs.meta_json`。

#### 4.3.1 章节链路（Task 10 对齐）
1. 编译 `Continuity Pack`（Constitution/Canon/Lexicon Policy/State/Promise/Window/Recent）。
   - 章节关键任务（`chapter.draft` / `chapter.continue` / `chapter.rewrite` / `prose.naturalize`）会强制最小深度为 `deep`（即使项目策略配置为 `standard/minimal`）。
   - 若编排后缺失必需上下文层，后端会在 `ai:pipeline:event` 发出 `type="warning"`、`errorCode="PIPELINE_CONTEXT_INCOMPLETE"`，并在 `meta.warningCode = "context_incomplete"` 中返回缺失层清单。
2. 装配技能栈（workflow/capability/extractor/policy/review），运行时会同时应用：
   - 项目级 `alwaysOnPolicySkills`
   - 项目级 `defaultCapabilityBundles`
   - 请求级 `skillSelection`（显式 skill/bundle/scene/context + 可关闭推断场景标签）
   - 技能元数据 `sceneTags/requiredContexts/automationTier`
   - 可选 `route override`
3. 生成章节计划/草稿/改写等任务输出。
4. 场景感知编排：`SceneClassifier` 基于语义特征（对话占比、动作密度、战斗信号、信息密度）判别 `dialogue/action/exposition/introspection/combat`，并按路由 `postTasks` + 默认映射合并后置任务。
5. 后置任务执行：`PostTaskExecutor` 执行 `review_continuity` / `extract_state` / `extract_assets`，后置失败不阻断主任务，结果按任务项记录 `ok/error`。
6. 写后回写 `Canon + State`：正式资产入库并写 `entity_provenance`，章节保存后回写 `story_state`；若激活技能声明了 `stateWrites`，会按项目级 `stateWritePolicy` 追加运行时状态写入（含 `character.action/appearance/knowledge` 与 `scene.danger_level/spatial_constraint`），并附带 `skillIds/affectsLayers` 元数据。

### 4.4 编辑器 9 按钮任务映射（canonical）
- `chapter.continue`（续写章节）
- `chapter.draft`（生成章节草稿）
- `chapter.plan`（生成章节计划）
- `chapter.rewrite`（改写选区）
- `prose.naturalize`（去 AI 味）
- `character.create`（创建角色卡）
- `world.create_rule`（创建世界规则）
- `plot.create_node`（创建剧情节点）
- `consistency.scan`（一致性扫描）

### 4.5 上下文抽取与人工确认入库
1. `get_chapter_context` 返回：
   - 章节上下文（角色/设定/剧情/术语/蓝图）
   - `assetCandidates`
   - `relationshipDrafts` / `involvementDrafts` / `sceneDrafts`
2. 抽取结果先写入 `structured_draft_*`（pending）。
3. 用户点“确认入库”后调用：
   - `apply_asset_candidate`
   - `apply_structured_draft`
   - `reject_structured_draft`（忽略草案）
4. 后端落库后回写草案状态：`pending -> applied | rejected`；批次内无 `pending` 时，`structured_draft_batches.status = completed`。

### 4.6 Provider/模型/任务路由
- 保存 Provider：
  - `save_provider` -> 校验配置 -> 存储 -> 运行时 reload adapter。
- 路由初始化：
  - 问题4修复：默认任务路由仅在 app DB 初始化阶段补齐（单一入口）。
- 路由读取：
  - 问题4修复：`list_task_routes` 为纯读接口，仅返回 canonical 去重视图。
- 路由写入：
  - `save_task_route` 做 canonical、字段 trim、重试次数边界控制（1..8）。
  - 路由对象新增可选字段：`modelPoolId` / `fallbackModelPoolId` / `postTasks`（兼容旧 provider/model 路由）。
- 路由策略推荐：
  - `recommend_routing_strategy`：按项目阶段与任务风险返回模板列表。
  - `apply_routing_strategy_template`：批量应用模板映射并持久化 `projects.routing_strategy_id`。
  - `get_project_routing_strategy`：读取项目当前已选策略 ID，用于重开项目自动回放策略。

### 4.9 回报事件生命周期
1. `FeedbackService` 在角色/关系/章节保存链路触发检测并写入 `feedback_events(status=open)`。
2. Dashboard 可执行 `acknowledge_feedback_event` / `resolve_feedback_event` / `ignore_feedback_event`。
3. 状态流转约束：`open -> acknowledged/resolved/ignored`，`acknowledged -> resolved/ignored`，终态不可再次迁移。
4. `resolve/ignore` 记录 `resolved_at`、`resolved_by`、`resolution_note`，并在高优先规则写入闭环动作备注。

### 4.10 场景分类离线评估
1. 基准集：`tests/fixtures/scene_classification_benchmark.json`（5 类场景，50 条样本）。
2. 回归评估：`tests/scene_classification_evaluation.test.ts` 输出准确率、混淆矩阵、各类召回率。
3. 阈值：准确率 `>= 0.85`，且相对基线回退不超过 `5%`（`tests/fixtures/scene_classification_baseline.json`）。
4. CI：`.github/workflows/scene-classification-regression.yml` 在 PR 中自动运行该评估。

### 4.7 设置、备份与发布能力
- Settings 前端已拆分：
  - `ModelPoolPanel`（模型池增删改查、池内模型配置、fallback 池配置）。
  - `ModelRoutingPanel`（任务路由配置）。
  - `DataOpsPanel`（备份恢复、完整性、语义诊断、Git 快照）。
- 编辑器设置：`load_editor_settings/save_editor_settings`（app DB）。
- 写作风格：`get_writing_style/save_writing_style`（项目库 `projects.writing_style`）。
- 项目级 AI 策略：`get_ai_strategy_profile/save_ai_strategy_profile`（项目库 `projects.ai_strategy_profile`）。
- 备份与完整性：`create_backup/list_backups/restore_backup/check_project_integrity`。
- Git 快照：`init_project_repository/get_project_repository_status/commit_project_snapshot/list_project_history`。
- 授权与更新：`get_license_status/activate_license/check_app_update/install_app_update`。

### 4.8 蓝图与晋升边界
- 蓝图页“一键全书生成”默认只执行 8 步 `blueprint.generate_step`（蓝图阶段）。
- `step-08-chapters` 的“确定性分区”采用显式 DTO 持久化（`certaintyZones`），运行时编排优先读取 DTO；若旧数据缺失 DTO，再回退文本分区解析。
- 资产晋升通过“确认并晋升”独立触发（角色/世界/剧情/术语/叙事/章节计划）。
- `chapter-plan` 章节选择策略（仅晋升入口）：
  1. 用户显式选择章节；
  2. 当前编辑器活动章节；
  3. 章节列表中首个可规划章节（优先未完成）；
  4. 若仍为空则提示“请选择章节以生成章节计划”，并跳过该晋升步骤。

## 5. 失败处理策略
- 统一错误结构：`AppErrorDto`。
- 前端统一错误入口：`invokeCommand` 抛出的错误对象。
- Pipeline 专用：
  - 流式失败通过 `ai:pipeline:event(type=error)` 返回 `errorCode/message/recoverable`。
  - 前端根据 `errorCode/phase` 给出操作建议。
- legacy 兼容：
  - pipeline 错误会被编码后注入 legacy stream chunk，供兼容桥接解析。

## 6. 当前已知流程差异
- 问题3修复：编辑器 AI 主链路为 `run_ai_task_pipeline + ai:pipeline:event`；旧流式命令不再是可用接口。
- 结构化抽取采用规则启发式，仍需人工确认，不自动直接写核心资产表。
- 项目级 AI 策略运行期真相源为项目库，不在 `project.json` 日常保存路径双写。
- 手动创建资产（character/world/plot/glossary/narrative）会落 `entity_provenance(source_kind=user_input)`，用于来源追溯。

## 7. 文档维护规则
以下变化必须同步更新本文档：
- command 注册增删。
- Pipeline 阶段、事件字段、错误码语义变化。
- 编辑器按钮任务映射与参数约束变化。
- 结构化草案“抽取 -> 确认 -> 落库”时序变化。
