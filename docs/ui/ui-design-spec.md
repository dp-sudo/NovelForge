# NovelForge UI 设计文档（MVP）

## 1. 文档信息
- 版本：v0.7
- 状态：S19（编辑器 AI Pipeline + 结构化草案确认闭环 + 设置页 AI 策略入口）
- 最后更新：2026-04-30
- 代码基线：`src/pages/*`、`src/components/*`、`src/api/*`

## 2. UI 目标
- 覆盖当前主闭环页面与关键可见状态。
- 保证写作链路中的保存、恢复、AI 任务状态、人工确认入库可见。
- 明确页面数据源（当前默认 Tauri command）。

## 3. 页面信息架构（当前）
- 顶层：`Project Center` + `App Shell`
- `App Shell` 侧栏页面：
  - 仪表盘 `dashboard`
  - 蓝图 `blueprint`
  - 角色 `characters`
  - 世界设定 `world`
  - 名词库 `glossary`
  - 剧情 `plot`
  - 叙事义务 `narrative`
  - 时间线 `timeline`
  - 关系图 `relationships`
  - 章节 `chapters`
  - 一致性检查 `consistency`
  - 导出 `export`
  - 设置 `settings`

## 4. 页面与交互现状
### 4.1 项目中心（`ProjectCenterPage`）
- 新建项目、打开项目、最近项目、清空最近项目。
- 新建项目要求 Windows 绝对路径并做路径可用性校验。
- 数据源：`projectApi`。

### 4.2 章节页与编辑器（`ChaptersPage` + `EditorPage`）
- 章节能力：创建、删除、卷分配、顺序维护。
- 编辑器基础状态：
  - 保存状态：已保存/保存中/未保存/自动保存/失败。
  - 自动保存：5 秒 debounce。
  - 草稿恢复：检测到较新草稿时弹窗确认恢复。
- 编辑器 AI：
  - 固定 9 按钮任务栏（写作/角色/世界观/剧情/审稿分组）。
  - 自定义指令输入框（回车发送 `custom`）。
  - 主路径使用 `run_ai_task_pipeline` + `ai:pipeline:event`。
  - 支持任务取消、阶段错误提示与建议动作。
- AI 预览面板：插入/替换/追加/复制/丢弃。

### 4.3 编辑器右侧上下文面板
- 标签页：角色 / 设定 / 剧情 / 名词。
- 章节上下文：目标字数、当前字数、前章摘要。
- 新增资产候选区：
  - 显示候选标签、类型、置信度、证据、操作按钮。
  - 通过 `apply_asset_candidate` 采纳入库并建立章节关联。
- 新增结构化草案区：
  - `关系`（relationshipDrafts）
  - `戏份`（involvementDrafts）
  - `场景`（sceneDrafts）
  - 每条草案支持“确认入库”，通过 `apply_structured_draft` 落库并回写状态。

### 4.4 蓝图页（`BlueprintPage`）
- 8 步蓝图编辑、保存、完成标记、重置。
- 支持 AI 生成步骤建议并应用。
- 数据源：`blueprintApi`。

### 4.5 角色 / 世界 / 名词库 / 剧情页
- 角色页：角色 CRUD、关系 CRUD、AI 生成角色卡。
- 世界页：规则 CRUD、分类筛选、AI 生成设定。
- 名词库：术语列表与新增。
- 剧情页：节点 CRUD、排序、AI 生成节点。
- 数据源：对应 `characterApi/worldApi/glossaryApi/plotApi`。

### 4.6 一致性检查页（`ConsistencyPage`）
- 当前章/全书扫描与问题状态流转。
- 支持 AI 一致性扫描入口。
- 数据源：`consistencyApi`。

### 4.7 导出页（`ExportPage`）
- 导出范围、格式、路径、结果显示。
- 格式：TXT / Markdown / DOCX / PDF / EPUB。
- 数据源：`exportApi`（Tauri command）。

### 4.8 时间线页与关系图页
- 时间线页：章节顺序、卷信息、更新时间展示。
- 关系图页：角色关系可视化与聚焦。
- 数据源：`timelineApi`、`characterApi`。

### 4.9 设置页（`SettingsPage`）
- 标签页：
  - 模型配置
  - 任务路由
  - 技能管理
  - AI 策略
  - 编辑器
  - 写作风格
  - 数据与备份
  - 关于
- 模型配置：
  - Provider 卡片、API Key、探活、刷新模型、远端 registry 检查/应用。
- 任务路由：
  - 按 canonical 任务类型配置 Provider/Model/Fallback/重试次数。
  - Provider 保存后会自动补齐缺失任务路由（快速接入机制）。
- 技能管理：
  - 技能列表、搜索、分类筛选、导入 `.md`、编辑与保存。
  - 编辑区高度扩展（容器约 `70vh`，最小高度 `480px`）。
- 编辑器设置：
  - 字号、行高、自动保存间隔、默认叙事视角。
  - 通过 `load_editor_settings/save_editor_settings` 持久化。
- 写作风格：
  - 语言风格、描写密度、对话比例、句子节奏、氛围、心理描写深度。
  - 保存到项目级 `writing_style`。
- AI 策略：
  - 项目级 `AiStrategyProfile` 配置面板（工作流栈、审查严格度、能力包、自动持久化策略、连续性/生成模式）。
  - 通过 `get_ai_strategy_profile/save_ai_strategy_profile` 读写项目库 `projects.ai_strategy_profile`。
- 数据与备份：
  - 备份/恢复、完整性检查、Git 初始化与快照提交、历史查看。
- 关于：
  - 授权激活、授权状态、更新检查与安装。

## 5. 关键交互规范（已实现）
- AI 输出先进入预览区或流式结果区，用户手动决定写入方式。
- 保存与自动保存分离，正式保存后会刷新上下文与候选状态。
- 结构化抽取先草案后确认，不做静默自动落库。
- 关键动作提供进行中/成功/失败可见反馈。

## 6. 当前已知 UI 风险点
- legacy AI 入口仍存在，可能导致用户混淆“主链路（pipeline）”与“兼容链路（legacy）”。
- 结构化抽取为规则启发式，可能出现误报，需依赖人工确认按钮。
- 自动更新依赖发布端点与签名配置；配置异常时会提示安装失败。

## 7. MVP 非目标（保持不变）
- 云同步
- 多人协作
- 插件市场

## 8. 文档维护规则
以下变化必须同步更新本文档：
- 页面新增/删除。
- 关键交互变化（保存/恢复/AI 任务/确认入库/错误提示）。
- 页面数据源切换（命令名、事件协议、DTO 字段）。
