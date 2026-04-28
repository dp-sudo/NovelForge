# NovelForge Skills 系统重构方案

> 本文档详细规划基于 Markdown 文件的 Skills 系统的完整重构方案。
> 涵盖后端存储架构、前端管理 UI、与现有 AI 系统的集成方式。

**版本:** v1.0  
**状态:** 规划中  
**目标:** 从硬编码 8 个 Skill 升级为"Markdown 文件驱动 + 用户可管理"的完整 Skills 系统

---

## 目录

1. [现状分析](#1-现状分析)
2. [重构目标](#2-重构目标)
3. [整体架构](#3-整体架构)
4. [Skill Markdown 文件格式](#4-skill-markdown-文件格式)
5. [文件存储方案](#5-文件存储方案)
6. [后端设计](#6-后端设计)
7. [前端设计](#7-前端设计)
8. [集成方案](#8-集成方案)
9. [分阶段实施计划](#9-分阶段实施计划)
10. [风险与注意事项](#10-风险与注意事项)

---

## 1. 现状分析

### 当前实现

| 维度 | 当前状态 |
|---|---|
| Skill 存储 | 硬编码在 Rust 结构体中（`skill_registry.rs:30-155`） |
| Skill 数量 | 8 个（文档规划 12 个，缺失 4 个） |
| 存储介质 | 纯内存（`Vec<SkillManifest>`），应用重启后丢失 |
| 可扩展性 | 无 `register_skill` 公共方法，运行时不可添加 |
| 前端消费 | `list_skills` Tauri 命令存在但**没有任何前端代码调用** |
| AI 命令栏 | `AiCommandBar.tsx` 硬编码 6 个按钮，不依赖 Skills 系统 |
| 数据库 | 没有任何 skill 相关的数据库表 |
| 持久化 | 无——Skill 无法被用户自定义 |

### 核心问题

1. **Skill 不可持久化**：应用重启后自定义 Skill 全部丢失
2. **不可扩展**：用户无法添加自己的 Skill
3. **不可编辑**：无法修改内置 Skill 的提示词内容
4. **前后端断裂**：`list_skills` 命令存在但前端未使用
5. **命名不一致**：Skill ID 用 `chapter.draft` 但前端 taskType 用 `generate_chapter_draft`

---

## 2. 重构目标

### 核心原则

1. **Markdown 文件作为 Skill 的唯一真实来源（Source of Truth）**
2. **用户完全可控**：导入、编辑、删除、禁用
3. **内置 Skill 可恢复**：出厂重置不丢失基础能力
4. **前后端一致**：`AiCommandBar` 由 Skills 系统驱动

### 功能性目标

```
用户可：
  ├─ 浏览所有已安装 Skill（列表 + 详情）
  ├─ 导入新的 .md Skill 文件（拖拽 / 文件选择器）
  ├─ 编辑已有 Skill 的 Markdown 内容（内置编辑器）
  ├─ 删除自定义 Skill（内置 Skill 可重置）
  ├─ 启用/禁用单个 Skill
  └─ 查看 Skill 的输入/输出 Schema
```

---

## 3. 整体架构

```
┌─────────────────────────────────────────────────────┐
│                    Frontend (React)                   │
│                                                       │
│  SkillsPage (设置 Tab)   AiCommandBar (编辑器)      │
│  ├─ 技能列表浏览          └─ 动态加载自 Skills API   │
│  ├─ Markdown 编辑器                                  │
│  ├─ 导入按钮（文件选择）                              │
│  └─ 预览面板                                         │
└──────────────────┬──────────────────────────────────┘
                   │ Tauri invoke
┌──────────────────▼──────────────────────────────────┐
│                   Backend (Rust)                      │
│                                                       │
│  skills_commands.rs    SkillRegistryService          │
│  ├─ list_skills        ├─ load_from_fs()              │
│  ├─ get_skill          ├─ save_to_file()              │
│  ├─ create_skill       ├─ delete_file()               │
│  ├─ update_skill       ├─ reset_builtin()             │
│  ├─ delete_skill       └─ import_from_md()            │
│  └─ reset_builtin_skills                              │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│               Storage Layer                           │
│                                                       │
│  ┌─────────────────────┐  ┌──────────────────────┐   │
│  │ ~/.novelforge/skills/ │  │ App Database          │   │
│  │ ├─ chapter.draft.md   │  │ skill_index 表        │   │
│  │ ├─ custom-my-skill.md │  │ (元数据索引，加速查询) │   │
│  │ └─ ...                │  └──────────────────────┘   │
│  └─────────────────────┘                               │
│                                                       │
│  ┌─────────────────────┐                              │
│  │ resources/builtin-skills/ (Bundled)               │
│  │ ├─ chapter.draft.md                                │
│  │ ├─ chapter.continue.md                             │
│  │ ├─ character.create.md                             │
│  │ └─ ... (12 个内置)                                 │
│  └─────────────────────┘                              │
└──────────────────────────────────────────────────────┘
```

---

## 4. Skill Markdown 文件格式

### 完整格式定义

每个 Skill 是一个 `.md` 文件，包含 YAML frontmatter + Markdown 正文。

```markdown
---
# ── 核心标识 ──
id: chapter.draft                    # 唯一 ID（文件名去掉扩展名）
name: 生成章节草稿                    # 显示名称
description: 根据章节目标、角色、设定、主线节点生成章节正文草稿
version: 1                           # 版本号，导入时递增
source: builtin                       # builtin | user | imported

# ── 分类与标签 ──
category: writing                     # writing | character | world | plot | review | utility
tags: [写作, 章节, 草稿]

# ── 输入/输出 Schema ──
inputSchema:
  type: object
  properties:
    chapterId:
      type: string
      description: 目标章节 ID
    userInstruction:
      type: string
      description: 用户的额外指令
  required: [chapterId]

outputSchema:
  type: object
  properties:
    draft:
      type: string
      description: 生成的章节正文

# ── 行为配置 ──
requiresUserConfirmation: true        # 执行结果是否需要用户确认才写入
writesToProject: false                # 是否直接写入项目文件
promptStrategy: replace               # replace | append | insert 提示词拼接策略

# ── 元数据 ──
author: NovelForge
createdAt: 2026-04-28
updatedAt: 2026-04-28
icon: ✍️                              # 可选图标 emoji

# ── 任务路由覆盖（可选） ──
taskRoute:
  taskType: chapter_draft
  providerId: ""                      # 空 = 使用全局路由
  modelId: ""
---

# 生成章节草稿

## 角色设定

你是一名专业的长篇小说创作助手，精通叙事结构、人物塑造和文学风格。
你擅长将抽象的章节目标转化为具体的、高质量的叙事文本。

## 任务描述

根据以下提供的小说设定、角色信息、世界观规则和主线节点，
为用户指定的章节生成完整的正文草稿。

## 项目上下文

{projectContext}

## 章节上下文

{chapterContext}

## 关联角色

{relatedCharacters}

## 关联世界规则

{worldRules}

## 关联主线节点

{plotNodes}

## 用户指令

{userInstruction}

## 输出要求

1. 章节应有明确的叙事推进
2. 保持与前文一致的视角和文风
3. 融入关联角色的性格特征
4. 遵循世界观规则的约束
5. 自然地呼应主线节点
6. 字数：{targetWords} 字左右

## 输出格式

直接输出章节正文，无需额外说明。
```

### Frontmatter 字段说明

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `id` | string | ✅ | 全局唯一标识，对应文件名 `{id}.md` |
| `name` | string | ✅ | 前端显示名称 |
| `description` | string | ✅ | 简短描述（列表中使用） |
| `version` | integer | 默认 1 | 版本号，导入覆盖时递增 |
| `source` | enum | ✅ | `builtin` / `user` / `imported` |
| `category` | enum | 默认 utility | 分组：`writing`/`character`/`world`/`plot`/`review`/`utility` |
| `tags` | string[] | 可选 | 用于过滤和搜索 |
| `inputSchema` | JSON Schema | ✅ | 描述该 Skill 需要的输入参数 |
| `outputSchema` | JSON Schema | ✅ | 描述该 Skill 的输出结构 |
| `requiresUserConfirmation` | bool | 默认 true | 结果写入前是否需要用户确认 |
| `writesToProject` | bool | 默认 false | 是否直接修改项目文件 |
| `promptStrategy` | enum | 默认 replace | 提示词上下文拼接方式 |
| `author` | string | 可选 | 创建者 |
| `createdAt` | date | ✅ | ISO 8601 日期 |
| `updatedAt` | date | ✅ | ISO 8601 日期 |
| `icon` | string | 可选 | 显示用 emoji |
| `taskRoute` | object | 可选 | 覆盖默认的任务路由配置 |

### 模板变量

正文 Markdown 中可以使用 `{variable}` 占位符，系统在执行时注入：

| 变量 | 来源 | 说明 |
|---|---|---|
| `{projectContext}` | `ContextService::collect_global_context_only()` | 项目概要设定 |
| `{chapterContext}` | `ContextService::collect_chapter_context()` | 当前章节信息 |
| `{relatedCharacters}` | `chapter_links` 表 | 章节关联角色列表 |
| `{worldRules}` | `chapter_links` 表 | 章节关联世界规则 |
| `{plotNodes}` | `chapter_links` 表 | 章节关联主线节点 |
| `{userInstruction}` | 用户输入 | 用户的自定义指令 |
| `{targetWords}` | 输入参数 | 目标字数 |
| `{selectedText}` | 输入参数 | 用户选中的文本 |

---

## 5. 文件存储方案

### 目录结构

```
~/.novelforge/
├── skills/                          # 用户 skill 目录（运行时）
│   ├── chapter.draft.md             # 内置 skill（用户可编辑）
│   ├── chapter.continue.md
│   ├── character.create.md
│   ├── consistency.scan.md
│   ├── custom-my-helper.md          # 用户自定义 skill
│   └── imported-workflow.md         # 用户导入的 skill
│
└── novelforge.db                    # App 数据库（含 skill_index 表）

[App Bundle]
└── resources/
    └── builtin-skills/              # 安装包内置（只读基线）
        ├── chapter.draft.md
        ├── chapter.continue.md
        ├── chapter.rewrite.md
        ├── prose.naturalize.md
        ├── chapter.plan.md
        ├── blueprint.generate_step.md
        ├── character.create.md
        ├── world.create_rule.md
        ├── plot.create_node.md
        ├── consistency.scan.md
        ├── context.collect.md
        └── import.extract_assets.md
```

### 初始化流程

```
应用首次启动：
  1. 检测 ~/.novelforge/skills/ 目录是否存在
  2. 若不存在：
     a. 创建目录
     b. 将 resources/builtin-skills/*.md 复制到 skills/
     c. 写入 skill_index 表
  3. 若存在但缺少某些内置 skill：
     a. 补充缺失的内置 .md 文件（不覆盖已有的）
  4. 扫描 skills/ 目录刷新内存 registry
```

### 数据库索引表

```sql
CREATE TABLE IF NOT EXISTS skill_index (
  id TEXT PRIMARY KEY,               -- skill ID，对应文件名
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  category TEXT NOT NULL DEFAULT 'utility',
  source TEXT NOT NULL DEFAULT 'user', -- builtin | user | imported
  version INTEGER NOT NULL DEFAULT 1,
  tags TEXT,                          -- JSON array
  is_enabled INTEGER NOT NULL DEFAULT 1,
  file_path TEXT NOT NULL,            -- 对应 .md 文件的绝对路径
  file_hash TEXT,                     -- 文件内容 SHA256，用于检测变更
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

---

## 6. 后端设计

### 6.1 新的 Rust 模块结构

```
src-tauri/src/
├── services/
│   ├── skill_registry.rs      ← 重写：基于文件系统的 SkillRegistry
│   └── ... (现有服务)
│
├── commands/
│   ├── skill_commands.rs      ← 新建：所有 Skill 相关的 Tauri 命令
│   └── ... (现有命令)
```

### 6.2 SkillRegistry 重写

```rust
pub struct SkillRegistry {
    skills_dir: PathBuf,               // ~/.novelforge/skills/
    builtin_dir: PathBuf,              // resources/builtin-skills/ (embedded)
}

impl SkillRegistry {
    // ── 生命周期 ──
    pub fn new(app_data_dir: PathBuf, builtin_dir: PathBuf) -> Result<Self>;
    pub fn initialize(&mut self) -> Result<()>;     // 首次运行复制内置 skill
    pub fn reload(&mut self) -> Result<()>;          // 重新扫描文件系统

    // ── CRUD ──
    pub fn list_skills(&self) -> Result<Vec<SkillManifest>>;
    pub fn get_skill(&self, id: &str) -> Result<Option<SkillManifest>>;
    pub fn create_skill(&self, manifest: SkillManifest, content: &str) -> Result<()>;
    pub fn update_skill(&self, id: &str, content: &str) -> Result<()>;
    pub fn delete_skill(&self, id: &str) -> Result<()>;
    pub fn reset_builtin(&self, id: &str) -> Result<()>;  // 从 builtin 恢复原始

    // ── 文件操作 ──
    fn load_from_fs(&self, id: &str) -> Result<Option<(SkillManifest, String)>>;
    fn save_to_file(&self, manifest: &SkillManifest, content: &str) -> Result<()>;
    fn delete_file(&self, id: &str) -> Result<()>;
    fn parse_skill_file(content: &str) -> Result<(SkillManifest, String)>;
    fn read_file_content(path: &Path) -> Result<String>;
}
```

### 6.3 Tauri 命令（新建 `skill_commands.rs`）

```rust
// ── 列表与查询 ──
#[tauri::command]
async fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillManifest>, AppErrorDto>;

#[tauri::command]
async fn get_skill(id: String, state: State<'_, AppState>) -> Result<SkillManifest, AppErrorDto>;

#[tauri::command]
async fn get_skill_content(id: String, state: State<'_, AppState>) -> Result<String, AppErrorDto>;

// ── 增删改 ──
#[tauri::command]
async fn create_skill(
    manifest: SkillManifest,
    content: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto>;

#[tauri::command]
async fn update_skill(
    id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto>;

#[tauri::command]
async fn delete_skill(id: String, state: State<'_, AppState>) -> Result<(), AppErrorDto>;

// ── 导入与重置 ──
#[tauri::command]
async fn import_skill_file(
    file_path: String,       // 用户选择的 .md 文件路径
    state: State<'_, AppState>,
) -> Result<SkillManifest, AppErrorDto>;

#[tauri::command]
async fn reset_builtin_skill(id: String, state: State<'_, AppState>) -> Result<SkillManifest, AppErrorDto>;
```

### 6.4 SkillManifest 扩展

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: i32,
    pub source: String,              // "builtin" | "user" | "imported"
    pub category: String,            // "writing" | "character" | "world" | "plot" | "review" | "utility"
    pub tags: Vec<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub requires_user_confirmation: bool,
    pub writes_to_project: bool,
    pub prompt_strategy: String,     // "replace" | "append" | "insert"
    pub author: Option<String>,
    pub icon: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

### 6.5 AppState 变更

```rust
pub struct AppState {
    pub skill_registry: Arc<RwLock<SkillRegistry>>,  // 从 SkillRegistry 改为线程安全
    // ... 其他现有字段
}
```

### 6.6 前端 API 模块（新建 `src/api/skillsApi.ts`）

```typescript
export interface SkillManifest {
  id: string;
  name: string;
  description: string;
  version: number;
  source: "builtin" | "user" | "imported";
  category: string;
  tags: string[];
  inputSchema: Record<string, unknown>;
  outputSchema: Record<string, unknown>;
  requiresUserConfirmation: boolean;
  writesToProject: boolean;
  promptStrategy: string;
  author?: string;
  icon?: string;
  createdAt: string;
  updatedAt: string;
}

export async function listSkills(): Promise<SkillManifest[]>;
export async function getSkill(id: string): Promise<SkillManifest>;
export async function getSkillContent(id: string): Promise<string>;
export async function createSkill(manifest: SkillManifest, content: string): Promise<SkillManifest>;
export async function updateSkill(id: string, content: string): Promise<SkillManifest>;
export async function deleteSkill(id: string): Promise<void>;
export async function importSkillFile(filePath: string): Promise<SkillManifest>;
export async function resetBuiltinSkill(id: string): Promise<SkillManifest>;
```

---

## 7. 前端设计

### 7.1 页面结构

```text
设置
└── 技能管理 (新 Tab)
    ├── 技能列表 (左面板)
    │   ├── 分类过滤 (全部 / 写作 / 角色 / 世界观 / 剧情 / 审稿 / 工具)
    │   ├── 搜索框
    │   └── 技能卡片列表
    │       ├── 图标 + 名称 + 描述
    │       ├── source 标签 (内置/自定义/已导入)
    │       ├── 启用/禁用开关
    │       └── 选中态 → 右面板详情
    │
    ├── 技能详情 (右面板)
    │   ├── 基本信息 (ID, 版本, 分类, 标签)
    │   ├── Schema 查看 (输入/输出)
    │   ├── Markdown 编辑器 (代码编辑器风格)
    │   │   ├── 语法高亮
    │   │   ├── 行号
    │   │   └── 只读 frontmatter (保护关键字段)
    │   └── 操作按钮
    │       ├── 保存修改
    │       ├── 删除 (仅自定义/已导入)
    │       ├── 重置为出厂 (仅内置)
    │       └── 预览效果
    │
    └── 工具栏 (顶部)
        ├── 导入技能 (文件选择器 → .md)
        ├── 导出技能 (选中 → .md)
        ├── 刷新列表
        └── 恢复所有内置技能
```

### 7.2 AiCommandBar 重构

从硬编码改为动态加载：

```typescript
export function AiCommandBar({ onCommand, disabled }: AiCommandBarProps) {
  const [skills, setSkills] = useState<SkillManifest[]>([]);

  useEffect(() => {
    listSkills().then(setSkills).catch(() => {});
  }, []);

  const activeSkills = skills.filter(s => s.source !== "context.collect");
  // 按 category 分组: writing, character, world, plot, review
  // 每组渲染为按钮或下拉

  return (
    <div className="flex flex-col gap-2">
      {groups.map(group => (
        <div key={group.category} className="flex gap-2 flex-wrap">
          {group.skills.map(skill => (
            <button key={skill.id} onClick={() => onCommand(skill.id, "")}>
              {skill.icon} {skill.name}
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
```

### 7.3 组件树

```
src/
├── components/
│   ├── skills/
│   │   ├── SkillsManager.tsx        # 主容器（左右分栏布局）
│   │   ├── SkillList.tsx            # 左面板：技能列表
│   │   ├── SkillCard.tsx            # 列表中的技能卡片
│   │   ├── SkillDetail.tsx          # 右面板：技能详情 + 编辑器
│   │   ├── SkillEditor.tsx          # Markdown 编辑器组件
│   │   ├── SkillSchemaView.tsx      # Schema 展示面板
│   │   ├── SkillImportButton.tsx    # 导入按钮 + 文件选择器
│   │   └── SkillFilterBar.tsx       # 分类过滤 + 搜索
│   │
│   └── ai/
│       └── AiCommandBar.tsx         ← 重构：动态加载技能
│
├── api/
│   └── skillsApi.ts                 ← 新建：Skills API 封装
│
├── pages/
│   └── Settings/
│       └── SettingsPage.tsx         ← 新增 "技能管理" Tab
│
└── stores/
    └── skillStore.ts                ← 新建：Zustand store for skills
```

### 7.4 SkillEditor 组件设计

```typescript
interface SkillEditorProps {
  initialContent: string;         // .md 文件的完整内容
  readOnlyFields: string[];       // 只读 frontmatter 字段（如 id, source）
  onSave: (content: string) => Promise<void>;
}
```

功能：
- 语法高亮（可使用 `CodeMirror` 或 `Monaco Editor` 的 Markdown 模式）
- 只读保护的 frontmatter（灰色背景，不可修改 `id`/`source`/`version`）
- 正文可自由编辑
- 保存时自动格式化 frontmatter
- 未保存提示

---

## 8. 集成方案

### 8.1 与现有 PromptBuilder 的集成

当前 `PromptBuilder` 在 Rust 端构建提示词，其方法如 `build_chapter_draft()` 等内部构造带角色/任务/上下文的提示字符串。

重构后，Skill 的 **正文 Markdown 内容** 替代 `PromptBuilder` 中的硬编码提示模板：

```
重构前：
  PromptBuilder::build_chapter_draft(context, instruction)
    → 返回硬编码模板 + 上下文

重构后：
  SkillRegistry::get_skill("chapter.draft")?.content
    → 读取 chapter.draft.md 文件
    → 替换 {projectContext}, {userInstruction} 等占位符
    → 返回最终提示词
```

这意味着：
1. `PromptBuilder` 逐步迁移到 Skill Markdown 文件
2. 每个内置 Skill 的 `.md` 文件内容是 `PromptBuilder` 中对应方法的提示模板
3. 用户编辑 Skill 的 `.md` 文件即编辑提示词

### 8.2 与 AiCommandBar 的集成

| 当前 | 重构后 |
|---|---|
| `AiCommandBar.tsx` 硬编码 6 个按钮 | 从 `listSkills()` API 动态加载 |
| taskType 使用不同的命名 (`generate_chapter_draft`) | taskType 统一为 skill.id (`chapter.draft`) |
| 无分类 | 按 `category` 字段分组 |
| 无图标 | 显示 `icon` 字段 emoji |

### 8.3 与任务路由的集成

Skill 的 `taskRoute` 字段可以覆盖全局任务路由。当 AiService 收到一个 skill ID 时：
1. 查找该 skill 的 `taskRoute` 配置
2. 如果有覆盖，使用 `providerId` + `modelId`
3. 如果没有覆盖，使用全局 `llm_task_routes` 表

---

## 9. 分阶段实施计划

### 第一阶段：基础设施（预计 4-6 小时）

| # | 任务 | 文件 | 依赖 |
|---|---|---|---|
| 1.1 | 扩展 `SkillManifest` 结构体 | `skill_registry.rs` | 无 |
| 1.2 | 实现 `.md` 文件解析（frontmatter + body） | `skill_registry.rs` | 1.1 |
| 1.3 | 实现文件系统 CRUD（创建/读取/更新/删除） | `skill_registry.rs` | 1.2 |
| 1.4 | 创建 `skill_index` 数据库表 + 迁移 | `app_database.rs` | 无 |
| 1.5 | 创建 12 个内置 Skill `.md` 文件 | `resources/builtin-skills/` | 无 |
| 1.6 | 实现首次初始化流程（复制内置 → 用户目录） | `skill_registry.rs` | 1.4, 1.5 |
| 1.7 | 将 `SkillRegistry` 改为 `Arc<RwLock<>>` | `state.rs`, `skill_registry.rs` | 1.3 |

### 第二阶段：API 与命令（预计 4-6 小时）

| # | 任务 | 文件 | 依赖 |
|---|---|---|---|
| 2.1 | 新建 `skill_commands.rs`：list/get/get_content | `commands/skill_commands.rs` | 1.3 |
| 2.2 | 新建 `skill_commands.rs`：create/update/delete | `commands/skill_commands.rs` | 1.3 |
| 2.3 | 新建 `skill_commands.rs`：import/reset | `commands/skill_commands.rs` | 1.6 |
| 2.4 | 注册命令到 `lib.rs` | `lib.rs` | 2.1-2.3 |
| 2.5 | 新建前端 `skillsApi.ts` | `src/api/skillsApi.ts` | 2.4 |

### 第三阶段：前端 UI（预计 6-8 小时）

| # | 任务 | 文件 | 依赖 |
|---|---|---|---|
| 3.1 | 新建 `SkillList` + `SkillCard` 组件 | `src/components/skills/` | 2.5 |
| 3.2 | 新建 `SkillDetail` + `SkillEditor` 组件 | `src/components/skills/` | 2.5 |
| 3.3 | 新建 `SkillsManager` 主容器 + 分栏布局 | `src/components/skills/` | 3.1, 3.2 |
| 3.4 | 新建 `SkillImportButton` + 文件选择器 | `src/components/skills/` | 2.3 |
| 3.5 | 设置页新增"技能管理"Tab | `SettingsPage.tsx` | 3.3 |
| 3.6 | 新建 `skillStore` Zustand store | `src/stores/skillStore.ts` | 无 |

### 第四阶段：集成与迁移（预计 4-6 小时）

| # | 任务 | 文件 | 依赖 |
|---|---|---|---|
| 4.1 | 重构 `AiCommandBar`：从 Skills API 加载 | `AiCommandBar.tsx` | 2.5 |
| 4.2 | 统一 taskType 命名（`generate_chapter_draft` → `chapter.draft`） | `EditorPage.tsx`, `ai_commands.rs` | 无 |
| 4.3 | 迁移 `PromptBuilder` 部分方法到 Skill Markdown | `prompt_builder.rs`, `resources/builtin-skills/` | 1.2 |
| 4.4 | Skill `taskRoute` 覆盖集成到 AiService | `ai_service.rs` | 1.1 |
| 4.5 | 测试：12 个内置 Skill 全部可用 | e2e | 4.1 |
| 4.6 | 清理：移除旧的硬编码注册逻辑 | `skill_registry.rs` | 4.3 |

### 总计工时

| 阶段 | 预估工时 |
|---|---|
| 第一阶段：基础设施 | 4-6h |
| 第二阶段：API 与命令 | 4-6h |
| 第三阶段：前端 UI | 6-8h |
| 第四阶段：集成与迁移 | 4-6h |
| **总计** | **18-26h** |

---

## 10. 风险与注意事项

### 兼容性

1. **向后兼容**：保留旧的 `list_skills` 命令名和返回值格式，前端无感知
2. **任务路由兼容**：Skill ID 与 taskType 的映射，在迁移期间提供双向兼容层
3. **数据迁移**：现有用户的 `~/.novelforge/` 目录无 skills/ 子目录，首次启动自动创建

### 安全红线

1. **frontmatter 保护**：`id`、`source`、`createdAt` 字段禁止用户修改（编辑器只读保护）
2. **文件路径安全**：防止路径穿越攻击（`../../etc/passwd`）
3. **文件大小限制**：单 Skill 文件不超过 1MB
4. **导入校验**：导入的 `.md` 文件必须包含合法 frontmatter
5. **XSS 防护**：Markdown 渲染时注意转义

### 已知问题

1. **内置 Skill 覆盖**：用户编辑内置 Skill 后，再次保存不会丢失，但"重置"功能应恢复到安装包中的原始版本
2. **并发写入**：使用 `Arc<RwLock<>>` 保护 Registry，读多写少
3. **占位符解析**：`{variable}` 只替换已知变量，未知变量保持原样
4. **Skill 禁用**：禁用只是不在 AiCommandBar 显示，不删除文件

### 未涵盖的范围（后续考虑）

- **Skill 分享/市场**：在线技能市场、导入 URL
- **Skill 版本管理**：Git 风格 diff/历史
- **Skill 工作组**：多个 Skill 组合为工作流
- **Skill 测试沙箱**：在隔离环境中测试 Skill 效果

---

## 附录 A：12 个内置 Skill 清单

| # | ID | 名称 | 分类 | 确认 | 写入 | 对应 PromptBuilder 方法 |
|---|---|---|---|---|---|---|
| 1 | `chapter.draft` | 生成章节草稿 | writing | ✅ | ❌ | `build_chapter_draft()` |
| 2 | `chapter.continue` | 续写章节 | writing | ✅ | ❌ | `build_continue()` |
| 3 | `chapter.rewrite` | 改写选区 | writing | ✅ | ❌ | `build_rewrite()` |
| 4 | `prose.naturalize` | 去 AI 味 | writing | ✅ | ❌ | `build_naturalize()` |
| 5 | `chapter.plan` | 生成章节计划 | writing | ❌ | ❌ | `build_chapter_plan()` |
| 6 | `blueprint.generate_step` | 生成蓝图步骤 | utility | ✅ | ❌ | `build_blueprint_step()` |
| 7 | `character.create` | 创建角色卡 | character | ✅ | ❌ | `build_character_create()` |
| 8 | `world.create_rule` | 创建世界规则 | world | ❌ | ❌ | `build_world_create_rule()` |
| 9 | `plot.create_node` | 创建剧情节点 | plot | ❌ | ❌ | `build_plot_create_node()` |
| 10 | `consistency.scan` | 一致性扫描 | review | ❌ | ✅ | `build_consistency_scan()` |
| 11 | `context.collect` | 收集上下文 | utility | ❌ | ❌ | 内部服务 |
| 12 | `import.extract_assets` | 导入资产抽取 | utility | ❌ | ✅ | 新建 |

> ✅ = 当前已实现 | ❌ = 需要新增

## 附录 B：Skill Markdown 完整示例

详见 [示例文件](resources/builtin-skills/chapter.draft.md)（实施时创建）。
