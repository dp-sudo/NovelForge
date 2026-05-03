# NovelForge

**NovelForge** 是一款基于 Tauri 2 + React 19 的 Windows 桌面小说创作辅助工具，提供蓝图规划、角色管理、世界观构建、章节编辑、AI 辅助创作等功能，帮助作者高效完成长篇小说创作。

## 项目概览

- **技术栈**: Tauri 2.10 + React 19 + TypeScript 5 + Vite 8 + Tailwind CSS 4
- **后端**: Rust (Tauri) + SQLite (rusqlite)
- **状态管理**: Zustand 5
- **数据查询**: TanStack Query 5
- **UI 组件**: Radix UI + Lucide React
- **目标平台**: Windows 桌面应用 (MSI 安装包)

## 核心功能

### 1. 项目管理
- 创建/打开小说项目
- 项目元数据管理（书名、作者、类型、目标字数）
- 最近项目列表
- 项目完整性检查
- Git 版本控制集成

### 2. 蓝图规划（8 步法）
- **Step 1**: 创作锚点（核心灵感、命题、情感）
- **Step 2**: 类型风格（主类型、叙事视角、节奏）
- **Step 3**: 故事前提（一句话概念、三段式摘要）
- **Step 4**: 角色设定（主角、反派、配角关系）
- **Step 5**: 世界观（背景、规则、地点、组织）
- **Step 6**: 名词表（人名、地名、术语、禁用词）
- **Step 7**: 情节结构（主线目标、阶段、冲突、转折）
- **Step 8**: 章节规划（卷结构、章节列表、目标）

### 3. 资产管理
- **角色**: 姓名、别名、角色类型、外貌、动机、欲望、恐惧、缺陷、成长弧
- **世界规则**: 标题、分类、描述、约束等级、相关实体
- **名词表**: 术语、类型、别名、锁定/禁用状态
- **情节节点**: 标题、节点类型、排序、目标、冲突、情感曲线

### 4. 章节编辑器
- 富文本编辑器（Textarea 实现）
- 实时字数统计
- 自动保存草稿（5 秒防抖）
- 手动保存与版本管理
- 草稿恢复提示
- 章节快照（Git-like）
- 卷管理与章节归卷
- 查找/替换功能（Ctrl+F）

### 5. AI 辅助创作
- **AI Pipeline 架构**: 统一任务流（validate → context → route → prompt → generate → postprocess → persist → done）
- **9 种编辑器任务**:
  - 续写章节 (`chapter.continue`)
  - 生成章节草稿 (`chapter.draft`)
  - 生成章节计划 (`chapter.plan`)
  - 改写选区 (`chapter.rewrite`)
  - 去 AI 味 (`prose.naturalize`)
  - 创建角色卡 (`character.create`)
  - 创建世界规则 (`world.create_rule`)
  - 创建剧情节点 (`plot.create_node`)
  - 一致性扫描 (`consistency.scan`)
- **流式响应**: 实时显示生成进度
- **任务路由**: 支持主路由 + 备用路由
- **多 Provider 支持**: DeepSeek、Kimi、智谱 GLM、MiniMax、OpenAI、Anthropic、Gemini、自定义
- **模型注册表**: 本地 + 远程更新

### 6. 上下文系统
- 章节关联角色/设定/情节/名词
- 资产候选自动抽取（角色、地点、组织、术语）
- 结构化草案（关系、戏份、场景）
- 人工确认入库流程
- 前章摘要引用

### 7. 一致性检查
- 名词表冲突检测
- 角色设定冲突
- 世界规则违反
- 时间线矛盾
- 文风偏离

### 8. 导出功能
- DOCX 导出（章节标题、摘要、分卷）
- PDF 导出
- EPUB 导出
- TXT 纯文本导出
- Markdown 导出

### 9. 设置与配置
- LLM Provider 配置（API Key、Base URL、模型）
- 任务路由配置
- 编辑器设置（字号、行高、自动保存间隔）
- 写作风格配置（语言风格、描写密度、对话比例、节奏、氛围）
- 技能管理（内置技能 + 自定义技能）
- 授权激活
- 自动更新

## 项目结构

```
NovelForge/
├── src/                          # 前端源码
│   ├── adapters/                 # 适配器层
│   ├── api/                      # Tauri 命令调用封装
│   ├── app/                      # 应用入口与路由
│   ├── components/               # React 组件
│   │   ├── ai/                   # AI 相关组件
│   │   ├── cards/                # 卡片组件
│   │   ├── dialogs/              # 对话框组件
│   │   ├── editor/               # 编辑器组件
│   │   ├── forms/                # 表单组件
│   │   ├── layout/               # 布局组件
│   │   ├── skills/               # 技能管理组件
│   │   └── ui/                   # 基础 UI 组件
│   ├── domain/                   # 领域类型与常量
│   ├── errors/                   # 错误处理
│   ├── hooks/                    # React Hooks
│   ├── infra/                    # 基础设施层
│   ├── lib/                      # 工具函数
│   ├── pages/                    # 页面组件
│   │   ├── Blueprint/            # 蓝图页面
│   │   ├── Chapters/             # 章节列表页面
│   │   ├── Characters/           # 角色管理页面
│   │   ├── Consistency/          # 一致性检查页面
│   │   ├── Dashboard/            # 仪表盘页面
│   │   ├── Editor/               # 编辑器页面
│   │   ├── Export/               # 导出页面
│   │   ├── Glossary/             # 名词表页面
│   │   ├── Narrative/            # 叙事义务页面
│   │   ├── Plot/                 # 情节页面
│   │   ├── ProjectCenter/        # 项目中心页面
│   │   ├── Relationships/        # 关系图页面
│   │   ├── Settings/             # 设置页面
│   │   ├── Timeline/             # 时间线页面
│   │   └── World/                # 世界观页面
│   ├── stores/                   # Zustand 状态管理
│   ├── types/                    # TypeScript 类型定义
│   ├── utils/                    # 工具函数
│   ├── index.ts                  # 入口文件
│   └── main.tsx                  # React 渲染入口
├── src-tauri/                    # Tauri 后端源码
│   ├── src/
│   │   ├── adapters/             # LLM 适配器
│   │   ├── commands/             # Tauri 命令
│   │   ├── domain/               # 领域模型
│   │   ├── infra/                # 基础设施（数据库、文件系统）
│   │   ├── services/             # 业务服务
│   │   ├── errors.rs             # 错误定义
│   │   ├── lib.rs                # 库入口
│   │   ├── main.rs               # 主入口
│   │   └── state.rs              # 应用状态
│   ├── migrations/               # 数据库迁移
│   │   ├── app/                  # 应用级数据库迁移
│   │   └── project/              # 项目级数据库迁移
│   ├── Cargo.toml                # Rust 依赖配置
│   └── tauri.conf.json           # Tauri 配置
├── resources/                    # 资源文件
│   ├── builtin-skills/           # 内置技能模板
│   └── model-registry/           # LLM 模型注册表
├── prompts/                      # AI 提示词模板
├── docs/                         # 文档目录
│   ├── architecture/             # 架构文档
│   ├── ui/                       # UI 设计文档
│   ├── runtime/                  # 运行时流程文档
│   └── api/                      # API 集成文档
├── public/                       # 静态资源
├── dist/                         # 构建输出
├── index.html                    # HTML 入口
├── package.json                  # Node.js 依赖配置
├── tsconfig.json                 # TypeScript 配置
├── vite.config.ts                # Vite 配置
├── tailwind.config.js            # Tailwind CSS 配置
├── components.json               # Shadcn UI 配置
├── AGENTS.md                     # AI 编码规范
├── CLAUDE.md                     # Claude 特定指南
└── README.md                     # 本文件
```

## 快速开始

### 环境要求

- **Node.js**: >= 18.0.0
- **pnpm**: >= 8.0.0
- **Rust**: >= 1.77.2
- **操作系统**: Windows 10/11

### 安装依赖

```bash
# 安装前端依赖
pnpm install

# Rust 依赖会在首次构建时自动安装
```

### 开发模式

```bash
# 启动 Tauri 开发服务器（前端 + 后端热重载）
pnpm tauri:dev
```

### 构建生产版本

```bash
# 构建 Windows MSI 安装包
pnpm tauri:build
```

构建产物位于 `src-tauri/target/release/bundle/msi/`。

### 测试

```bash
# 运行前端类型检查
pnpm typecheck

# 运行 Node.js 测试
pnpm test

# 运行集成测试
pnpm test:integration
```

## 核心技术架构

### 前端架构

- **路由**: 基于 Zustand 的客户端路由（无 React Router）
- **状态管理**: 
  - `projectStore`: 项目元数据、统计数据
  - `uiStore`: UI 状态（路由、侧边栏、主题、全局错误）
  - `editorStore`: 编辑器状态（内容、保存状态、AI 流状态）
- **数据查询**: TanStack Query 用于异步数据获取与缓存
- **样式**: Tailwind CSS 4 + CSS 变量主题系统

### 后端架构

- **命令层**: Tauri 命令（`#[tauri::command]`）
- **服务层**: 
  - `AiPipelineService`: AI 任务编排与流式响应
  - `AiService`: LLM 适配器管理
  - `SkillRegistry`: 技能模板管理
  - `ProjectService`: 项目生命周期管理
  - `ChapterService`: 章节内容管理
  - `VolumeService`: 卷管理
  - `CharacterService`: 角色管理
  - `RelationshipService`: 角色关系管理
  - `WorldService`: 世界规则管理
  - `GlossaryService`: 名词表管理
  - `PlotService`: 情节节点管理
  - `NarrativeService`: 叙事义务管理
  - `BlueprintService`: 蓝图步骤管理
  - `ConsistencyService`: 一致性检查
  - `ContextService`: 上下文聚合与结构化抽取
  - `DashboardService`: 仪表盘统计
  - `ExportService`: 导出功能
  - `SearchService`: 关键字搜索
  - `VectorService`: 语义搜索
  - `IntegrityService`: 项目完整性检查
  - `BackupService`: 备份与恢复
  - `ImportService`: 章节导入
  - `GitService`: Git 版本控制
  - `LicenseService`: 授权管理
  - `ModelRegistryService`: 模型注册表
  - `SettingsService`: 设置管理
- **适配器层**: 
  - `OpenAiCompatibleAdapter`: 统一 LLM 接口适配
  - 支持 OpenAI、Anthropic、Gemini 协议
- **基础设施层**:
  - `app_database`: 应用级 SQLite（Provider、TaskRoute、Model）
  - `database`: 项目级 SQLite（Chapter、Character、WorldRule 等）
  - `credential_manager`: 系统密钥环集成（Windows Credential Manager）
  - `logger`: 结构化日志
  - `migrator`: 数据库迁移管理

### AI Pipeline 流程

```
用户触发任务
  ↓
validate (参数校验)
  ↓
context (上下文聚合: 章节、角色、设定、名词)
  ↓
route (任务路由: 查找 Provider + Model)
  ↓
prompt (提示词构建: 技能模板 + 上下文注入)
  ↓
generate (LLM 流式生成)
  ↓
postprocess (结果整理: 去除标记、格式化)
  ↓
persist (可选: 自动落库)
  ↓
done (返回结果)
```

每个阶段通过 `ai:pipeline:event` 事件向前端推送进度。

### 数据库设计

#### 应用级数据库 (`~/.novelforge/novelforge.db`)

- `llm_providers`: LLM Provider 配置
- `llm_models`: 模型注册表
- `llm_model_refresh_logs`: 模型刷新日志
- `llm_task_routes`: 任务路由配置
- `llm_model_registry_state`: 模型注册表状态
- `app_settings`: 应用设置（编辑器设置等）
- `recent_projects`: 最近打开项目

#### 项目级数据库 (`<ProjectRoot>/database/project.sqlite`)

- `projects`: 项目元数据
- `blueprint_steps`: 蓝图步骤
- `chapters`: 章节元数据
- `characters`: 角色
- `character_relationships`: 角色关系
- `world_rules`: 世界规则
- `glossary_terms`: 名词表
- `plot_nodes`: 情节节点
- `chapter_characters`: 章节-角色关联
- `chapter_world_rules`: 章节-设定关联
- `chapter_plot_nodes`: 章节-情节关联
- `consistency_issues`: 一致性问题
- `narrative_obligations`: 叙事义务
- `asset_candidates`: 资产候选
- `structured_draft_batches`: 结构化草案批次
- `structured_draft_items`: 结构化草案项
- `ai_requests`: AI 请求审计
- `ai_pipeline_runs`: AI Pipeline 运行审计
- `snapshots`: 章节快照
- `volumes`: 卷管理

### 文件系统布局

```
<ProjectRoot>/
├── project.json                  # 项目元数据
├── database/
│   ├── project.sqlite            # 项目数据库
│   └── vector-index.json         # 语义索引
├── manuscript/                   # 正文目录
│   ├── chapters/                 # 章节正文
│   │   └── chapter_<id>.md
│   ├── drafts/                   # 草稿目录
│   │   └── chapter_<id>_draft.md
│   └── snapshots/                # 快照目录
│       └── snapshot_<id>.md
├── blueprint/                    # 蓝图目录
│   ├── step-01-anchor.md
│   ├── step-02-genre.md
│   └── ...
├── exports/                      # 导出目录
├── backups/                      # 备份目录
│   └── backup_<timestamp>.zip
└── .git/                         # Git 仓库（可选）
```

## 文档索引

本项目维护以下核心文档：

1. **[README.md](README.md)** - 项目总览与快速开始（本文件）
2. **[docs/README.md](docs/README.md)** - 文档入口与导航
3. **[架构文档](docs/architecture/windows-desktop-architecture.md)** - 技术架构、模块设计、数据流
4. **[UI 设计文档](docs/ui/ui-design-spec.md)** - 页面结构、组件规范、交互设计
5. **[运行时流程文档](docs/runtime/runtime-process-spec.md)** - 关键流程、状态机、错误处理
6. **[API 集成文档](docs/api/api-integration-spec.md)** - Tauri 命令、事件协议、类型定义

### 文档更新原则

- **变更即更新**: 代码行为变化时，必须同步更新相关文档
- **责任分工**: 
  - 架构文档: 后端/Tauri 实现负责人
  - UI 设计文档: 前端实现负责人
  - 运行时流程文档: 主流程串联开发负责人
  - API 集成文档: 前后端接口变更提交人
- **验收清单**: 
  - 链接路径可打开
  - 命令名、字段名、错误码与代码一致
  - 描述的是已实现行为，非计划性描述
  - 事件协议与 UI 消费逻辑一致

## 开发规范

请参阅 **[AGENTS.md](AGENTS.md)** 了解本项目的 AI 编码规范，包括：

- 最小必要改动原则
- 上下文理解要求
- 测试优先与验证闭环
- 安全与破坏性操作处理
- 文档与注释规范

## 授权与更新

- **授权**: 支持在线激活与离线激活
- **自动更新**: 基于 Tauri Updater 插件，检查远程更新并下载安装

## 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交变更 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目为私有项目，未经授权不得分发或商用。

## 联系方式

- **项目主页**: https://novelforge.app
- **问题反馈**: https://github.com/novelforge/desktop/issues
- **邮箱**: support@novelforge.app

---

**NovelForge** - 让创作更高效，让故事更精彩。
