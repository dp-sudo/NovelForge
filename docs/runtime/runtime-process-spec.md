# NovelForge 运行流程文档（Main / Renderer / API / Service）

## 1. 文档信息
- 版本：v0.5
- 状态：S17 发布能力已接入（语义检索 / Git / 授权 / 自动更新）
- 最后更新：2026-04-27

## 2. 运行时角色
### 2.1 Main（Tauri + Rust）
- 启动应用并注册 `invoke_handler`
- 托管 `AppState`
- 执行本地文件系统、SQLite、Provider/模型相关逻辑

### 2.2 Renderer（React）
- 页面渲染与用户交互
- 通过 `src/api/*` 调用 `invokeCommand`
- 主闭环页面不再依赖隐式 DevEngine 回退

### 2.3 API 适配层（`src/api/*`）
- `tauriClient.invokeCommand()` 统一调用与错误对象转换
- 主闭环 API 直接透传 `projectRoot` 调用 command
- `contextApi` 已接入 Rust command（`get_chapter_context`）
- 时间线视图通过 `timelineApi` 调用 `list_timeline_entries`
- 设置页发布能力调用：Git 快照 / 授权 / 更新检查均通过 Tauri command

### 2.4 DevEngine（`src/api/dev-engine.ts`）
- localStorage 模拟实现
- 主要用于保留兼容模块（如 settings/export）与测试场景

## 3. 启动与路由流程
1. `main.rs` 调用 `app_lib::run()`。
2. `lib.rs` 注册 command 与 `AppState::default()`。
3. 前端进入 `project-center`；若未进入项目，则只渲染项目中心。
4. 进入项目后切换到 `AppShell` + 各业务页面。

## 4. 主流程时序（当前实现）
### 4.1 项目创建/打开
- 代码可用路径：`projectApi` -> `create_project/open_project/list_recent_projects/clear_recent_projects`
- 当前 UI 主路径：`ProjectCenterPage` 调用 `projectApi`，并要求输入有效 Windows 保存目录
- Rust `ProjectService` 行为：创建目录、初始化项目库、写 `project.json`、记录最近项目
- 最近项目列表读取时会自动剔除无效项目路径，并回写最近项目缓存文件

### 4.2 章节写作与草稿恢复
1. `ChaptersPage` 创建章节（`create_chapter`）。
2. `EditorPage` 输入正文，状态变为 `unsaved`。
3. 5 秒 debounce 后调用 `autosave_draft`。
4. Ctrl+S/保存按钮调用 `save_chapter_content`。
5. 打开章节时调用 `recover_draft`，若草稿更新则弹恢复提示。
6. 删除章节调用 `delete_chapter`，后端执行软删除并重排章节索引。

### 4.3 AI 章节流式生成
1. `EditorPage` 调用 `streamAiChapterTask({ projectRoot, chapterId, taskType, userInstruction })`。
2. Rust command `stream_ai_chapter_task`：
   - 收集上下文（`ContextService`）
   - 构建 Prompt（`PromptBuilder`）
   - 调用 `AiService.stream_generate()`
   - 发送 `ai:stream-chunk:{requestId}` 和 `ai:stream-done:{requestId}`
   - `AiService.stream_generate()` 会优先按 `task_type -> llm_task_routes` 解析 provider/model，并在需要时从应用级配置库懒加载 Provider 适配器
3. 前端 `createEventStream()` 监听事件并以 async generator 向编辑器逐段回传。
4. 编辑器上下文侧栏通过 `get_chapter_context(projectRoot, chapterId)` 获取 Rust 聚合数据。
5. 上下文 payload 已包含 `assetCandidates`（章节文本资产抽取候选），由编辑器右侧面板渲染。

### 4.4 Provider / 模型 / 任务路由
- 设置页模型标签调用：
  - `list_providers`, `save_provider`, `delete_provider`, `test_provider_connection`
  - `refresh_provider_models`, `get_provider_models`, `get_refresh_logs`
- 任务路由与 registry command 已注册：
  - `list_task_routes`, `save_task_route`, `delete_task_route`
  - `check_remote_registry`, `apply_registry_update`
- 当前 UI：任务路由标签已连通 CRUD（Provider/Model/Fallback/重试次数），可保存并回显
- `save_provider` 保存后会触发运行时适配器重载；`test_provider_connection` 为真实探活
- registry 检查/应用增加 URL 安全约束 + Schema/签名字段校验，校验失败不落库覆盖模型

### 4.5 导出流程
- Rust 可用路径：`export_chapter` / `export_book`
- 导出格式：`txt/md/docx/pdf/epub`
- 相对 `outputPath` 按 `projectRoot` 解析
- 当前页面主路径：`ExportPage` 调用 `exportApi`（失败时回退 DevEngine）

### 4.6 Beta 页面流程（S16）
- 时间线页：`TimelinePage` -> `list_timeline_entries`，按 `chapter_index` 排序浏览
- 关系图页：`RelationshipsPage` -> `characterApi`（角色 + 关系），用于可视化与跳转到角色页

### 4.7 发布能力流程（S17）
- 语义检索：
  - `search_project` 在 FTS 结果后叠加 `VectorService` 语义召回
  - `rebuild_search_index` 会同时重建 FTS 与向量索引
- Git 快照：
  1. 设置页触发 `init_project_repository` 初始化仓库
  2. 触发 `commit_project_snapshot` 执行 `git add -A` + `git commit`
  3. 调用 `list_project_history` 查看最近提交
- 授权：
  - 设置页调用 `activate_license`，本地写入 `~/.novelforge/license.json`
  - 启动后可通过 `get_license_status` 离线读取授权态
- 自动更新：
  - `check_app_update` 通过 `tauri-plugin-updater` 检查更新
  - `install_app_update` 下载并安装更新包（安装后需重启应用）

## 5. 命令清单（按模块）
- Project：`validate_project`, `create_project`, `open_project`, `list_recent_projects`, `clear_recent_projects`, `init_project_repository`, `get_project_repository_status`, `commit_project_snapshot`, `list_project_history`
- Chapter：`list_chapters`, `list_timeline_entries`, `create_chapter`, `save_chapter_content`, `autosave_draft`, `recover_draft`, `delete_chapter`
- Search：`search_project`, `search_project_semantic`, `rebuild_search_index`, `rebuild_vector_index`
- Context：`get_chapter_context`
- Blueprint：`list_blueprint_steps`, `save_blueprint_step`, `mark_blueprint_completed`, `reset_blueprint_step`
- Character：`list_characters`, `create_character`, `update_character`, `delete_character`, `list_character_relationships`, `create_character_relationship`, `delete_character_relationship`
- World/Glossary/Plot：`list_world_rules`, `create_world_rule`, `delete_world_rule`, `list_glossary_terms`, `create_glossary_term`, `list_plot_nodes`, `create_plot_node`, `reorder_plot_nodes`
- Consistency/Dashboard：`scan_chapter_consistency`, `list_consistency_issues`, `update_issue_status`, `get_dashboard_stats`
- Export：`export_chapter`, `export_book`
- Settings：`get_license_status`, `activate_license`, `check_app_update`, `install_app_update`, `list_providers`, `save_provider`, `load_provider`, `delete_provider`, `test_provider_connection`, `refresh_provider_models`, `get_provider_models`, `get_refresh_logs`, `list_task_routes`, `save_task_route`, `delete_task_route`, `check_remote_registry`, `apply_registry_update`, `load_provider_config`, `save_provider_config`
- AI：`generate_ai_preview`, `stream_ai_generate`, `stream_ai_chapter_task`, `ai_scan_consistency`, `register_ai_provider`, `test_ai_connection`, `list_skills`, `generate_blueprint_suggestion`, `ai_generate_character`

## 6. 失败处理与降级策略
- Rust 侧统一错误结构：`AppErrorDto`
- 前端 `invokeCommand` 捕获异常后转成统一错误对象
- 主闭环 API 不再 silent fallback，调用失败直接向上抛错并由页面处理
- 仍保留兼容 fallback 的模块：`exportApi`
- `settingsApi` 仅编辑器设置子模块仍使用 localStorage

## 7. 当前已知流程差异
- 导出链路仍在兼容期，保留 fallback 分支。
- 编辑器设置仍未迁移到 Tauri 配置存储。
- 自动更新依赖发布端点与签名，当前配置错误会导致“可检查但不可安装”。
- `npm run tauri:dev` 的端到端手工验收仍需持续覆盖项目创建/章节编辑/上下文/时间线/关系图/导出链路。

## 8. 文档维护规则
以下变更必须同步更新本文档：
- 命令注册增删
- 主流程时序变化
- fallback 策略变化
- AI 流式事件协议变化
