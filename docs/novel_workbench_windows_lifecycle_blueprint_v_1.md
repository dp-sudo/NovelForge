# 文火 NovelForge：Windows 本地优先 AI 长篇小说创作平台正式开发文档包 v1.0

> 本文档由原《新一代 Windows 本地优先 AI 长篇小说创作平台：全生命周期开发文档包 v1.0》拆分细化而来。  
> 目标读者：产品负责人、UI/UX 设计师、前端工程师、Rust/Tauri 工程师、AI Agent 工程师、测试工程师、以及可执行开发任务的 LLM Agent。  
> 产品名：**文火 NovelForge**  
> 产品形态：Windows 桌面软件  
> 核心定位：本地优先的 AI 长篇小说策划、写作、一致性治理与作品资产管理平台。

---

# 0. 文档包使用说明

## 0.1 六份正式文档

本项目拆分为以下 6 份可执行正式文档：

1. **《MVP 产品需求文档 PRD》**
2. **《Windows 桌面端技术架构设计》**
3. **《数据库与本地文件协议》**
4. **《AI Agent / Skill / Prompt 系统设计》**
5. **《UI 页面原型说明文档》**
6. **《开发任务排期与人力配置表》**

## 0.2 面向 LLM Agent 的执行原则

所有后续让 LLM Agent 执行开发时，应遵守以下原则：

1. **不得照搬 Moho 墨火的品牌、UI、文案、图标、源码、资源文件或模块命名。**
2. **必须以本文档定义的“文火 NovelForge”独立产品结构开发。**
3. **所有本地数据默认归用户所有，项目必须可迁移、可备份、可导出。**
4. **MVP 只实现主闭环，不提前做复杂插件市场、云同步、多人协作。**
5. **AI 所有写入行为必须可预览、可撤销、可保存版本。**
6. **章节正文不得只存数据库，必须有本地 Markdown 文件承载。**
7. **所有关键操作必须有错误处理、日志记录、用户可理解的失败提示。**
8. **LLM Agent 编码时必须优先保证数据安全、项目可恢复和可测试性。**

---

# 文档一：《MVP 产品需求文档 PRD》

## 1. 文档信息

| 项目 | 内容 |
|---|---|
| 文档名称 | MVP 产品需求文档 PRD |
| 产品名称 | 文火 NovelForge |
| 目标版本 | MVP v0.1 |
| 平台 | Windows 10 / Windows 11 桌面端 |
| 产品形态 | 本地优先桌面软件 |
| 主要用户 | 中文长篇小说作者、网文作者、剧本创作者、IP 设定创作者 |
| MVP 目标 | 完成从新建项目、搭建创作蓝图、创建角色设定、规划章节、AI 生成章节草稿、保存、检查、导出的最小闭环 |

## 2. 产品愿景

文火 NovelForge 的愿景是成为作者的 **本地 AI 小说创作 IDE**。它不是一个简单的 AI 聊天窗口，而是一个围绕长期作品工程建立的创作工作台。

核心口号：

> 让灵感成为工程，让故事稳定完稿。

## 3. MVP 核心目标

MVP 版本只验证产品主闭环是否成立：

1. 用户可以创建一个本地小说项目。
2. 用户可以完成基础创作蓝图。
3. 用户可以创建角色、设定、主线节点。
4. 用户可以创建章节并写作。
5. 用户可以调用 AI 生成章节草稿或续写。
6. 系统可以基于已有角色、设定、主线自动组装上下文。
7. 系统可以进行基础一致性检查。
8. 用户可以导出 TXT / Markdown。
9. 关闭软件后再次打开，项目数据完整恢复。

## 4. MVP 非目标

以下内容不进入 MVP：

1. 多人协作。
2. 云同步。
3. 插件市场。
4. Prompt 模板市场。
5. 完整商业支付系统。
6. 团队权限管理。
7. 移动端。
8. Web 端。
9. 复杂 3D 或高级图谱交互。
10. 全自动写完整本小说。

## 5. 用户画像

### 5.1 个人长篇小说作者

特点：
- 有大量灵感，但缺少结构化整理能力。
- 可能同时维护世界观、角色卡、章节稿。
- 经常遇到“写到中后期设定崩坏”的问题。

核心需求：
- 把灵感、设定、角色、章节沉淀在一个本地项目中。
- AI 能理解当前项目上下文。
- 写作过程可保存、可恢复、可导出。

### 5.2 连载型网文作者

特点：
- 需要高频更新。
- 关注章节节奏、爽点、冲突、伏笔回收。
- 需要快速生成初稿、续写、润色、检查。

核心需求：
- 快速生成章节草稿。
- 快速回顾前文摘要与角色设定。
- 检查命名、设定、角色行为是否前后矛盾。

### 5.3 剧本 / 游戏叙事创作者

特点：
- 重视角色关系、事件线、世界规则。
- 可能有多个故事线和势力组织。

核心需求：
- 结构化维护角色、世界观、剧情节点。
- 支持导出设定集和章节草案。

## 6. MVP 用户旅程

### 6.1 首次启动旅程

1. 用户安装并打开文火 NovelForge。
2. 进入项目中心。
3. 点击「新建作品工程」。
4. 输入作品名称、作者名、类型、目标字数。
5. 选择本地保存目录。
6. 系统创建项目文件夹与数据库。
7. 进入项目仪表盘。
8. 系统提示用户完成创作蓝图。

### 6.2 创作蓝图旅程

1. 用户进入「蓝图」。
2. 按步骤填写灵感定锚、类型策略、故事母题、角色工坊、世界规则、名词锁定、剧情骨架、章节路线。
3. 每一步可以手写，也可以让 AI 生成建议。
4. 用户确认后保存到项目资产库。
5. 完成后仪表盘显示蓝图完成度。

### 6.3 章节写作旅程

1. 用户进入「章节」。
2. 创建卷、章或单章。
3. 设置章节标题、目标字数、章节摘要、关联主线节点、出场角色。
4. 进入章节编辑器。
5. 用户手写正文或点击「生成草稿」。
6. 系统自动组装上下文并调用 AI。
7. AI 流式输出草稿。
8. 用户选择插入、替换、追加或丢弃。
9. 系统自动保存 Markdown 文件和数据库元信息。

### 6.4 一致性检查旅程

1. 用户在章节编辑器或检查中心点击「检查」。
2. 选择检查范围：当前章 / 多章 / 全书。
3. 系统执行基础检查：命名、角色、世界规则、时间线、文风。
4. 输出问题列表。
5. 用户点击问题定位原文。
6. 用户采纳或忽略修复建议。

### 6.5 导出旅程

1. 用户进入「导出」。
2. 选择导出范围：单章 / 多章 / 全书。
3. 选择格式：TXT / Markdown。
4. 选择保存位置。
5. 系统生成导出文件。
6. 导出完成后提供打开文件夹按钮。

## 7. MVP 功能清单

### 7.1 P0 必须实现

| 模块 | 功能 | 说明 |
|---|---|---|
| 项目中心 | 新建项目 | 创建本地项目目录、project.json、SQLite 数据库 |
| 项目中心 | 打开项目 | 选择本地项目目录并加载 |
| 项目中心 | 最近项目 | 本地记录最近打开的项目路径 |
| 项目仪表盘 | 数据概览 | 显示字数、章节数、角色数、设定数、检查问题数 |
| 蓝图 | 8 步基础表单 | 支持手写保存 |
| 蓝图 | AI 生成建议 | 至少支持故事母题、角色、剧情骨架生成 |
| 角色 | 新建 / 编辑 / 删除角色 | 支持姓名、别名、动机、缺陷、目标、关系备注 |
| 世界 | 新建 / 编辑 / 删除设定 | 支持规则、地点、组织、术语 |
| 名词库 | 新建 / 编辑 / 锁定名词 | 支持人名、地名、术语、别名、禁用词 |
| 剧情 | 主线节点管理 | 支持标题、目标、冲突、顺序、状态 |
| 章节 | 章节列表 | 新建、重命名、删除、排序 |
| 编辑器 | Markdown 编辑 | 支持正文输入、自动保存、字数统计 |
| 编辑器 | AI 章节草稿 | 根据上下文生成章节正文 |
| 编辑器 | AI 续写 / 改写 | 基于选中文本或光标位置操作 |
| 检查 | 基础一致性检查 | 命名、角色、设定、禁用词、AI 腔基础检查 |
| 设置 | 模型配置 | 支持 OpenAI-compatible API 基础配置 |
| 导出 | TXT / Markdown 导出 | 支持单章和全书导出 |
| 数据安全 | 自动保存 | 编辑器 5 秒内自动保存草稿 |
| 数据安全 | 启动恢复 | 检查未保存草稿并提示恢复 |

### 7.2 P1 应尽量实现

| 模块 | 功能 | 说明 |
|---|---|---|
| 导入 | TXT / Markdown 导入 | 批量导入旧章节 |
| 版本 | 手动快照 | 用户点击保存当前章节版本 |
| 搜索 | 全局搜索 | 搜索章节、角色、设定、名词 |
| 检查 | 问题状态 | 未处理 / 已忽略 / 已修复 |
| AI | Prompt 预览 | 开发模式下可查看上下文组装结果 |
| 编辑器 | 局部插入策略 | 插入到光标、替换选区、追加到结尾 |

### 7.3 P2 暂缓

| 模块 | 功能 | 说明 |
|---|---|---|
| 图谱 | 高级关系图谱 | Beta 再做 |
| Git | Git 版本管理 | Beta 再做 |
| 授权 | 商业激活 | Beta 再做 |
| 导出 | DOCX / PDF / EPUB | Beta / v1.0 再做 |
| 插件 | 自定义 Skill 插件 | v1.0 再做 |

## 8. 详细需求说明

### 8.1 项目中心

#### 8.1.1 新建项目

输入字段：
- 作品名称，必填，1-80 字。
- 作者名，可选。
- 类型，必填，可选：玄幻、都市、科幻、悬疑、言情、历史、奇幻、轻小说、剧本、其他。
- 目标字数，可选，默认 300000。
- 保存目录，必填。

系统行为：
- 在保存目录下创建项目根目录。
- 生成 `project.json`。
- 创建 `database/project.sqlite`。
- 创建标准文件夹结构。
- 初始化数据库表。
- 把项目加入最近项目列表。

验收标准：
- 新建成功后自动进入仪表盘。
- 关闭软件后再次打开能在最近项目中看到该项目。
- 项目目录移动后，通过「打开项目」仍可加载。
- 项目名非法字符必须自动替换或提示。

#### 8.1.2 打开项目

输入：项目根目录。

校验：
- 必须存在 `project.json`。
- 必须存在 `database/project.sqlite`。
- 项目版本必须兼容当前应用。

失败提示：
- 不是有效项目目录。
- 项目版本过旧，需要迁移。
- 数据库损坏，请从备份恢复。

### 8.2 项目仪表盘

显示卡片：
- 当前总字数。
- 章节数量。
- 已完成章节数量。
- 角色数量。
- 设定数量。
- 主线节点数量。
- 未解决一致性问题数量。
- 最近编辑章节。

快捷入口：
- 继续写作。
- 完成蓝图。
- 创建角色。
- 创建章节。
- 运行检查。
- 导出作品。

验收标准：
- 所有统计数据来自数据库和章节文件实时计算或缓存。
- 点击卡片能跳转对应页面。

### 8.3 创作蓝图

MVP 实现 8 个步骤，每步支持：
- 手动编辑。
- AI 生成建议。
- 保存。
- 标记完成。
- 重置当前步骤。

8 步字段：

#### Step 1：灵感定锚

字段：
- 核心灵感。
- 核心命题。
- 核心情绪。
- 目标读者。
- 商业卖点。
- 读者期待。

#### Step 2：类型策略

字段：
- 主类型。
- 子类型。
- 叙事视角。
- 文风关键词。
- 节奏类型。
- 禁用风格。

#### Step 3：故事母题

字段：
- 一句话梗概。
- 三段式梗概。
- 开端。
- 中段。
- 高潮。
- 结局方向。

#### Step 4：角色工坊

字段：
- 主角。
- 反派。
- 关键配角。
- 角色关系摘要。
- 角色成长弧线。

#### Step 5：世界规则

字段：
- 世界背景。
- 能力 / 技术 / 制度规则。
- 地点。
- 组织。
- 不可违反规则。

#### Step 6：名词锁定

字段：
- 人名。
- 地名。
- 组织名。
- 术语。
- 别名。
- 禁用名词。

#### Step 7：剧情骨架

字段：
- 主线目标。
- 阶段节点。
- 关键冲突。
- 反转。
- 高潮。
- 结局。

#### Step 8：章节路线

字段：
- 卷结构。
- 章节列表。
- 章节目标。
- 出场人物。
- 关联主线节点。

### 8.4 角色工坊

角色卡字段：
- 姓名。
- 别名。
- 角色类型：主角、反派、配角、路人、组织角色。
- 年龄。
- 性别，可选。
- 身份。
- 外貌关键词。
- 核心动机。
- 欲望。
- 恐惧。
- 缺陷。
- 成长弧线。
- 与其他角色关系。
- 不可改变设定。
- 备注。

验收标准：
- 可增删改查。
- 可被章节关联。
- 可被 AI 上下文检索。
- 删除角色前，如果已有章节引用，需要提示。

### 8.5 世界观设定库

设定类型：
- 世界规则。
- 地点。
- 组织。
- 道具。
- 能力。
- 历史事件。
- 术语。

字段：
- 标题。
- 类型。
- 描述。
- 约束等级：弱设定、普通设定、强约束、绝对不可违反。
- 相关角色。
- 相关章节。
- 示例。
- 备注。

### 8.6 主线 / 支线骨架

MVP 先实现主线节点。

字段：
- 节点标题。
- 节点类型：开端、转折、冲突、失败、胜利、高潮、结局、支线。
- 顺序。
- 剧情目标。
- 冲突。
- 情绪曲线。
- 关联角色。
- 关联章节。
- 状态：未使用、规划中、已写入、需调整。

### 8.7 章节编辑器

#### 8.7.1 布局

左侧：章节树。  
中间：Markdown 编辑器。  
右侧：上下文卡片。  
底部：AI 指令栏。  
顶部：章节标题、字数、保存状态、版本按钮、检查按钮。

#### 8.7.2 编辑能力

- 输入正文。
- 自动保存。
- 手动保存。
- 字数统计。
- 当前章摘要编辑。
- 状态切换：草稿、写作中、待修订、已完成。
- 快捷键：Ctrl+S 保存，Ctrl+F 搜索。

#### 8.7.3 AI 能力

按钮：
- 生成章节草稿。
- 续写。
- 改写选中内容。
- 润色选中内容。
- 去 AI 味。
- 检查当前章。

AI 结果处理：
- 插入到光标。
- 替换选区。
- 追加到章节末尾。
- 复制到剪贴板。
- 丢弃。

### 8.8 一致性检查中心

MVP 检查项：
- 锁定名词是否被误写。
- 禁用词是否出现。
- 角色年龄 / 身份 / 关系是否与角色卡冲突。
- 世界规则强约束是否被违反。
- 是否出现未登记的重要新角色 / 地点 / 组织。
- 是否存在明显 AI 腔套话。

问题字段：
- 问题类型。
- 严重程度：低、中、高、阻断。
- 所在章节。
- 原文片段。
- 关联资产。
- 解释。
- 修复建议。
- 状态。

### 8.9 设置

#### 8.9.1 模型配置

字段：
- Provider 名称。
- API Base URL。
- API Key。
- Model。
- Temperature。
- Max Tokens。
- 是否启用流式输出。

MVP 只要求 OpenAI-compatible API。

安全要求：
- API Key 不得明文写入日志。
- API Key 本地加密保存。
- UI 显示时默认隐藏。

### 8.10 导出

MVP 支持：
- 单章 TXT。
- 单章 Markdown。
- 全书 TXT。
- 全书 Markdown。

导出选项：
- 是否包含章节标题。
- 是否包含章节摘要。
- 是否按卷分隔。
- 是否导出设定集。

## 9. 成功指标

MVP 内测阶段建议指标：

| 指标 | 目标 |
|---|---|
| 新建项目成功率 | ≥ 99% |
| 自动保存丢稿率 | 0 严重丢稿 |
| 章节 AI 生成成功率 | ≥ 95% |
| 项目重新打开成功率 | ≥ 99% |
| 导出成功率 | ≥ 98% |
| 30 分钟内完成首章草稿的用户比例 | ≥ 60% |
| 用户认为“上下文有帮助”的比例 | ≥ 70% |

## 10. 验收清单

MVP 完成后必须逐项验收：

1. Windows 可安装、可启动。
2. 可新建本地项目。
3. 可打开已有项目。
4. 可完成 8 步蓝图基础填写。
5. 可创建角色、设定、名词、主线节点。
6. 可创建章节并保存 Markdown 文件。
7. 可调用 AI 生成章节草稿。
8. AI 生成时能读取当前章节关联角色、设定、主线节点。
9. 可进行基础一致性检查。
10. 可导出 TXT / Markdown。
11. 软件崩溃或关闭后，内容可恢复。
12. API Key 不出现在日志中。
13. 删除关键资产时有引用提示。
14. 所有 P0 功能都有基础测试。

## 11. 给 LLM Agent 的 PRD 执行提示词

```text
你是文火 NovelForge 的产品实现 Agent。请严格按照《MVP 产品需求文档 PRD》实现功能，不要擅自扩展云同步、多人协作、插件市场等非 MVP 功能。当前目标是构建最小可用闭环：新建项目 → 创作蓝图 → 角色/设定/主线 → 章节写作 → AI 生成 → 一致性检查 → 导出 → 重新打开恢复。实现时必须保证本地数据安全、自动保存、错误提示和可测试性。每完成一个模块，输出：实现文件、核心逻辑、测试方式、未完成风险。
```

---

# 文档二：《Windows 桌面端技术架构设计》

## 1. 文档信息

| 项目 | 内容 |
|---|---|
| 文档名称 | Windows 桌面端技术架构设计 |
| 产品名称 | 文火 NovelForge |
| 目标版本 | MVP v0.1 → v1.0 可演进 |
| 运行平台 | Windows 10 / Windows 11 |
| 推荐技术栈 | Tauri + React + TypeScript + Rust + SQLite |
| 架构原则 | 本地优先、分层清晰、数据可迁移、AI 可替换、UI 可迭代 |

## 2. 技术选型

### 2.1 桌面框架

推荐：Tauri + React + TypeScript + Rust。

原因：
- 安装包相对轻量。
- Rust 适合本地文件、数据库、加密、导出、后台任务。
- React 适合复杂工作台 UI。
- Tauri Command 机制适合前端调用本地能力。

### 2.2 前端技术

| 技术 | 用途 |
|---|---|
| React | UI 构建 |
| TypeScript | 类型安全 |
| Vite | 前端构建 |
| Tailwind CSS | 样式系统 |
| shadcn/ui | 基础组件 |
| Zustand | 客户端状态管理 |
| TanStack Query | 异步数据与缓存 |
| TipTap 或 Monaco | 章节编辑器 |
| React Flow | 图谱与节点关系，MVP 可暂缓 |
| Recharts | 仪表盘图表，MVP 可轻量使用 |

### 2.3 后端本地能力

| 技术 | 用途 |
|---|---|
| Rust | 本地服务层 |
| rusqlite / sqlx | SQLite 数据访问 |
| serde | JSON 序列化 |
| tokio | 异步任务 |
| reqwest | AI HTTP API 调用 |
| keyring / Windows Credential Manager | API Key 安全保存 |
| notify | 文件变化监听，Beta 可加入 |
| zip | 项目备份包 |
| pulldown-cmark | Markdown 处理 |

### 2.4 数据存储策略

- SQLite：结构化数据、索引、任务、问题、元信息。
- Markdown：章节正文、蓝图正文、可读设定文档。
- JSON：配置、工作流、Prompt 模板、项目元信息。
- 本地加密存储：API Key 与授权信息。
- 搜索索引：MVP 使用 SQLite FTS5，后续可切换 Tantivy。

## 3. 总体架构

```text
┌────────────────────────────────────────────────────┐
│                   React UI 层                       │
│ 页面 / 组件 / 编辑器 / 状态管理 / Query Cache         │
├────────────────────────────────────────────────────┤
│                Tauri Command Bridge                 │
│ 前端 invoke() 调用 Rust 命令，Rust event 推送流式结果  │
├────────────────────────────────────────────────────┤
│                 Rust Application 层                 │
│ ProjectService / ChapterService / AiService 等       │
├────────────────────────────────────────────────────┤
│                 Domain 领域层                       │
│ Project / Chapter / Character / WorldRule / Issue    │
├────────────────────────────────────────────────────┤
│                 Infrastructure 层                   │
│ SQLite / FileSystem / Crypto / Export / Logger       │
├────────────────────────────────────────────────────┤
│                 External Adapter 层                 │
│ OpenAI-compatible / Ollama / LM Studio / Custom API   │
└────────────────────────────────────────────────────┘
```

## 4. 代码仓库结构

```text
novelforge/
  package.json
  pnpm-lock.yaml
  vite.config.ts
  tsconfig.json
  src/
    app/
      App.tsx
      router.tsx
      providers.tsx
    pages/
      ProjectCenter/
      Dashboard/
      Blueprint/
      Characters/
      World/
      Plot/
      Chapters/
      Editor/
      Consistency/
      Export/
      Settings/
    components/
      layout/
      editor/
      forms/
      cards/
      dialogs/
      ai/
    stores/
      projectStore.ts
      uiStore.ts
      editorStore.ts
    api/
      tauriClient.ts
      projectApi.ts
      chapterApi.ts
      aiApi.ts
    types/
      project.ts
      chapter.ts
      character.ts
      ai.ts
    styles/
      globals.css
  src-tauri/
    Cargo.toml
    tauri.conf.json
    src/
      main.rs
      commands/
        project_commands.rs
        chapter_commands.rs
        blueprint_commands.rs
        character_commands.rs
        world_commands.rs
        plot_commands.rs
        ai_commands.rs
        consistency_commands.rs
        export_commands.rs
        settings_commands.rs
      services/
        project_service.rs
        chapter_service.rs
        blueprint_service.rs
        ai_service.rs
        context_service.rs
        consistency_service.rs
        export_service.rs
        backup_service.rs
        settings_service.rs
      domain/
        project.rs
        chapter.rs
        character.rs
        world_rule.rs
        plot_node.rs
        issue.rs
      infra/
        db.rs
        migrations.rs
        fs.rs
        crypto.rs
        logger.rs
        markdown.rs
        search.rs
      adapters/
        ai_provider.rs
        openai_compatible.rs
        ollama.rs
      errors.rs
      state.rs
    migrations/
      0001_init.sql
      0002_fts.sql
```

## 5. 模块边界

### 5.1 UI 层职责

负责：
- 页面布局。
- 表单交互。
- 编辑器状态。
- 调用 Tauri 命令。
- 展示错误和加载状态。
- 接收 AI 流式输出事件。

不负责：
- 直接读写本地文件。
- 直接操作 SQLite。
- 保存 API Key 明文。
- 拼接复杂 AI Prompt。

### 5.2 Tauri Command 层职责

负责：
- 暴露稳定接口给前端。
- 参数校验。
- 调用 Service。
- 转换错误为前端可理解结构。

示例命令：

```rust
#[tauri::command]
async fn create_project(input: CreateProjectInput, state: State<'_, AppState>) -> Result<ProjectDto, AppError>;
```

### 5.3 Service 层职责

负责：
- 业务流程编排。
- 数据一致性。
- 数据库事务。
- 文件与数据库双写策略。
- AI 上下文组装。
- 导出与备份。

### 5.4 Domain 层职责

负责：
- 核心实体定义。
- 枚举与状态机。
- 领域校验。
- 与 UI 无关的业务规则。

### 5.5 Infrastructure 层职责

负责：
- SQLite。
- 文件系统。
- 加密。
- 日志。
- 搜索索引。
- Markdown 解析。

### 5.6 Adapter 层职责

负责：
- 不同 AI Provider 的请求格式转换。
- 流式响应解析。
- 错误标准化。

## 6. 前后端通信协议

### 6.1 Tauri invoke 调用规范

前端统一通过 `src/api/tauriClient.ts` 调用。

```ts
export async function invokeCommand<TInput, TOutput>(
  command: string,
  input?: TInput
): Promise<TOutput> {
  try {
    return await invoke<TOutput>(command, input as any);
  } catch (error) {
    throw normalizeTauriError(error);
  }
}
```

### 6.2 标准错误结构

```ts
export interface AppErrorDto {
  code: string;
  message: string;
  detail?: string;
  recoverable: boolean;
  suggestedAction?: string;
}
```

错误码示例：

| 错误码 | 含义 |
|---|---|
| PROJECT_INVALID_PATH | 无效项目路径 |
| PROJECT_VERSION_UNSUPPORTED | 项目版本不兼容 |
| DB_OPEN_FAILED | 数据库打开失败 |
| FILE_WRITE_FAILED | 文件写入失败 |
| AI_PROVIDER_NOT_CONFIGURED | 未配置模型 |
| AI_REQUEST_FAILED | AI 请求失败 |
| AI_STREAM_INTERRUPTED | AI 流式输出中断 |
| EXPORT_FAILED | 导出失败 |
| SECRET_SAVE_FAILED | 密钥保存失败 |

### 6.3 AI 流式输出事件

Rust 通过 Tauri event 推送：

```ts
interface AiStreamEvent {
  requestId: string;
  type: 'start' | 'delta' | 'done' | 'error';
  delta?: string;
  error?: AppErrorDto;
  metadata?: Record<string, unknown>;
}
```

事件名：

```text
ai://stream
```

前端接收后按 `requestId` 合并文本。

## 7. 核心服务设计

### 7.1 ProjectService

方法：
- create_project(input)
- open_project(path)
- validate_project(path)
- list_recent_projects()
- update_project_metadata(input)
- get_dashboard_stats(project_id)

关键要求：
- 创建项目时文件夹、数据库、project.json 必须保持一致。
- 任一步失败要回滚已创建内容，或标记为可修复状态。

### 7.2 ChapterService

方法：
- create_chapter(input)
- update_chapter_metadata(input)
- read_chapter_content(chapter_id)
- save_chapter_content(chapter_id, content)
- autosave_draft(chapter_id, content)
- recover_draft(chapter_id)
- reorder_chapters(input)
- delete_chapter(chapter_id)

关键要求：
- 正文写入 Markdown 文件。
- 数据库保存章节元信息。
- 自动保存草稿与正式保存分离。

### 7.3 ContextService

方法：
- collect_chapter_context(chapter_id, user_instruction)
- collect_selection_context(chapter_id, selected_text)
- build_prompt_context(task_type, context)

上下文来源：
- Project。
- Blueprint。
- Chapter。
- Character。
- WorldRule。
- PlotNode。
- Glossary。
- Previous chapter summary。

### 7.4 AiService

方法：
- test_provider(config)
- generate_blueprint_step(input)
- generate_chapter_draft(input)
- continue_chapter(input)
- rewrite_selection(input)
- deai_text(input)
- scan_consistency(input)

要求：
- 支持流式输出。
- 支持取消请求。
- 支持超时。
- 失败时返回标准错误。
- 不得把 API Key 打到日志中。

### 7.5 ConsistencyService

方法：
- scan_chapter(chapter_id)
- scan_range(chapter_ids)
- create_issue(issue)
- update_issue_status(issue_id, status)
- resolve_issue(issue_id)

MVP 可混合使用规则检查 + AI 检查。

### 7.6 ExportService

方法：
- export_chapter_txt(chapter_id, path)
- export_chapter_md(chapter_id, path)
- export_book_txt(project_id, path)
- export_book_md(project_id, path)

要求：
- 导出前按章节顺序读取。
- 失败不能破坏原项目。
- 导出完成返回文件路径。

## 8. 状态管理设计

### 8.1 前端 Store

`projectStore`：
- currentProject
- recentProjects
- dashboardStats

`editorStore`：
- activeChapterId
- editorContent
- saveStatus
- lastSavedAt
- isDirty
- aiStreamingState

`uiStore`：
- theme
- sidebarCollapsed
- activeRoute
- modalState

### 8.2 Query 缓存

使用 TanStack Query 管理：
- 项目统计。
- 角色列表。
- 设定列表。
- 主线节点。
- 章节列表。
- 一致性问题。

## 9. 自动保存与恢复架构

### 9.1 自动保存策略

- 编辑器内容变化后 debounce 5 秒保存草稿。
- 用户 Ctrl+S 或点击保存时写入正式正文文件。
- 正式保存后清理对应草稿。

### 9.2 恢复策略

启动或打开章节时：
1. 检查是否存在比正式文件更新的草稿。
2. 如果存在，提示用户：恢复草稿 / 查看差异 / 丢弃。
3. 恢复后不立即覆盖正式文件，需用户确认保存。

## 10. 安全架构

### 10.1 API Key 存储

优先级：
1. Windows Credential Manager。
2. 本地加密文件。
3. 禁止明文 project.json。

### 10.2 日志脱敏

必须脱敏：
- API Key。
- Authorization Header。
- 用户完整正文。
- 大段 Prompt。

允许记录：
- 请求 ID。
- Provider 名称。
- 模型名。
- Token 估算。
- 错误码。
- 错误摘要。

## 11. 性能要求

| 场景 | 目标 |
|---|---|
| 冷启动 | ≤ 3 秒进入项目中心 |
| 打开普通项目 | ≤ 2 秒显示仪表盘 |
| 打开 30 万字项目 | ≤ 5 秒可操作 |
| 保存章节 | ≤ 300ms 完成本地写入 |
| 全局搜索 | 10 万字内 ≤ 500ms |
| AI 首 token | 取决于模型，但 UI 必须立即显示等待状态 |

## 12. 测试策略

### 12.1 Rust 单元测试

覆盖：
- 项目创建。
- 数据库迁移。
- 文件写入。
- 自动保存。
- 导出。
- Prompt 上下文组装。

### 12.2 前端测试

覆盖：
- 表单校验。
- 页面跳转。
- 编辑器保存状态。
- AI 流式输出显示。

### 12.3 E2E 测试

使用 Playwright：
- 新建项目。
- 完成蓝图。
- 创建章节。
- 保存正文。
- 导出 Markdown。

## 13. 构建与发布

### 13.1 开发命令

```bash
pnpm install
pnpm tauri dev
```

### 13.2 构建命令

```bash
pnpm tauri build
```

### 13.3 发布产物

- Windows 安装包。
- 便携版可选。
- 更新日志。
- 校验 Hash。

## 14. 给 LLM Agent 的架构执行提示词

```text
你是文火 NovelForge 的桌面端架构实现 Agent。请按照《Windows 桌面端技术架构设计》创建 Tauri + React + TypeScript + Rust 项目结构。严格区分 UI 层、Tauri Command 层、Service 层、Domain 层、Infrastructure 层和 AI Adapter 层。前端不得直接读写文件或数据库。所有 Rust 命令必须返回标准错误结构。优先实现 ProjectService、ChapterService、SettingsService、AiService、ExportService 的可运行骨架，并为每个核心服务写基础测试。
```

---

# 文档三：《数据库与本地文件协议》

## 1. 文档信息

| 项目 | 内容 |
|---|---|
| 文档名称 | 数据库与本地文件协议 |
| 产品名称 | 文火 NovelForge |
| 数据原则 | 本地优先、可读、可迁移、可备份、可恢复 |
| 数据库 | SQLite |
| 正文格式 | Markdown |
| 配置格式 | JSON / YAML |

## 2. 数据设计原则

1. **章节正文必须存为 Markdown 文件。**
2. **SQLite 只保存结构化元信息、索引、关联、状态。**
3. **项目根目录必须可整体迁移到其他电脑。**
4. **API Key 不得写入项目目录。**
5. **所有文件路径使用相对路径。**
6. **数据库变更必须有迁移脚本。**
7. **写入正文与更新数据库必须尽量事务化。**
8. **删除操作必须软删除优先，重要资产删除前检查引用。**

## 3. 项目根目录协议

标准项目目录：

```text
novelforge-project/
  project.json
  database/
    project.sqlite
    backups/
  manuscript/
    chapters/
      ch-0001.md
      ch-0002.md
    drafts/
      ch-0001.autosave.md
    snapshots/
      ch-0001/
        2026-04-26T120000.md
  blueprint/
    step-01-anchor.md
    step-02-genre.md
    step-03-premise.md
    step-04-characters.md
    step-05-world.md
    step-06-glossary.md
    step-07-plot.md
    step-08-chapters.md
  assets/
    covers/
    attachments/
  exports/
  backups/
  prompts/
  workflows/
  logs/
```

## 4. project.json 协议

文件：`project.json`

```json
{
  "schemaVersion": "1.0.0",
  "appMinVersion": "0.1.0",
  "projectId": "uuid",
  "name": "示例小说",
  "author": "作者名",
  "genre": "玄幻",
  "targetWords": 300000,
  "createdAt": "2026-04-26T12:00:00+08:00",
  "updatedAt": "2026-04-26T12:00:00+08:00",
  "database": "database/project.sqlite",
  "manuscriptRoot": "manuscript/chapters",
  "settings": {
    "defaultNarrativePov": "third_limited",
    "language": "zh-CN",
    "autosaveIntervalMs": 5000
  }
}
```

要求：
- `projectId` 创建后不可改变。
- `schemaVersion` 用于迁移。
- 路径全部相对项目根目录。

## 5. Markdown 章节文件协议

文件示例：`manuscript/chapters/ch-0001.md`

```markdown
---
id: ch_0001
index: 1
title: 第一章 风起
status: drafting
summary: 主角第一次发现异常。
wordCount: 3200
createdAt: 2026-04-26T12:00:00+08:00
updatedAt: 2026-04-26T13:00:00+08:00
linkedPlotNodes:
  - plot_001
appearingCharacters:
  - char_001
linkedWorldRules:
  - rule_001
---

# 第一章 风起

正文从这里开始。
```

要求：
- Frontmatter 与数据库元信息保持同步。
- 正文以 Markdown 保存。
- 如果 Frontmatter 与数据库冲突，以数据库为主，但应提示修复。

## 6. SQLite 数据库表结构

### 6.1 projects

```sql
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  author TEXT,
  genre TEXT,
  target_words INTEGER DEFAULT 0,
  current_words INTEGER DEFAULT 0,
  narrative_pov TEXT,
  style_tags TEXT,
  project_path TEXT NOT NULL,
  schema_version TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### 6.2 blueprint_steps

```sql
CREATE TABLE blueprint_steps (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  step_key TEXT NOT NULL,
  title TEXT NOT NULL,
  content TEXT,
  content_path TEXT,
  status TEXT NOT NULL DEFAULT 'not_started',
  ai_generated INTEGER NOT NULL DEFAULT 0,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, step_key),
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
```

`status` 可选：
- not_started
- in_progress
- completed

### 6.3 characters

```sql
CREATE TABLE characters (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  name TEXT NOT NULL,
  aliases TEXT,
  role_type TEXT NOT NULL,
  age TEXT,
  gender TEXT,
  identity_text TEXT,
  appearance TEXT,
  motivation TEXT,
  desire TEXT,
  fear TEXT,
  flaw TEXT,
  arc_stage TEXT,
  locked_fields TEXT,
  notes TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
```

### 6.4 character_relationships

```sql
CREATE TABLE character_relationships (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  source_character_id TEXT NOT NULL,
  target_character_id TEXT NOT NULL,
  relationship_type TEXT NOT NULL,
  description TEXT,
  status TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id),
  FOREIGN KEY(source_character_id) REFERENCES characters(id),
  FOREIGN KEY(target_character_id) REFERENCES characters(id)
);
```

### 6.5 world_rules

```sql
CREATE TABLE world_rules (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  category TEXT NOT NULL,
  description TEXT NOT NULL,
  constraint_level TEXT NOT NULL DEFAULT 'normal',
  related_entities TEXT,
  examples TEXT,
  contradiction_policy TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
```

`constraint_level`：
- weak
- normal
- strong
- absolute

### 6.6 glossary_terms

```sql
CREATE TABLE glossary_terms (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  term TEXT NOT NULL,
  term_type TEXT NOT NULL,
  aliases TEXT,
  description TEXT,
  locked INTEGER NOT NULL DEFAULT 0,
  banned INTEGER NOT NULL DEFAULT 0,
  preferred_usage TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, term),
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
```

### 6.7 plot_nodes

```sql
CREATE TABLE plot_nodes (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  node_type TEXT NOT NULL,
  sort_order INTEGER NOT NULL,
  goal TEXT,
  conflict TEXT,
  emotional_curve TEXT,
  status TEXT NOT NULL DEFAULT 'planned',
  related_characters TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
```

### 6.8 chapters

```sql
CREATE TABLE chapters (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  volume_id TEXT,
  chapter_index INTEGER NOT NULL,
  title TEXT NOT NULL,
  summary TEXT,
  status TEXT NOT NULL DEFAULT 'drafting',
  target_words INTEGER DEFAULT 0,
  current_words INTEGER DEFAULT 0,
  content_path TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, chapter_index),
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
```

`status`：
- planned
- drafting
- revising
- completed
- archived

### 6.9 chapter_links

```sql
CREATE TABLE chapter_links (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT NOT NULL,
  target_type TEXT NOT NULL,
  target_id TEXT NOT NULL,
  relation_type TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id),
  FOREIGN KEY(chapter_id) REFERENCES chapters(id)
);
```

`target_type`：
- character
- world_rule
- plot_node
- glossary_term
- obligation

### 6.10 narrative_obligations

```sql
CREATE TABLE narrative_obligations (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  obligation_type TEXT NOT NULL,
  description TEXT NOT NULL,
  planted_chapter_id TEXT,
  expected_payoff_chapter_id TEXT,
  actual_payoff_chapter_id TEXT,
  payoff_status TEXT NOT NULL DEFAULT 'open',
  severity TEXT NOT NULL DEFAULT 'medium',
  related_entities TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
```

### 6.11 consistency_issues

```sql
CREATE TABLE consistency_issues (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  issue_type TEXT NOT NULL,
  severity TEXT NOT NULL,
  chapter_id TEXT,
  source_text TEXT,
  source_start INTEGER,
  source_end INTEGER,
  related_asset_type TEXT,
  related_asset_id TEXT,
  explanation TEXT NOT NULL,
  suggested_fix TEXT,
  status TEXT NOT NULL DEFAULT 'open',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id),
  FOREIGN KEY(chapter_id) REFERENCES chapters(id)
);
```

`status`：
- open
- ignored
- fixed
- false_positive

### 6.12 ai_requests

```sql
CREATE TABLE ai_requests (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  task_type TEXT NOT NULL,
  provider TEXT,
  model TEXT,
  prompt_preview TEXT,
  status TEXT NOT NULL,
  error_code TEXT,
  error_message TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
```

注意：
- `prompt_preview` 只保存脱敏后的短摘要，不保存完整正文。

### 6.13 snapshots

```sql
CREATE TABLE snapshots (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  snapshot_type TEXT NOT NULL,
  title TEXT,
  file_path TEXT NOT NULL,
  note TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id),
  FOREIGN KEY(chapter_id) REFERENCES chapters(id)
);
```

### 6.14 settings

```sql
CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

## 7. FTS 搜索表

MVP 使用 SQLite FTS5：

```sql
CREATE VIRTUAL TABLE search_index USING fts5(
  entity_type,
  entity_id,
  title,
  body,
  tokenize = 'unicode61'
);
```

索引实体：
- chapter
- character
- world_rule
- glossary_term
- plot_node

## 8. 数据写入协议

### 8.1 创建章节

流程：
1. 开启数据库事务。
2. 生成章节 ID。
3. 计算 `content_path`。
4. 写入 Markdown 初始文件。
5. 插入 `chapters` 表。
6. 插入关联数据。
7. 更新搜索索引。
8. 提交事务。
9. 若失败，删除已创建的文件或记录待修复。

### 8.2 保存章节正文

流程：
1. 写入临时文件 `ch-0001.md.tmp`。
2. 写入成功后原子替换正式文件。
3. 更新 `chapters.current_words`、`updated_at`、`version`。
4. 更新搜索索引。
5. 清理对应 autosave。

### 8.3 自动保存草稿

流程：
1. 写入 `manuscript/drafts/ch-0001.autosave.md`。
2. 不更新正式 `version`。
3. 记录草稿更新时间。

### 8.4 删除资产

原则：
- 章节、角色、设定优先软删除。
- 如果有引用，必须弹出确认。
- 软删除字段为 `is_deleted = 1`。

## 9. 备份协议

### 9.1 自动备份

触发时机：
- 每日首次打开项目。
- 大版本迁移前。
- 用户手动备份。

备份位置：

```text
backups/
  2026-04-26_120000_novelforge-backup.zip
```

备份内容：
- project.json
- database/project.sqlite
- manuscript/
- blueprint/
- prompts/
- workflows/

不备份：
- API Key。
- 本机授权缓存。
- 临时日志。

### 9.2 恢复协议

恢复步骤：
1. 用户选择 zip 备份。
2. 系统校验 manifest。
3. 选择恢复目录。
4. 解压。
5. 校验 project.json 与数据库。
6. 加入最近项目。

## 10. 迁移协议

每个数据库版本使用独立 SQL 文件：

```text
migrations/
  0001_init.sql
  0002_fts.sql
  0003_add_obligations.sql
```

迁移表：

```sql
CREATE TABLE schema_migrations (
  version TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL
);
```

迁移要求：
- 迁移前自动备份。
- 迁移失败必须回滚。
- 不可静默破坏用户数据。

## 11. 数据一致性检查

启动项目时检查：
- project.json 是否存在。
- SQLite 是否可打开。
- chapters 表中的 content_path 文件是否存在。
- Markdown Frontmatter 是否与数据库基本一致。
- 是否存在孤立的草稿。
- 是否有未完成迁移。

输出：
- 正常。
- 可自动修复。
- 需要用户选择。
- 严重损坏。

## 12. 给 LLM Agent 的数据库执行提示词

```text
你是文火 NovelForge 的数据层实现 Agent。请按照《数据库与本地文件协议》实现 SQLite 迁移脚本、项目目录初始化、Markdown 文件读写、章节保存、自动保存、搜索索引和备份恢复。章节正文必须以 Markdown 文件保存，数据库只保存元信息和关联。所有路径必须使用相对路径。API Key 不得进入项目目录。写入时必须先写临时文件再原子替换。请为创建项目、创建章节、保存章节、打开项目、导出项目写测试。
```

---

# 文档四：《AI Agent / Skill / Prompt 系统设计》

## 1. 文档信息

| 项目 | 内容 |
|---|---|
| 文档名称 | AI Agent / Skill / Prompt 系统设计 |
| 产品名称 | 文火 NovelForge |
| 目标 | 构建可控、可扩展、上下文感知的小说创作 AI 系统 |
| MVP 范围 | OpenAI-compatible 模型调用、上下文组装、章节生成、续写、改写、去 AI 味、基础一致性检查 |

## 2. AI 系统设计原则

1. **AI 是协作者，不是自动替代作者。**
2. **所有 AI 写入必须经过用户确认。**
3. **锁定设定优先级高于 AI 创造性。**
4. **不得无提示改变角色动机、世界规则、主线结局。**
5. **AI 输出必须能追溯调用任务和上下文摘要。**
6. **上下文必须分层组装，避免无脑塞入全项目。**
7. **Prompt 模板与业务代码分离。**
8. **所有结构化输出必须做 JSON Schema 校验。**

## 3. AI 架构总览

```text
用户意图 / UI 操作
        ↓
Task Router 任务路由
        ↓
Context Collector 上下文收集
        ↓
Prompt Builder 提示词构建
        ↓
Model Adapter 模型适配器
        ↓
Stream Parser 流式解析
        ↓
Result Validator 结果校验
        ↓
User Review 用户确认
        ↓
Project Writer 写入项目资产
```

## 4. 三层上下文系统

### 4.1 固定上下文 Global Context

来源：
- 项目元信息。
- 蓝图步骤。
- 文风约束。
- 叙事视角。
- 禁用词。
- 锁定名词。

示例：

```json
{
  "projectName": "长夜行舟",
  "genre": "东方玄幻",
  "pov": "第三人称限制视角",
  "style": ["冷峻", "克制", "画面感"],
  "bannedStyle": ["网络段子腔", "过度解释", "鸡汤式总结"],
  "lockedTerms": ["玄灯司", "夜潮", "白骨渡"]
}
```

### 4.2 相关上下文 Related Context

来源：
- 当前章节。
- 关联主线节点。
- 出场角色。
- 相关世界规则。
- 上一章摘要。
- 下一章目标。

### 4.3 动态检索上下文 Retrieved Context

来源：
- 搜索索引。
- 角色名。
- 地点名。
- 选中文本关键词。
- 前文摘要。

MVP 检索策略：
- 先基于章节关联读取。
- 再使用 SQLite FTS 搜索关键词。
- 后续版本再引入向量检索。

## 5. Agent 设计

### 5.1 主控 Agent：OrchestratorAgent

职责：
- 理解用户任务。
- 判断任务类型。
- 选择 Skill。
- 调用 Context Collector。
- 组织结果返回 UI。

输入：
- 当前页面。
- 当前章节 ID。
- 用户指令。
- 选中文本。

输出：
- 执行计划。
- 调用的 Skill。
- 生成结果。
- 风险提示。

### 5.2 蓝图 Agent：BlueprintAgent

职责：
- 生成灵感定锚。
- 生成类型策略。
- 生成一句话梗概。
- 生成三段式梗概。
- 拆分主线骨架。

### 5.3 角色 Agent：CharacterAgent

职责：
- 创建角色卡。
- 补全角色动机、缺陷、欲望、恐惧。
- 分析角色关系。
- 检查角色行为是否偏离设定。

### 5.4 世界观 Agent：WorldAgent

职责：
- 创建世界规则。
- 创建地点、组织、能力体系。
- 判断章节是否违反强约束设定。

### 5.5 剧情 Agent：PlotAgent

职责：
- 生成主线节点。
- 设计冲突。
- 调整节奏。
- 规划章节路线。

### 5.6 章节 Agent：ChapterAgent

职责：
- 生成章节提纲。
- 生成章节正文草稿。
- 续写。
- 局部改写。
- 章节摘要。

### 5.7 审稿 Agent：ReviewAgent

职责：
- 检查逻辑问题。
- 检查一致性问题。
- 检查节奏问题。
- 检查 AI 腔。

### 5.8 去 AI 味 Agent：ProseNaturalizerAgent

职责：
- 降低模板化表达。
- 减少空泛总结。
- 增强动作、对话、感官细节。
- 保持事实不变。

## 6. Skill 规范

### 6.1 Skill Manifest 格式

```json
{
  "id": "chapter.draft",
  "name": "生成章节草稿",
  "description": "根据章节目标、角色、设定、主线节点生成章节正文草稿",
  "inputSchema": {
    "chapterId": "string",
    "userInstruction": "string",
    "targetWords": "number"
  },
  "outputSchema": {
    "draft": "string",
    "summary": "string",
    "usedContext": "array",
    "risks": "array"
  },
  "requiresUserConfirmation": true,
  "writesToProject": false
}
```

### 6.2 内置 Skill 清单

| Skill ID | 名称 | MVP | 说明 |
|---|---|---|---|
| context.collect | 收集上下文 | 是 | 所有 AI 任务前置 |
| blueprint.generate_step | 生成蓝图步骤 | 是 | 生成单个蓝图步骤建议 |
| character.create | 创建角色卡 | 是 | 根据需求生成角色 |
| world.create_rule | 创建世界规则 | 是 | 生成设定规则 |
| plot.create_node | 创建剧情节点 | 是 | 生成主线节点 |
| chapter.plan | 生成章节计划 | 是 | 标题、摘要、目标、冲突 |
| chapter.draft | 生成章节草稿 | 是 | 生成正文 |
| chapter.continue | 续写章节 | 是 | 根据光标前文续写 |
| chapter.rewrite | 改写选区 | 是 | 不改变事实 |
| prose.naturalize | 去 AI 味 | 是 | 自然化表达 |
| consistency.scan | 一致性扫描 | 是 | 输出问题列表 |
| import.extract_assets | 旧稿抽取资产 | 否 | Beta |
| export.package | 整理导出 | 否 | Beta |

## 7. 工作流设计

### 7.1 章节草稿生成工作流

```yaml
id: workflow.chapter_draft
name: 章节草稿生成
steps:
  - skill: context.collect
    input:
      scope: current_chapter
  - skill: chapter.plan
    input:
      useExistingPlan: true
  - skill: chapter.draft
    input:
      targetWords: 3000
  - skill: consistency.scan
    input:
      mode: draft_preview
  - action: user_review
  - action: insert_or_discard
```

### 7.2 当前章检查工作流

```yaml
id: workflow.chapter_review
name: 当前章一致性检查
steps:
  - skill: context.collect
    input:
      scope: current_chapter
  - skill: consistency.scan
    input:
      checks:
        - glossary
        - character
        - world_rule
        - prose_style
  - action: show_issue_list
```

### 7.3 去 AI 味工作流

```yaml
id: workflow.prose_naturalize
name: 去 AI 味
steps:
  - skill: context.collect
    input:
      scope: selection
  - skill: prose.naturalize
    input:
      preserveFacts: true
      preservePOV: true
  - action: diff_preview
  - action: user_apply
```

## 8. Prompt 模板规范

所有 Prompt 使用统一结构：

```text
# 角色
你是文火 NovelForge 内置的{agent_name}。

# 任务
{task_goal}

# 项目上下文
{global_context}

# 当前相关上下文
{related_context}

# 用户输入
{user_instruction}

# 约束
1. 不得违反锁定名词与强约束设定。
2. 不得擅自改变角色核心动机。
3. 不得引入未确认的新主线设定。
4. 输出必须符合指定格式。

# 输出格式
{output_format}
```

## 9. 关键 Prompt 模板

### 9.1 章节草稿 Prompt

```text
# 角色
你是专业长篇小说章节写作助手，擅长按照既定角色、世界规则和剧情节点生成稳定的章节草稿。

# 任务
根据当前章节目标生成一版章节正文草稿。

# 固定上下文
{global_context}

# 当前章节信息
章节标题：{chapter_title}
章节摘要：{chapter_summary}
目标字数：{target_words}
章节状态：{chapter_status}

# 关联剧情节点
{plot_nodes}

# 出场角色
{characters}

# 相关世界规则
{world_rules}

# 上一章摘要
{previous_chapter_summary}

# 写作要求
{user_instruction}

# 严格约束
1. 不得改写已锁定设定。
2. 不得新增没有铺垫的重大世界规则。
3. 不得让角色做出明显违背动机的行为。
4. 不要使用空泛总结句，例如“这一刻，他明白了命运的重量”。
5. 对话、动作、环境描写要服务于冲突推进。
6. 保持叙事视角一致。

# 输出
请只输出章节正文，不要输出解释。
```

### 9.2 章节计划 Prompt

```text
# 角色
你是长篇小说剧情规划师。

# 任务
为当前章节生成可执行章节计划。

# 输入
项目背景：{global_context}
关联主线节点：{plot_nodes}
出场角色：{characters}
用户要求：{user_instruction}

# 输出 JSON
{
  "title": "章节标题",
  "summary": "章节摘要",
  "sceneBeats": ["场景节拍1", "场景节拍2"],
  "conflict": "本章核心冲突",
  "characterProgress": "角色推进",
  "foreshadowing": ["可埋伏笔"],
  "risks": ["潜在风险"]
}
```

### 9.3 角色卡 Prompt

```text
# 角色
你是小说角色设计师。

# 任务
根据用户设想创建结构化角色卡。

# 项目上下文
{global_context}

# 用户设想
{user_instruction}

# 输出 JSON
{
  "name": "角色名",
  "aliases": [],
  "roleType": "主角/反派/配角/路人",
  "identity": "身份",
  "motivation": "核心动机",
  "desire": "欲望",
  "fear": "恐惧",
  "flaw": "缺陷",
  "arc": "成长弧线",
  "relationships": [],
  "lockedFacts": [],
  "notes": "备注"
}
```

### 9.4 一致性检查 Prompt

```text
# 角色
你是长篇小说一致性审稿员。

# 任务
检查当前章节是否违反角色、名词、世界规则、时间线或文风约束。

# 已锁定名词
{glossary}

# 角色卡
{characters}

# 世界规则
{world_rules}

# 当前章节正文
{chapter_content}

# 检查维度
1. 名词误写或别名误用。
2. 角色动机、身份、关系冲突。
3. 世界规则冲突。
4. 新增未登记的重要角色、地点、组织。
5. 明显 AI 腔、套话、空泛总结。

# 输出 JSON
{
  "issues": [
    {
      "issueType": "glossary/character/world_rule/timeline/prose_style",
      "severity": "low/medium/high/blocker",
      "sourceText": "原文片段",
      "explanation": "问题说明",
      "suggestedFix": "修复建议",
      "relatedAsset": "关联资产"
    }
  ]
}
```

### 9.5 去 AI 味 Prompt

```text
# 角色
你是中文小说文本修订编辑，擅长去除模板化 AI 腔，并保持事实不变。

# 任务
改写用户选中的文本，让表达更自然、更具体、更有动作和画面感。

# 原文
{selected_text}

# 相关上下文
{related_context}

# 约束
1. 不改变事实。
2. 不改变人物关系。
3. 不新增重大设定。
4. 不改变叙事视角。
5. 减少空泛感叹和总结。
6. 保留原文核心信息。

# 输出
只输出改写后的文本。
```

## 10. 模型适配器协议

### 10.1 ProviderConfig

```ts
interface ProviderConfig {
  id: string;
  name: string;
  type: 'openai_compatible' | 'ollama' | 'lm_studio' | 'custom';
  baseUrl: string;
  apiKeySecretRef?: string;
  model: string;
  temperature: number;
  maxTokens: number;
  stream: boolean;
}
```

### 10.2 GenerateRequest

```ts
interface GenerateRequest {
  requestId: string;
  taskType: string;
  systemPrompt?: string;
  userPrompt: string;
  temperature?: number;
  maxTokens?: number;
  stream: boolean;
}
```

### 10.3 GenerateResponse

```ts
interface GenerateResponse {
  requestId: string;
  text: string;
  finishReason?: string;
  usage?: {
    promptTokens?: number;
    completionTokens?: number;
    totalTokens?: number;
  };
}
```

## 11. 结果校验

结构化输出必须：
- 尝试解析 JSON。
- 如果解析失败，触发修复 Prompt。
- 修复仍失败，返回原文并提示用户。

章节正文输出：
- 不做强 JSON 校验。
- 需要检查是否为空。
- 需要检查是否明显跑题。
- 需要检查是否包含模型解释性废话，例如“以下是章节正文”。

## 12. AI 写入权限

| 操作 | 是否允许自动写入 | 要求 |
|---|---|---|
| 生成草稿 | 否 | 用户确认插入 |
| 续写 | 否 | 用户确认插入 |
| 改写 | 否 | Diff 预览后应用 |
| 创建角色卡 | 否 | 用户确认保存 |
| 创建设定 | 否 | 用户确认保存 |
| 一致性问题 | 是 | 可自动写入问题列表，但不得改正文 |
| 自动摘要 | P1 可自动 | 需可撤销 |

## 13. AI 评测方案

### 13.1 测试样本

准备 5 个内置测试项目：
- 玄幻。
- 都市。
- 悬疑。
- 科幻。
- 言情。

每个项目包含：
- 项目蓝图。
- 3 个角色。
- 3 条世界规则。
- 3 个主线节点。
- 2 章正文。

### 13.2 评测维度

| 维度 | 评分 |
|---|---|
| 是否遵守角色设定 | 1-5 |
| 是否遵守世界规则 | 1-5 |
| 是否推进章节目标 | 1-5 |
| 是否文风自然 | 1-5 |
| 是否减少 AI 腔 | 1-5 |
| 是否产生无关设定 | 1-5，越少越高 |

## 14. 给 LLM Agent 的 AI 系统执行提示词

```text
你是文火 NovelForge 的 AI 系统实现 Agent。请按照《AI Agent / Skill / Prompt 系统设计》实现 ContextService、PromptBuilder、AiService、ModelAdapter、SkillRegistry 和基础工作流。MVP 只实现 OpenAI-compatible 调用、章节草稿、续写、改写、去 AI 味、蓝图生成和一致性检查。所有 AI 输出不得直接修改用户正文，必须返回预览结果。结构化输出必须 JSON 校验。日志不得保存完整正文和 API Key。
```

---

# 文档五：《UI 页面原型说明文档》

## 1. 文档信息

| 项目 | 内容 |
|---|---|
| 文档名称 | UI 页面原型说明文档 |
| 产品名称 | 文火 NovelForge |
| 设计目标 | 专业、沉浸、本地工程感、低干扰、高信息密度 |
| 目标平台 | Windows 桌面端 |
| MVP 范围 | 项目中心、仪表盘、蓝图、角色、世界、剧情、章节、编辑器、检查、导出、设置 |

## 2. UI 设计原则

1. **不复制 Moho 视觉与布局。**
2. **整体采用小说 IDE 式工作台。**
3. **核心写作区必须安静、低干扰。**
4. **复杂功能用侧栏、抽屉、弹窗分层承载。**
5. **AI 功能必须始终显示来源、状态和可撤销操作。**
6. **所有保存状态必须可见。**
7. **新手模式默认简洁，专家模式显示更多上下文与调试信息。**

## 3. 整体信息架构

主导航：

```text
项目中心
└── 项目工作台
    ├── 仪表盘
    ├── 蓝图
    ├── 角色
    ├── 世界
    ├── 名词
    ├── 剧情
    ├── 章节
    ├── 检查
    ├── 导出
    └── 设置
```

## 4. 全局布局

```text
┌──────────────────────────────────────────────────────┐
│ TopBar：项目名 / 全局搜索 / AI 状态 / 保存状态 / 设置  │
├───────────────┬──────────────────────────────────────┤
│ Sidebar       │ Main Content                         │
│ 仪表盘         │ 当前页面内容                           │
│ 蓝图           │                                      │
│ 角色           │                                      │
│ 世界           │                                      │
│ 名词           │                                      │
│ 剧情           │                                      │
│ 章节           │                                      │
│ 检查           │                                      │
│ 导出           │                                      │
│ 设置           │                                      │
├───────────────┴──────────────────────────────────────┤
│ StatusBar：字数 / 自动保存 / 模型 / 当前任务 / 错误提示 │
└──────────────────────────────────────────────────────┘
```

## 5. 视觉系统

### 5.1 主题

MVP 默认暗色主题，后续支持亮色主题。

建议语义色：
- 背景：深色中性。
- 卡片：略浅于背景。
- 主色：暖橙或琥珀色，体现“文火”。
- 成功：绿色。
- 警告：黄色。
- 错误：红色。
- 信息：蓝色。

### 5.2 字体层级

| 用途 | 建议大小 |
|---|---|
| 页面标题 | 24px |
| 区块标题 | 18px |
| 正文 UI | 14px |
| 辅助说明 | 12px |
| 编辑器正文 | 16-18px 可调 |

### 5.3 组件风格

- 卡片圆角 12px。
- 按钮圆角 8px。
- 表单项高度 36-40px。
- 编辑器行高 1.75。
- 弹窗宽度 480-720px。
- 重要操作按钮右对齐。

## 6. 页面原型

## 6.1 项目中心

### 6.1.1 页面目标

让用户新建、打开、恢复项目。

### 6.1.2 布局

```text
┌────────────────────────────────────────────┐
│ 文火 NovelForge                            │
│ 本地优先 AI 长篇小说创作平台                  │
├──────────────────────┬─────────────────────┤
│ 左侧操作区             │ 右侧最近项目           │
│ [新建作品工程]          │ 项目卡片 1             │
│ [打开本地项目]          │ 项目卡片 2             │
│ [从备份恢复]            │ 项目卡片 3             │
│ [导入旧稿创建项目] P1    │                       │
└──────────────────────┴─────────────────────┘
```

### 6.1.3 交互

- 点击新建作品工程 → 打开新建项目弹窗。
- 点击最近项目 → 校验路径后进入仪表盘。
- 项目不存在 → 显示「移除记录 / 重新定位」。

### 6.1.4 新建项目弹窗字段

- 作品名称。
- 作者名。
- 类型。
- 目标字数。
- 保存路径。

按钮：
- 取消。
- 创建项目。

## 6.2 项目仪表盘

### 6.2.1 页面目标

展示项目状态，并提供快捷入口。

### 6.2.2 布局

```text
┌────────────────────────────────────────────┐
│ 项目仪表盘                                  │
├──────────┬──────────┬──────────┬──────────┤
│ 总字数    │ 章节数    │ 角色数    │ 设定数    │
├──────────┴──────────┴──────────┴──────────┤
│ 创作进度                                   │
│ 蓝图完成度  ███████░░░ 70%                 │
│ 未解决问题  3                              │
│ 未回收伏笔  5                              │
├────────────────────────────────────────────┤
│ 快捷操作：继续写作 / 完成蓝图 / 创建章节 / 检查 │
├────────────────────────────────────────────┤
│ 最近编辑章节列表                            │
└────────────────────────────────────────────┘
```

## 6.3 蓝图页面

### 6.3.1 页面目标

引导用户搭建作品底座。

### 6.3.2 布局

```text
┌──────────────┬──────────────────────┬──────────────┐
│ 8 步流程      │ 当前步骤表单           │ AI 建议面板    │
│ 1 灵感定锚    │ 输入字段               │ [生成建议]     │
│ 2 类型策略    │ 多行文本               │ 建议结果       │
│ 3 故事母题    │ 标签选择               │ [采用] [重试]  │
│ ...          │ [保存] [标记完成]      │ 风险提示       │
└──────────────┴──────────────────────┴──────────────┘
```

### 6.3.3 状态

- 未开始：灰色圆点。
- 进行中：蓝色圆点。
- 已完成：绿色对勾。
- 有风险：黄色提示。

## 6.4 角色页面

### 6.4.1 页面目标

管理角色卡。

### 6.4.2 布局

```text
┌────────────────────────────────────────────┐
│ 角色工坊  [新建角色] [AI 创建角色] [搜索]      │
├──────────────┬─────────────────────────────┤
│ 角色列表       │ 角色详情表单                 │
│ 主角           │ 姓名 / 别名 / 类型            │
│ 反派           │ 动机 / 欲望 / 恐惧 / 缺陷      │
│ 配角           │ 成长弧线 / 关系 / 锁定设定     │
└──────────────┴─────────────────────────────┘
```

### 6.4.3 交互

- 点击角色卡 → 右侧显示详情。
- 新建角色 → 空表单。
- AI 创建角色 → 输入一句设想 → AI 返回结构化角色卡 → 用户确认保存。
- 删除角色 → 如果章节引用，提示风险。

## 6.5 世界页面

### 6.5.1 页面目标

管理世界规则、地点、组织、道具、能力体系。

### 6.5.2 布局

```text
┌────────────────────────────────────────────┐
│ 世界设定库 [新建设定] [AI 生成] [筛选类型]      │
├──────────────┬─────────────────────────────┤
│ 分类树         │ 设定详情                     │
│ 世界规则       │ 标题                         │
│ 地点           │ 类型                         │
│ 组织           │ 约束等级                     │
│ 道具           │ 描述                         │
│ 能力           │ 相关角色 / 章节               │
└──────────────┴─────────────────────────────┘
```

## 6.6 名词页面

### 6.6.1 页面目标

锁定人名、地名、组织名、术语、别名和禁用词。

### 6.6.2 布局

```text
┌────────────────────────────────────────────┐
│ 名词库 [新增名词] [批量导入] [检查误写]        │
├────────────────────────────────────────────┤
│ 表格：名词 / 类型 / 别名 / 是否锁定 / 是否禁用 / 描述 │
└────────────────────────────────────────────┘
```

## 6.7 剧情页面

### 6.7.1 页面目标

管理主线节点。

### 6.7.2 布局

```text
┌────────────────────────────────────────────┐
│ 剧情骨架 [新增节点] [AI 拆分主线]              │
├────────────────────────────────────────────┤
│ 时间轴 / 列表切换                             │
│ 01 开端：主角发现异常                          │
│ 02 冲突：第一次失败                            │
│ 03 转折：发现组织真相                          │
└────────────────────────────────────────────┘
```

节点详情抽屉：
- 标题。
- 类型。
- 顺序。
- 目标。
- 冲突。
- 情绪曲线。
- 关联角色。
- 关联章节。

## 6.8 章节列表页面

### 6.8.1 页面目标

管理章节结构并进入写作。

### 6.8.2 布局

```text
┌────────────────────────────────────────────┐
│ 章节 [新建章节] [批量生成章节路线] [搜索]       │
├────────────────────────────────────────────┤
│ 表格：序号 / 标题 / 状态 / 字数 / 摘要 / 最近编辑 │
└────────────────────────────────────────────┘
```

交互：
- 双击章节 → 进入编辑器。
- 拖拽排序 → 更新 chapter_index。
- 删除章节 → 软删除并提示。

## 6.9 章节编辑器页面

### 6.9.1 页面目标

提供核心写作空间。

### 6.9.2 布局

```text
┌──────────────────────────────────────────────────────┐
│ 章节标题 / 状态 / 字数 / 保存状态 / 快照 / 检查          │
├──────────────┬────────────────────────┬──────────────┤
│ 章节树         │ Markdown 编辑器          │ 上下文面板      │
│ 卷一           │                        │ 章节摘要        │
│  第一章        │                        │ 出场角色        │
│  第二章        │                        │ 相关设定        │
│               │                        │ 主线节点        │
│               │                        │ 名词锁定        │
├──────────────┴────────────────────────┴──────────────┤
│ AI 指令栏：生成草稿 / 续写 / 改写 / 润色 / 去 AI 味 / 自定义 │
└──────────────────────────────────────────────────────┘
```

### 6.9.3 AI 输出面板

AI 生成时右侧或底部出现预览面板：

```text
┌────────────────────────────────────────────┐
│ AI 结果预览                                  │
│ 状态：生成中 / 已完成 / 失败                   │
│ 文本内容                                     │
│ [插入到光标] [替换选区] [追加到末尾] [丢弃]      │
└────────────────────────────────────────────┘
```

### 6.9.4 保存状态

状态文案：
- 已保存。
- 正在保存。
- 有未保存修改。
- 自动保存草稿于 12:30。
- 保存失败，点击重试。

## 6.10 一致性检查页面

### 6.10.1 页面目标

集中展示问题并辅助修复。

### 6.10.2 布局

```text
┌────────────────────────────────────────────┐
│ 一致性检查 [检查当前章] [检查全书] [筛选]       │
├──────────────┬─────────────────────────────┤
│ 问题列表       │ 问题详情                     │
│ 高：角色冲突    │ 原文片段                     │
│ 中：名词误写    │ 问题说明                     │
│ 低：AI 腔      │ 修复建议                     │
│               │ [定位原文] [忽略] [标记修复]  │
└──────────────┴─────────────────────────────┘
```

## 6.11 导出页面

### 6.11.1 页面目标

导出作品或资产。

### 6.11.2 布局

```text
┌────────────────────────────────────────────┐
│ 导出中心                                    │
├────────────────────────────────────────────┤
│ 导出范围：单章 / 多章 / 全书 / 设定集          │
│ 导出格式：TXT / Markdown                    │
│ 选项：包含标题 / 包含摘要 / 按卷分隔           │
│ 保存位置：[选择路径]                         │
│ [开始导出]                                  │
└────────────────────────────────────────────┘
```

## 6.12 设置页面

### 6.12.1 页面目标

配置模型、编辑器、自动保存、主题。

Tabs：
- 模型。
- 编辑器。
- 自动保存。
- 数据与备份。
- 关于。

### 6.12.2 模型设置

字段：
- Provider。
- Base URL。
- API Key。
- Model。
- Temperature。
- Max Tokens。
- Stream。
- 测试连接。

## 7. 组件清单

| 组件 | 用途 |
|---|---|
| AppShell | 全局布局 |
| TopBar | 顶部栏 |
| Sidebar | 主导航 |
| StatusBar | 底部状态 |
| ProjectCard | 最近项目卡片 |
| StatCard | 仪表盘统计卡 |
| BlueprintStepNav | 蓝图步骤导航 |
| EntityList | 角色/设定列表 |
| EntityForm | 通用资产表单 |
| ChapterTree | 章节树 |
| MarkdownEditor | 正文编辑器 |
| ContextPanel | 上下文面板 |
| AiCommandBar | AI 指令栏 |
| AiPreviewPanel | AI 输出预览 |
| IssueList | 问题列表 |
| ExportPanel | 导出配置 |
| SettingsForm | 设置表单 |

## 8. 空状态设计

| 页面 | 空状态文案 | 按钮 |
|---|---|---|
| 角色 | 还没有角色。先创建主角，让故事有第一个支点。 | 新建角色 / AI 创建 |
| 世界 | 还没有世界设定。先写下不可违反的规则。 | 新建设定 |
| 剧情 | 还没有剧情节点。先规划第一个转折。 | 新增节点 |
| 章节 | 还没有章节。创建第一章开始写作。 | 新建章节 |
| 检查 | 暂无问题。你可以检查当前章节。 | 开始检查 |

## 9. 可访问性与快捷键

快捷键：
- Ctrl+S：保存。
- Ctrl+F：当前页面搜索。
- Ctrl+P：全局快速跳转，P1。
- Ctrl+Enter：执行当前 AI 指令。
- Esc：关闭弹窗或 AI 预览。

可访问性：
- 所有按钮必须有可读 label。
- 表单错误必须文本提示。
- 不只依赖颜色表达状态。

## 10. 给 LLM Agent 的 UI 执行提示词

```text
你是文火 NovelForge 的 UI 实现 Agent。请按照《UI 页面原型说明文档》使用 React + TypeScript + Tailwind + shadcn/ui 实现页面。不要复制任何现有竞品 UI。优先实现 AppShell、项目中心、仪表盘、蓝图、角色、世界、剧情、章节列表、章节编辑器、检查、导出、设置。所有页面先接入 mock 数据，再逐步接入 Tauri API。章节编辑器必须包含章节树、Markdown 编辑区、上下文面板和 AI 指令栏。所有 AI 结果必须进入预览面板，用户确认后才插入正文。
```

---

# 文档六：《开发任务排期与人力配置表》

## 1. 文档信息

| 项目 | 内容 |
|---|---|
| 文档名称 | 开发任务排期与人力配置表 |
| 产品名称 | 文火 NovelForge |
| 目标版本 | MVP v0.1 |
| 建议周期 | 12 周 MVP |
| 团队规模 | 3-6 人，或由 LLM Agent 辅助小团队开发 |

## 2. MVP 团队角色

### 2.1 最小团队配置

| 角色 | 人数 | 职责 |
|---|---:|---|
| 产品 / 项目负责人 | 1 | 需求决策、范围控制、验收 |
| 前端工程师 | 1 | React UI、编辑器、状态管理 |
| Rust/Tauri 工程师 | 1 | 本地服务、数据库、文件系统、导出 |
| AI 工程师 | 1 可兼职 | Prompt、上下文、模型调用、AI 工作流 |
| 测试 / QA | 1 可兼职 | 测试用例、回归、验收 |
| UI/UX 设计师 | 1 可兼职 | 原型、视觉、组件规范 |

### 2.2 LLM Agent 分工建议

| Agent | 职责 |
|---|---|
| ProductAgent | 维护 PRD、验收标准、用户故事 |
| FrontendAgent | 实现 React 页面和组件 |
| TauriAgent | 实现 Rust commands 和 services |
| DataAgent | 实现 SQLite、文件协议、迁移 |
| AiAgent | 实现 Prompt、Context、Model Adapter |
| TestAgent | 生成测试用例和自动化测试 |
| DocAgent | 更新开发文档和用户手册 |

## 3. 12 周 MVP 里程碑

| 阶段 | 周期 | 目标 | 主要产出 |
|---|---|---|---|
| M0 准备 | 第 1 周 | 项目搭建与需求冻结 | 仓库、脚手架、PRD、原型 |
| M1 本地项目基础 | 第 2-3 周 | 项目创建、打开、数据库、文件结构 | 项目中心、ProjectService、迁移脚本 |
| M2 创作资产管理 | 第 4-5 周 | 蓝图、角色、世界、名词、剧情 | 表单页面、CRUD、基础索引 |
| M3 章节编辑器 | 第 6-7 周 | 章节列表、Markdown 编辑、自动保存 | 编辑器、ChapterService、恢复机制 |
| M4 AI 主闭环 | 第 8-9 周 | 模型配置、上下文、章节生成 | AiService、Prompt、AI 预览 |
| M5 检查与导出 | 第 10 周 | 一致性检查、TXT/MD 导出 | Issue 列表、ExportService |
| M6 稳定化与验收 | 第 11-12 周 | Bug 修复、测试、打包 | 安装包、测试报告、MVP 验收 |

## 4. Sprint 详细任务

## Sprint 1：项目初始化与基础框架

周期：第 1 周。

目标：完成可运行空壳和基础规范。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S1-01 | 初始化 Tauri + React + TS 项目 | TauriAgent / FrontendAgent | P0 | `pnpm tauri dev` 可启动 |
| S1-02 | 配置 Tailwind + shadcn/ui | FrontendAgent | P0 | 可渲染基础按钮、卡片 |
| S1-03 | 建立目录结构 | 全体 | P0 | 符合架构文档 |
| S1-04 | 定义 TypeScript 类型 | FrontendAgent | P0 | project/chapter/character 类型存在 |
| S1-05 | 定义 Rust Domain 类型 | TauriAgent | P0 | project/chapter/character struct 存在 |
| S1-06 | 建立错误结构 AppError | TauriAgent | P0 | 前端能显示标准错误 |
| S1-07 | 建立基础路由 | FrontendAgent | P0 | 项目中心、仪表盘、设置页面可跳转 |
| S1-08 | 建立日志系统 | TauriAgent | P1 | Rust 能写入脱敏日志 |

### 交付物

- 可运行桌面空壳。
- 基础布局。
- 标准错误结构。
- 初始代码规范。

## Sprint 2：本地项目系统

周期：第 2 周。

目标：实现新建项目、打开项目、最近项目。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S2-01 | 实现项目目录初始化 | DataAgent | P0 | 创建标准目录 |
| S2-02 | 实现 project.json 写入 | DataAgent | P0 | 内容符合协议 |
| S2-03 | 实现 SQLite 初始化 | DataAgent | P0 | project.sqlite 创建成功 |
| S2-04 | 实现迁移系统 | DataAgent | P0 | 0001_init.sql 可执行 |
| S2-05 | 实现 create_project command | TauriAgent | P0 | 前端可调用创建项目 |
| S2-06 | 实现 open_project command | TauriAgent | P0 | 可打开有效项目 |
| S2-07 | 实现最近项目记录 | TauriAgent | P1 | 最近项目列表可显示 |
| S2-08 | 实现项目中心 UI | FrontendAgent | P0 | 可新建/打开项目 |

### 验收场景

1. 新建项目后进入仪表盘。
2. 关闭软件后最近项目仍存在。
3. 打开无效目录时显示错误。

## Sprint 3：数据库 CRUD 与仪表盘

周期：第 3 周。

目标：实现基础数据服务和仪表盘统计。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S3-01 | 实现 ProjectService | TauriAgent | P0 | 可读取项目元信息 |
| S3-02 | 实现 DashboardStats | TauriAgent | P0 | 返回统计数据 |
| S3-03 | 实现基础 Query API 封装 | FrontendAgent | P0 | 前端可调用 Rust API |
| S3-04 | 实现仪表盘 UI | FrontendAgent | P0 | 显示统计卡片 |
| S3-05 | 编写项目服务测试 | TestAgent | P0 | 创建/打开测试通过 |

## Sprint 4：创作蓝图模块

周期：第 4 周。

目标：实现 8 步蓝图表单。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S4-01 | 创建 blueprint_steps 表与服务 | DataAgent | P0 | 可保存步骤 |
| S4-02 | 实现蓝图 command | TauriAgent | P0 | list/update/complete 可用 |
| S4-03 | 实现蓝图页面 UI | FrontendAgent | P0 | 8 步可切换 |
| S4-04 | 实现步骤状态 | FrontendAgent | P0 | 未开始/进行中/完成 |
| S4-05 | 实现 AI 建议按钮占位 | FrontendAgent | P1 | UI 有入口，AI 后续接入 |

## Sprint 5：角色、世界、名词、剧情资产

周期：第 5 周。

目标：实现创作资产基础管理。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S5-01 | 实现 characters CRUD | DataAgent/TauriAgent | P0 | 角色可增删改查 |
| S5-02 | 实现 world_rules CRUD | DataAgent/TauriAgent | P0 | 设定可增删改查 |
| S5-03 | 实现 glossary_terms CRUD | DataAgent/TauriAgent | P0 | 名词可锁定/禁用 |
| S5-04 | 实现 plot_nodes CRUD | DataAgent/TauriAgent | P0 | 主线节点可排序 |
| S5-05 | 实现角色页面 | FrontendAgent | P0 | 角色列表+详情表单 |
| S5-06 | 实现世界页面 | FrontendAgent | P0 | 分类+详情表单 |
| S5-07 | 实现名词页面 | FrontendAgent | P1 | 表格编辑 |
| S5-08 | 实现剧情页面 | FrontendAgent | P0 | 节点列表与详情 |

## Sprint 6：章节列表与章节服务

周期：第 6 周。

目标：实现章节创建、列表、读取、保存。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S6-01 | 实现 chapters 表服务 | DataAgent | P0 | 可创建章节记录 |
| S6-02 | 实现 Markdown 文件创建 | DataAgent | P0 | ch-0001.md 正确生成 |
| S6-03 | 实现 ChapterService | TauriAgent | P0 | read/save 可用 |
| S6-04 | 实现章节列表页面 | FrontendAgent | P0 | 可创建/打开章节 |
| S6-05 | 实现章节排序 | FrontendAgent/TauriAgent | P1 | 可调整顺序 |
| S6-06 | 实现章节软删除 | TauriAgent | P1 | 删除可恢复或隐藏 |

## Sprint 7：章节编辑器与自动保存

周期：第 7 周。

目标：实现核心写作体验。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S7-01 | 集成 Markdown 编辑器 | FrontendAgent | P0 | 可输入正文 |
| S7-02 | 实现章节树 | FrontendAgent | P0 | 左侧章节切换 |
| S7-03 | 实现上下文面板 | FrontendAgent | P0 | 显示角色/设定/主线 |
| S7-04 | 实现手动保存 | FrontendAgent/TauriAgent | P0 | Ctrl+S 保存 |
| S7-05 | 实现自动保存草稿 | FrontendAgent/TauriAgent | P0 | 5 秒 debounce |
| S7-06 | 实现草稿恢复 | TauriAgent/FrontendAgent | P0 | 重开提示恢复 |
| S7-07 | 实现字数统计 | FrontendAgent | P0 | 实时显示字数 |

## Sprint 8：AI 设置与模型调用

周期：第 8 周。

目标：打通 AI Provider。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S8-01 | 实现模型设置 UI | FrontendAgent | P0 | 可保存配置 |
| S8-02 | 实现 API Key 安全保存 | TauriAgent | P0 | 不进日志/项目目录 |
| S8-03 | 实现 OpenAI-compatible Adapter | AiAgent | P0 | 可请求模型 |
| S8-04 | 实现 test_provider | AiAgent | P0 | 测试连接可用 |
| S8-05 | 实现流式事件 | AiAgent/TauriAgent | P0 | 前端能接收 delta |
| S8-06 | 实现 AI 预览面板 | FrontendAgent | P0 | 显示生成内容 |

## Sprint 9：上下文与章节 AI 生成

周期：第 9 周。

目标：实现章节生成、续写、改写、去 AI 味。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S9-01 | 实现 ContextService | AiAgent/TauriAgent | P0 | 可收集章节上下文 |
| S9-02 | 实现 PromptBuilder | AiAgent | P0 | 可生成章节 Prompt |
| S9-03 | 实现 chapter.draft | AiAgent | P0 | 可生成章节草稿 |
| S9-04 | 实现 chapter.continue | AiAgent | P0 | 可续写 |
| S9-05 | 实现 chapter.rewrite | AiAgent | P1 | 可改写选区 |
| S9-06 | 实现 prose.naturalize | AiAgent | P1 | 可去 AI 味 |
| S9-07 | 接入编辑器 AI 指令栏 | FrontendAgent | P0 | 用户确认后插入 |

## Sprint 10：一致性检查与导出

周期：第 10 周。

目标：实现基础检查和 TXT/MD 导出。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S10-01 | 实现规则型名词检查 | AiAgent/TauriAgent | P0 | 发现禁用词/误写 |
| S10-02 | 实现 AI 一致性检查 | AiAgent | P0 | 返回问题 JSON |
| S10-03 | 实现 issues 表写入 | DataAgent | P0 | 问题可保存 |
| S10-04 | 实现检查中心 UI | FrontendAgent | P0 | 问题列表可查看 |
| S10-05 | 实现 TXT 导出 | TauriAgent | P0 | 单章/全书可导出 |
| S10-06 | 实现 Markdown 导出 | TauriAgent | P0 | 单章/全书可导出 |
| S10-07 | 实现导出页面 | FrontendAgent | P0 | 可选择格式与范围 |

## Sprint 11：测试、修复、稳定性

周期：第 11 周。

目标：修复主流程问题，提高稳定性。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S11-01 | 编写核心 Rust 单元测试 | TestAgent | P0 | 项目/章节/导出测试通过 |
| S11-02 | 编写前端关键测试 | TestAgent | P1 | 表单/页面基础测试通过 |
| S11-03 | 编写 E2E 主流程测试 | TestAgent | P0 | 新建→写作→导出通过 |
| S11-04 | 做数据损坏演练 | TauriAgent/TestAgent | P1 | 有错误提示 |
| S11-05 | 做 AI 失败演练 | AiAgent/TestAgent | P0 | 失败可恢复 |
| S11-06 | 修复 P0 Bug | 全体 | P0 | 无阻断问题 |

## Sprint 12：打包与 MVP 验收

周期：第 12 周。

目标：生成可安装 MVP。

### 任务清单

| ID | 任务 | 负责人 | 优先级 | 验收标准 |
|---|---|---|---|---|
| S12-01 | 配置 Tauri Build | TauriAgent | P0 | 可生成安装包 |
| S12-02 | 准备内测安装说明 | DocAgent | P0 | 用户可按说明安装 |
| S12-03 | 完成 MVP 验收清单 | ProductAgent/TestAgent | P0 | PRD 验收全部通过 |
| S12-04 | 整理已知问题 | ProductAgent | P0 | 有 Known Issues |
| S12-05 | 准备下一阶段 Backlog | ProductAgent | P1 | Beta backlog 完成 |

## 5. 依赖关系

```text
项目初始化
  ↓
项目创建 / 数据库 / 文件协议
  ↓
蓝图 / 角色 / 设定 / 主线 CRUD
  ↓
章节服务 / 编辑器 / 自动保存
  ↓
AI 设置 / 模型调用
  ↓
上下文组装 / 章节生成
  ↓
一致性检查 / 导出
  ↓
测试 / 打包 / 验收
```

## 6. MVP 风险与应对

| 风险 | 等级 | 应对 |
|---|---|---|
| Tauri 与编辑器集成复杂 | 中 | 先使用简单 Markdown 编辑器，再优化 |
| SQLite 与 Markdown 双写不一致 | 高 | 统一由 ChapterService 写入，禁止前端直写 |
| AI 输出不稳定 | 高 | 强化 Prompt、预览确认、失败重试 |
| 自动保存丢稿 | 高 | autosave 与正式保存分离，启动恢复 |
| API Key 泄露 | 高 | 使用 Credential Manager，日志脱敏 |
| UI 范围膨胀 | 中 | MVP 严格只做主闭环 |
| 导出格式复杂 | 低 | MVP 只做 TXT / Markdown |

## 7. MVP 发布验收清单

### 7.1 功能验收

- [ ] 新建项目成功。
- [ ] 打开项目成功。
- [ ] 最近项目可用。
- [ ] 仪表盘统计正确。
- [ ] 8 步蓝图可保存。
- [ ] 角色 CRUD 可用。
- [ ] 世界设定 CRUD 可用。
- [ ] 名词库可用。
- [ ] 主线节点可用。
- [ ] 章节创建可用。
- [ ] Markdown 编辑器可用。
- [ ] 手动保存可用。
- [ ] 自动保存可用。
- [ ] 草稿恢复可用。
- [ ] 模型配置可用。
- [ ] AI 流式输出可用。
- [ ] 章节草稿生成可用。
- [ ] 续写可用。
- [ ] 一致性检查可用。
- [ ] TXT / Markdown 导出可用。

### 7.2 质量验收

- [ ] 关闭后重新打开不丢数据。
- [ ] 保存失败有提示。
- [ ] AI 失败有提示。
- [ ] API Key 不出现在日志。
- [ ] 项目路径移动后可重新打开。
- [ ] 主要 P0 功能有测试。
- [ ] Windows 安装包可安装。

## 8. Beta 阶段 Backlog

MVP 后优先进入：

1. DOCX 导出。
2. PDF / EPUB 导出。
3. 旧稿导入。
4. 资产自动抽取。
5. 手动版本快照。
6. Git 集成。
7. 伏笔与叙事义务中心。
8. 时间线视图。
9. 角色关系图。
10. 本地向量检索。
11. 授权激活系统。
12. 自动更新。

## 9. 给 LLM Agent 的排期执行提示词

```text
你是文火 NovelForge 的开发执行 Agent。请按照《开发任务排期与人力配置表》从 Sprint 1 开始逐步实现，不要跳跃开发。每个 Sprint 输出：完成的任务 ID、修改的文件、运行方式、测试结果、未完成项、下一步建议。遇到需求不明确时，以 MVP 主闭环为准，不要擅自加入云同步、多人协作、插件市场。所有代码必须遵守《Windows 桌面端技术架构设计》和《数据库与本地文件协议》。
```

---

# 附录 A：整体 MVP 主闭环

```text
启动软件
  ↓
项目中心
  ↓
新建本地项目
  ↓
项目仪表盘
  ↓
填写创作蓝图
  ↓
创建角色 / 设定 / 名词 / 主线
  ↓
创建章节
  ↓
章节编辑器写作
  ↓
AI 生成草稿 / 续写 / 改写
  ↓
用户确认插入
  ↓
自动保存 / 手动保存
  ↓
一致性检查
  ↓
导出 TXT / Markdown
  ↓
关闭并重新打开项目，数据完整恢复
```

# 附录 B：统一开发红线

1. 不得复制 Moho 墨火的 UI、图标、文案、代码、资源。
2. 不得把章节正文只存在数据库里。
3. 不得让 AI 自动覆盖用户正文。
4. 不得把 API Key 写入 project.json 或日志。
5. 不得在 MVP 做云同步和多人协作。
6. 不得绕过 Service 层直接操作数据库和文件。
7. 不得删除用户资产而不提示引用风险。
8. 不得在写入失败时假装保存成功。
9. 不得让项目只能在单台电脑上使用。
10. 不得牺牲数据安全换取功能速度。

# 附录 C：建议后续继续拆分的 LLM Agent 任务包

1. `task-001-init-tauri-react-project.md`
2. `task-002-project-service-and-file-protocol.md`
3. `task-003-sqlite-migrations-and-repositories.md`
4. `task-004-project-center-ui.md`
5. `task-005-dashboard-ui.md`
6. `task-006-blueprint-module.md`
7. `task-007-character-world-glossary-plot-crud.md`
8. `task-008-chapter-service-and-markdown-editor.md`
9. `task-009-autosave-and-recovery.md`
10. `task-010-ai-provider-settings.md`
11. `task-011-context-service-and-prompt-builder.md`
12. `task-012-chapter-draft-generation.md`
13. `task-013-consistency-scan.md`
14. `task-014-export-service.md`
15. `task-015-e2e-tests-and-build.md`

