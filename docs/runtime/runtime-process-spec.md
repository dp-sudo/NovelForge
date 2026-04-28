# NovelForge 运行流程文档（Main / Renderer / API / Service）

## 1. 文档信息
- 版本：v0.6
- 状态：S18（AI Pipeline v1 已接入编辑器主链路）
- 最后更新：2026-04-28

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
  - `settingsApi`（Provider/路由/编辑器设置/写作风格/Git/授权/更新）
  - `skillsApi`（技能管理）

### 2.4 兼容层
- 保留 legacy AI 命令：`generate_ai_preview`、`stream_ai_generate`、`stream_ai_chapter_task`。
- `stream_ai_chapter_task` 现已桥接到 pipeline 事件，再转发到旧事件名。

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
3. 调用 `run_ai_task_pipeline` 得到 `requestId`。
4. 通过 `streamTaskPipelineByRequestId` 监听 `ai:pipeline:event`。
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
7. 用户可触发 `cancel_ai_task_pipeline` 取消进行中的任务。

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
4. 后端落库后回写草案状态（`applied` 或保留 pending）。

### 4.6 Provider/模型/任务路由
- 保存 Provider：
  - `save_provider` -> 校验配置 -> 存储 -> 运行时 reload adapter。
  - 若有默认模型，会自动补齐缺失的任务路由（快速接入主 Provider + 主模型 ID）。
- 路由读取：
  - `list_task_routes` 会 canonical 化并在空表时按主种子自动生成默认路由。
- 路由写入：
  - `save_task_route` 做 canonical、字段 trim、重试次数边界控制（1..8）。

### 4.7 设置、备份与发布能力
- 编辑器设置：`load_editor_settings/save_editor_settings`（app DB）。
- 写作风格：`get_writing_style/save_writing_style`（项目库 `projects.writing_style`）。
- 备份与完整性：`create_backup/list_backups/restore_backup/check_project_integrity`。
- Git 快照：`init_project_repository/get_project_repository_status/commit_project_snapshot/list_project_history`。
- 授权与更新：`get_license_status/activate_license/check_app_update/install_app_update`。

## 5. 失败处理策略
- 统一错误结构：`AppErrorDto`。
- 前端统一错误入口：`invokeCommand` 抛出的错误对象。
- Pipeline 专用：
  - 流式失败通过 `ai:pipeline:event(type=error)` 返回 `errorCode/message/recoverable`。
  - 前端根据 `errorCode/phase` 给出操作建议。
- legacy 兼容：
  - pipeline 错误会被编码后注入 legacy stream chunk，供兼容桥接解析。

## 6. 当前已知流程差异
- `generate_ai_preview` 仍带 mock fallback（用于兼容与兜底），不是推荐主链路。
- `stream_ai_generate` 为旧流式入口，仍保留但不用于编辑器 9 按钮主流程。
- 结构化抽取采用规则启发式，仍需人工确认，不自动直接写核心资产表。

## 7. 文档维护规则
以下变化必须同步更新本文档：
- command 注册增删。
- Pipeline 阶段、事件字段、错误码语义变化。
- 编辑器按钮任务映射与参数约束变化。
- 结构化草案“抽取 -> 确认 -> 落库”时序变化。
