# NovelForge UI 设计文档（MVP）

## 1. 文档信息
- 版本：v0.5
- 状态：S17 发布能力 UI 已接入设置页（Git/授权/更新）
- 最后更新：2026-04-27
- 代码基线：`src/pages/*`、`src/components/*`、`src/api/*`

## 2. UI 目标
- 覆盖 MVP 主闭环核心页面
- 保证写作链路中保存状态、草稿恢复、AI 预览插入可见
- 明确标注页面数据源（Tauri command 为主，兼容 fallback 为辅）

## 3. 页面信息架构（当前）
- 顶层：`Project Center` 与 `App Shell`
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
- 交互：新建项目、打开本地项目、最近项目
- 新建项目新增保存目录输入与 Windows 路径校验
- 当前数据源：`projectApi`（纯 Tauri command）

### 4.2 章节页与编辑器（`ChaptersPage` + `EditorPage`）
- 章节页：新建、进入编辑器、删除
- 编辑器：
  - 保存状态徽标（已保存/保存中/未保存/自动保存/失败）
  - 自动保存 debounce（5 秒）
  - 草稿恢复提示（恢复/忽略）
  - AI 指令栏 + AI 预览面板（插入/替换/追加/复制）
  - 右侧上下文新增“资产候选”区块（章节文本抽取候选标签、类型、置信度、证据片段）
- 当前数据源：`chapterApi`（纯 Tauri command）；上下文侧栏来自 `contextApi` -> `get_chapter_context`（Rust）

### 4.3 蓝图页（`BlueprintPage`）
- 8 步导航、步骤保存、标记完成、重置
- AI 建议生成并可“采用/忽略”
- 当前数据源：`blueprintApi`（纯 Tauri command）

### 4.4 角色页（`CharactersPage`）
- 角色 CRUD、角色关系 CRUD、AI 生成角色卡
- 当前数据源：`characterApi`（纯 Tauri command）

### 4.5 世界设定 / 名词库 / 剧情页
- 世界设定：分类筛选、详情查看、创建、删除
- 名词库：术语列表、锁定/禁用标记、新增
- 剧情：节点列表、排序调整、新增
- 当前数据源：各自 API 均为纯 Tauri command

### 4.6 一致性检查页（`ConsistencyPage`）
- 全书/当前章切换扫描
- 问题列表、详情面板、状态流转（open/fixed/ignored/false_positive）
- 当前数据源：`consistencyApi`（纯 Tauri command）

### 4.7 导出页（`ExportPage`）
- 导出范围（单章/全书）、章节选择、格式、选项、导出结果卡片
- 支持格式：TXT / Markdown / DOCX / PDF / EPUB
- 当前数据源：`exportApi`（Tauri-first + DevEngine fallback）

### 4.8 时间线页（`TimelinePage`）
- 以章节为粒度展示时间线节点
- 支持正序/倒序切换，展示卷信息、摘要、更新时间
- 当前数据源：`timelineApi`（Tauri command：`list_timeline_entries`）

### 4.9 关系图页（`RelationshipsPage`）
- 角色关系可视化（节点 + 连线）
- 支持按角色聚焦关系明细，并可跳转到角色页面维护关系
- 当前数据源：`characterApi`（角色列表 + 关系列表）

### 4.10 设置页（`SettingsPage`）
- 标签页：模型配置 / 任务路由 / 编辑器 / 数据与备份 / 关于
- 已实现交互：
  - 模型配置：Provider 卡片、API Key、测试连接、刷新模型、保存
  - 任务路由：按任务类型配置 Provider/Model/Fallback/重试次数，支持 CRUD 与回显
  - 自定义 Provider：支持 `custom_openai_compatible` / `custom_anthropic_compatible` 关键字段输入与前端校验
  - 编辑器设置：字号/行高/自动保存间隔/叙事视角
  - 数据与备份：备份/恢复/完整性检查 + Git 仓库初始化、快照提交、历史查看
  - 关于：授权码激活、授权状态展示、检查更新、下载并安装更新
- 当前状态：
  - 编辑器设置走 DevEngine（localStorage）

## 5. 关键交互规范（已实现行为）
- AI 输出先进入预览区，用户显式选择后才写入编辑区
- 保存与自动保存分离，正式保存后清理对应自动草稿
- 关键动作 UI 可见三态（进行中/成功/失败）
- 一致性问题可追踪状态变更

## 6. 当前已知 UI 对齐差异
- 导出页面仍保留兼容 fallback 分支
- 编辑器设置仍未迁移到 Tauri 配置存储
- 自动更新依赖远端更新端点与签名配置，错误配置会导致安装失败提示

## 7. MVP 非目标（保持不变）
- 云同步
- 多人协作
- 插件市场

## 8. 文档维护规则
以下变化必须同步更新本文档：
- 页面新增/删除
- 关键交互变化（保存/恢复/AI 插入/错误提示）
- 页面数据来源切换（DevEngine -> Tauri 或反向）
