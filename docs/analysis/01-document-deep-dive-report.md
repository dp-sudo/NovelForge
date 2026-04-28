# 文火 NovelForge 核心文档深度解读报告 v1.0

> 生成日期：2026-04-28
> 分析范围：AGENTS.md、novel_workbench_windows_lifecycle_blueprint_v_1.md、novelforge_llm_provider_integration_spec_v_1.md
> 分析目标：提取设计意图、核心决策、架构原则、MVP 边界

---

## 1. AGENTS.md — 项目执行宪章

### 1.1 文档定位

AGENTS.md 不是技术架构文档，而是 **AI Agent 开发协作的行为规范和执行流程定义**。它约束的是"怎么开发"，而不是"开发什么"。

### 1.2 核心决策

| 决策 | 内容 | 影响 |
|---|---|---|
| 项目阶段 | MVP 主闭环实现与验收 | 所有工作应聚焦主闭环，不扩散 |
| 不编造原则 | 不编造路径、接口、命令输出、测试结果、提交记录 | 对 AI Agent 的行为底线约束 |
| 文档优先 | 先读实现再改文档；文档反映已实现行为 | 防止文档和实现脱节 |
| 修改范围 | 仅改请求范围内内容，不做顺手重构 | 控制范围膨胀 |
| 验证纪律 | 未验证前禁止宣称"完成/通过" | 必须提供可执行的验证证据 |

### 1.3 MVP 主闭环定义

```
启动软件 → 项目中心 → 新建项目 → 仪表盘 → 创作蓝图 →
创建角色/设定/名词/主线 → 创建章节 → 章节编辑器写作 →
AI 生成草稿/续写/改写 → 用户确认插入 → 自动保存/手动保存 →
一致性检查 → 导出 TXT/Markdown → 关闭重开数据完整恢复
```

### 1.4 非目标（MVP 禁止）

- 云同步
- 多人协作
- 插件市场
- DOCX/PDF/EPUB 导出（这些在代码中实际已实现——值得注意的偏差）

### 1.5 架构铁律

| 规则 | 含义 |
|---|---|
| 前端仅通过 Tauri command 调用后端 | 禁止前端直连数据库或文件系统 |
| 章节正文必须落盘为 Markdown | 数据库只保存元数据 |
| 原子写策略 | 临时文件 + 重命名 |
| API Key 不得明文写入项目目录和日志 | 安全红线 |

---

## 2. Blueprint — 全生命周期开发蓝图

### 2.1 文档定位

这是项目的 **完整设计规格说明书**，由六份子文档组成，覆盖了从产品需求到技术实现的全链路。

### 2.2 六份子文档解读

#### 文档一：MVP PRD

**目标用户**：中文长篇小说作者、网文作者、剧本/IP 创作者

**核心愿景**："让灵感成为工程，让故事稳定完稿"

**产品定位**：作者本地的 AI 小说创作 IDE，不是简单的 AI 聊天窗口

**P0 功能清单**（20 项）：
- 项目中心：新建/打开/最近项目
- 仪表盘：数据概览
- 蓝图：8 步表单 + AI 建议
- 角色/世界/名词/剧情：完整 CRUD
- 章节编辑器：Markdown + 自动保存
- AI 功能：草稿生成/续写/改写
- 一致性检查 + TXT/MD 导出

**PRD 特别值得注意的设计细节**：
- 蓝图 8 步是渐进式结构（灵感→类型→故事→角色→世界→名词→剧情→章节）
- 角色卡有 15+ 字段（动机/欲望/恐惧/缺陷/弧线/关系/锁定设定等丰富维度）
- 世界规则有约束等级（弱/普通/强/绝对不可违反）
- AI 写入全流程控制：生成→预览→确认插入/替换/追加/丢弃

#### 文档二：技术架构

**技术栈决策**：
```
Tauri 2.x + React + TypeScript + Rust + SQLite
```

**分层架构**（5 层）：
```
React UI → Tauri Command Bridge → Rust Application Service → Domain → Infrastructure
└── External Adapter（AI Provider 等）
```

**前端技术选型**：
- Vite 构建
- Tailwind CSS + shadcn/ui
- Zustand 状态管理
- TanStack Query 缓存（未在代码中实际使用）
- TipTap/Monaco 编辑器（未明确——实际使用 textarea）

**关键设计决策**：
- Rust 层必须返回标准错误结构 `AppErrorDto`
- AI 流式输出通过 Tauri event 推送
- 前后端通过 `invoke()` + `AppErrorDto` 通信

#### 文档三：数据库与文件协议

**数据设计原则**：
1. 章节正文 = Markdown 文件
2. SQLite = 结构化元信息/索引/关联
3. 项目根目录可整体迁移
4. API Key 不得写入项目目录
5. 所有路径使用相对路径

**标准项目目录结构**：
```
project.json
database/project.sqlite + backups/
manuscript/chapters/ + drafts/ + snapshots/
blueprint/
assets/covers/ + attachments/
exports/ + backups/ + prompts/ + workflows/ + logs/
```

**18 个 SQLite 表的核心设计意图**：
- `projects` — 项目根信息
- `blueprint_steps` — 8 步蓝图内容，支持 not_started/in_progress/completed
- `characters` — 角色卡，`is_deleted` 软删除
- `character_relationships` — 角色关系图谱基础
- `world_rules` — 四档约束等级
- `glossary_terms` — 锁定+禁用双机制
- `plot_nodes` — 剧情骨架排序
- `chapters` — 章节元信息，`content_path` 指向外部文件
- `chapter_links` — 章节与角色/设定/主线的多对多关联
- `consistency_issues` — 检查问题管理
- `ai_requests` — AI 调用审计日志
- `snapshots` — 版本快照
- `schema_migrations` — 数据库版本迁移

**写入协议**：先写临时文件 → 原子替换 → 更新数据库 → 清理 autosave

#### 文档四：AI/Skill/Prompt 系统

**三层上下文系统**（设计精良）：
1. **固定上下文**（Global）：项目元信息 + 蓝图 + 文风约束 + 锁定名词
2. **相关上下文**（Related）：当前章节 + 关联主线 + 出场角色 + 世界规则 + 前后章摘要
3. **动态检索**（Retrieved）：搜索索引 + 关键词匹配，Beta 后引入向量检索

**8 个 Agent** 的分工体系：
- OrchestratorAgent → BlueprintAgent → CharacterAgent → WorldAgent → PlotAgent → ChapterAgent → ReviewAgent → ProseNaturalizerAgent

**12 个内置 Skill**：
- 核心：context.collect（前置）、blueprint.generate_step、character.create、world.create_rule、plot.create_node
- 章节：chapter.plan、chapter.draft、chapter.continue、chapter.rewrite
- 质量：prose.naturalize、consistency.scan
- Beta：import.extract_assets、export.package

**Prompt 统一结构**：角色 → 任务 → 项目上下文 → 相关上下文 → 用户输入 → 约束 → 输出格式

**AI 评价方案**：5 个内置测试项目 × 6 个评价维度

#### 文档五：UI 页面原型

**12 个页面的完整原型设计**：
- 项目中心（左操作区 + 右最近项目）
- 仪表盘（4 统计卡片 + 进度 + 快捷操作 + 近期编辑）
- 蓝图（3 栏：步骤导航 + 表单 + AI 建议面板）
- 角色（列表 + 详情表单）
- 世界（分类树 + 详情）
- 名词（表格）
- 剧情（时间轴/列表切换）
- 章节列表（表格/拖拽排序）
- 编辑器（4 栏：章节树 + Markdown + 上下文面板 + AI 指令栏）
- 一致性检查（问题列表 + 详情）
- 导出（范围/格式/选项）
- 设置（Tabs：模型/编辑器/自动保存/数据备份/关于）

**编辑器布局**是核心 UI 资产：
```
TopBar: 标题/状态/字数/保存/快照/检查
├── 章节树 | Markdown 编辑器 | 上下文面板
└── AI 指令栏：生成草稿/续写/改写/润色/去AI味/自定义
```

**设计原则**：专业、沉浸、本地工程感、低干扰、高信息密度

#### 文档六：开发排期

**12 周 12 Sprint** 设计，但实际代码量远超此范围。关键依赖关系链：
```
项目初始化 → 数据库/文件 → 蓝图/角色/设定/主线 CRUD →
章节服务/编辑器/自动保存 → AI 设置/模型调用 →
上下文组装/章节生成 → 一致性检查/导出 → 测试/打包/验收
```

### 2.3 Blueprint 整体评价

**优势**：
- 设计粒度非常细，从用户旅程到数据库字段全覆盖
- 分层清晰（PRD→架构→数据→AI→UI→排期）
- 所有决策都有明确的理由
- 对 AI Agent 有专门的执行提示词

**问题**：
- 实际代码已经大幅超出文档规划的 MVP 范围
- 文档没有定义 Git/License/VectorSearch/Timeline 等功能
- 缺少测试策略的详细文档

---

## 3. LLM Provider Integration Spec — 模型接入设计

### 3.1 文档定位

这是一份高度专业化的 **LLM 供应商接入技术规范**，面向系统架构师和 Adapter 实现者。

### 3.2 核心设计哲学

**原则 1：不把所有供应商强行当成 OpenAI**
不同供应商之间存在模型名、上下文大小、输出格式、参数命名、流式格式、错误结构等实质性差异。

**原则 2：统一抽象，分供应商适配**
```
LlmService（业务层统一接口）
  ├── OpenAIAdapter（Responses API）
  ├── AnthropicAdapter（Messages API）
  ├── GeminiAdapter（GenerateContent API）
  ├── DeepSeekAdapter（OpenAI-compatible）
  ├── KimiAdapter（OpenAI-compatible）
  ├── ZhipuAdapter（OpenAI-style）
  ├── MiniMaxAdapter（Anthropic-compatible）
  └── CustomOpenAICompatibleAdapter
```

### 3.3 7 个供应商接入详细设计

| 供应商 | 推荐协议 | 推荐默认模型 | 上下文窗口 | 特点 |
|---|---|---|---|---|
| DeepSeek | OpenAI Chat Completions | deepseek-v4-flash | 1,000,000 | V4 支持 Thinking 双模式 |
| Kimi | OpenAI Chat Completions | kimi-k2.6 | 256,000 | 采样参数严格 |
| 智谱 GLM | OpenAI-style Chat Completions | glm-5.1 | 200,000 | 极其丰富的模型生态（15+ 模型） |
| MiniMax | Anthropic-compatible 优先 | MiniMax-M2.7 | 204,800 | 双协议支持 |
| OpenAI | Responses API 优先 | gpt-5.5 | 1,050,000 | 支持 reasoning effort |
| Anthropic | Messages API | claude-sonnet-4-6 | 1,000,000 | 工具/thinking 支持好 |
| Gemini | GenerateContent API | gemini-3.1-pro-preview | 1,000,000 | 多模态能力强 |

### 3.4 统一 Reasoning/Thinking 映射

这是文档中最精巧的设计之一——将 7 个供应商不同的 thinking 参数统一成 5 档：

| 统一值 | OpenAI | DeepSeek | Kimi | 智谱 | Anthropic | Gemini |
|---|---|---|---|---|---|---|
| 关闭 | effort=none | disabled | disabled | disabled | 不传 | minimal |
| 低/中/高 | low/medium/high | low/med/high | 默认/预算 | 开启/预算 | effort 映射 | level 映射 |
| 极高 | xhigh | high+预算 | 高预算 | 高预算 | max | high+预算 |

### 3.5 统一 JSON 输出策略（重要）

不同供应商的 JSON 支持差异很大，文档设计了分级策略：

| 支持级别 | 供应商 | 策略 |
|---|---|---|
| 原生 JSON Schema | OpenAI, Gemini | 直接使用 |
| JSON Object | DeepSeek, Kimi, 智谱 | response_format |
| 无原生支持 | Anthropic, MiniMax | 工具调用 schema / Prompt-only / 本地修复 |

### 3.6 模型注册表热更新

设计了完整的远程注册表方案：
- 文件路径：`resources/model-registry/llm-model-registry.json`
- 远程源：`https://updates.novelforge.app/llm-model-registry.json`
- 安全要求：HTTPS + ed25519 签名校验
- 合并策略：内置 → 远程覆盖 → Provider 实时覆盖 → 用户手动覆盖

### 3.7 三阶段实现规划

| 阶段 | 范围 | 对应代码状态 |
|---|---|---|
| MVP | OpenAI-compatible + DeepSeek + Kimi + 智谱 + 自定义 OpenAI-compatible | 已全部实现 |
| Beta | Anthropic + Gemini + MiniMax + 自定义 Anthropic-compatible + 能力检测 | **Rust 端已实现 Anthropic 和 Gemini Adapter** |
| v1.0 | 价格管理/Tokn 估算/自动路由/降级/限流/Batch API | 未实现，但部分结构已定义 |

### 3.8 统一错误结构

20 种标准化错误码（`missing_api_key` → `unknown`），每种都有用户可理解的提示文案。

---

## 4. 三份文档的关系与一致性

```
AGENTS.md（行为规范）
     │ 约束
     ▼
Blueprint（产品 + 架构 + 数据 + AI + UI + 排期）
     │ 引用
     ▼
LLM Provider Spec（AI 供应商接入细化设计）
```

**一致性检查**：
- AGENTS.md 引用了 Blueprint 和 LLM Spec 作为核心参考文档 ✓
- Blueprint 的 AI 系统设计部分与 LLM Spec 的接口定义一致 ✓
- AGENTS.md 的 MVP 边界与 Blueprint 的 PRD 一致（但代码已超范围） ⚠️
- Blueprint 的架构文档与代码中的 5 层结构一致 ✓

**关键不一致**：
- Blueprint 说 DOCX/PDF/EPUB 导出是 Beta，代码已实现 ✗
- Blueprint 说 Git 是 Beta，代码已实现 ✗
- LLM Spec 说 Anthropic/Gemini 是 Beta，Rust 端已实现 ✗
- AGENTS.md 要求"仅改请求范围内内容"，但实际存在明显的范围膨胀 ✗

---

## 5. 文档质量总评

| 维度 | 评分 | 说明 |
|---|---|---|
| 完整性 | ★★★★★ | 从 PRD 到数据库字段全覆盖 |
| 一致性 | ★★★★☆ | 三份文档之间整体一致，但与代码有偏差 |
| 可执行性 | ★★★★★ | 每个模块都有明确的验收标准和执行提示词 |
| 专业性 | ★★★★★ | 架构分层、LLM 映射、失败模式全覆盖 |
| 时效性 | ★★★☆☆ | 文档 vs 代码存在范围漂移，需要同步更新 |
