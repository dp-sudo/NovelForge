# 项目数据真实性与占位符代码审计报告 v1.0

> 生成日期：2026-04-28
> 审计范围：前端 TypeScript (src/) + 后端 Rust (src-tauri/) + 测试 (tests/)
> 审计方法：逐文件分析实现完整性，识别 Mock/占位符/未连接代码

---

## 1. 审计概述

### 1.1 总体评估

| 指标 | 数值 |
|---|---|
| 前端总文件数 | ~88 个 |
| 后端总文件数 | ~55 个 (13,327 行 Rust) |
| 测试文件数 | 6 个 |
| Mock/占位符文件 | 3 个核心风险文件 |
| 完全真实的模块 | 24/24 个 Rust Service |
| 部分真实的模块 | 少量前端 API 包装层 |

### 1.2 真实性等级定义

| 等级 | 定义 |
|---|---|
| ✅ 真实 | 生产级实现，有完整的业务逻辑和错误处理 |
| ⚠️ 部分真实 | 核心逻辑真实，但存在伪造的回退路径 |
| 🟡 占位符 | 有实现框架，但关键功能为空或返回硬编码值 |
| 🔴 伪造 | 完全 Mock，不调用任何真实服务 |

---

## 2. 后端 Rust (src-tauri/) — 真实性清单

### 2.1 全部 24 个 Service → ✅ 真实

| Service | 行数 | 状态 | 详细说明 |
|---|---|---|---|
| project_service.rs | 407 | ✅ | 完整的项目初始化、目录创建、project.json 读写、15 个标准目录 |
| chapter_service.rs | 1,180 | ✅ | 章节 CRUD + volumns + snapshots + autosave + 草稿恢复 + 排序 + 软删除 + 时间线 |
| character_service.rs | 337 | ✅ | CRUD + 软删除 + 角色关系管理，含单元测试 |
| world_service.rs | 107 | ✅ | 世界规则 CRUD |
| glossary_service.rs | 93 | ✅ | 名词术语 CRUD |
| plot_service.rs | 177 | ✅ | 主线节点 CRUD + 排序 |
| blueprint_service.rs | 192 | ✅ | 8 步蓝图 CRUD + 状态管理 |
| consistency_service.rs | 142 | ✅ | 一致性扫描 + 问题管理 |
| context_service.rs | 504 | ✅ | 三层上下文收集（全局+相关+检索） |
| dashboard_service.rs | 108 | ✅ | 仪表盘统计聚合 |
| export_service.rs | 1,035 | ✅ | 五种格式导出（txt/md/docx/pdf/epub）+ Zip |
| ai_service.rs | 336 | ✅ | AI 文本生成 + 流式 + 认证头管理 |
| search_service.rs | 156 | ✅ | SQLite FTS5 全文搜索 |
| vector_service.rs | 458 | ✅ | 余弦相似度语义搜索（超过 MVP 范围） |
| backup_service.rs | 198 | ✅ | Zip 备份创建 + 恢复 |
| import_service.rs | 386 | ✅ | 章节导入 + 资产抽取 |
| integrity_service.rs | 289 | ✅ | 项目完整性检查 |
| git_service.rs | 307 | ✅ | Git init/status/commit/history（超过 MVP 范围） |
| license_service.rs | 217 | ✅ | 离线授权验证（超过 MVP 范围） |
| settings_service.rs | 293 | ✅ | Provider 配置 CRUD |
| model_registry_service.rs | 647 | ✅ | 模型注册表管理 + 远程同步 + 离线缓存 |
| narrative_service.rs | 322 | ✅ | 叙事义务管理（契诃夫之枪） |
| prompt_builder.rs | 418 | ✅ | 结构化 Prompt 构建 |
| skill_registry.rs | 165 | ⚠️ | 框架完整但 Skill 注册表为空 |

### 2.2 LLM Adapters → ✅ 真实（超出文档范围）

| Adapter | 行数 | 状态 | 说明 |
|---|---|---|---|
| openai_compatible.rs | 501 | ✅ | Chat Completions + 流式 + 模型列表 + 能力检测 |
| anthropic.rs | 456 | ✅ | Messages API + 流式（文档说 Beta 阶段做） |
| gemini.rs | 374 | ✅ | GenerateContent + StreamGenerateContent（文档说 Beta 阶段做） |
| vendor_stubs.rs | 10 | ✅ | 类型别名，非占位符 |

### 2.3 数据库 → ✅ 真实

- `database.rs`：17 个表的 project-level 数据库
- `app_database.rs`：6 个表的 app-level 数据库
- 所有表结构与文档一致或扩展

### 2.4 Tauri Commands → ✅ 真实

注册 117 个 invoke handler，每个都有参数校验和错误处理。

---

## 3. 前端 TypeScript (src/) — 真实性清单

### 3.1 页面组件 → ✅ 真实

全部 15 个页面组件都实现了完整的 UI 结构、状态管理、API 调用：

| 页面 | 状态 | 说明 |
|---|---|---|
| ProjectCenterPage | ✅ | 新建/打开项目表单 |
| DashboardPage | ✅ | 统计数据卡片 |
| BlueprintPage | ✅ | 8 步导航 + 表单 + AI 面板 |
| CharactersPage | ✅ | 列表 + 详情 + AI 生成 |
| WorldPage | ✅ | 分类树 + 详情 |
| GlossaryPage | ✅ | 表格编辑 |
| PlotPage | ✅ | 节点列表 + 详情 |
| ChaptersPage | ✅ | 章节表格 + Volume 管理 |
| EditorPage | ✅ | 4 栏编辑器完整布局 |
| ConsistencyPage | ✅ | 问题列表 + 详情 |
| ExportPage | ✅ | 导出配置表单 |
| SettingsPage | ✅ | 模型/编辑器/备份/授权/关于 Tab |
| NarrativePage | ✅ | 伏笔跟踪 |
| TimelinePage | ✅ | 时间线视图 |
| RelationshipsPage | ✅ | 角色关系图 |

### 3.2 API 包装层 → ✅ 真实（但有回退）

| API 文件 | 状态 | 说明 |
|---|---|---|
| tauriClient.ts | ✅ | 统一 invoke 包装 + 错误标准化 |
| projectApi.ts | ✅ | 纯 Tauri invoke |
| chapterApi.ts | ✅ | 纯 Tauri invoke |
| characterApi.ts | ✅ | 纯 Tauri invoke |
| blueprintApi.ts | ✅ | 纯 Tauri invoke |
| worldApi.ts | ✅ | 纯 Tauri invoke |
| glossaryApi.ts | ✅ | 纯 Tauri invoke |
| plotApi.ts | ✅ | 纯 Tauri invoke |
| narrativeApi.ts | ✅ | 纯 Tauri invoke |
| timelineApi.ts | ✅ | 纯 Tauri invoke |
| consistencyApi.ts | ✅ | 纯 Tauri invoke |
| contextApi.ts | ✅ | 纯 Tauri invoke |
| statsApi.ts | ✅ | 纯 Tauri invoke |
| aiApi.ts | ✅ | 纯 Tauri invoke + 事件监听 |
| **exportApi.ts** | ⚠️ | Tauri invoke 失败后回退到 DevExport |
| **settingsApi.ts** | ⚠️ | Provider/Model 走 Tauri；Editor 设置走 localStorage DevSettings |

### 3.3 前端 Service 层 → ⚠️ 双模

`src/services/` 下的 11 个文件设计为**可在无 Tauri 环境下运行**，直接操作 SQLite + 文件系统：

| Service | 状态 | 说明 |
|---|---|---|
| project-service.ts | ✅ | 真实 SQLite 操作 |
| chapter-service.ts | ✅ | 真实文件 + DB 操作 |
| character-service.ts | ✅ | 真实 SQLite 操作 |
| world-service.ts | ✅ | 真实 SQLite 操作 |
| glossary-service.ts | ✅ | 真实 SQLite 操作 |
| plot-service.ts | ✅ | 真实 SQLite 操作 |
| consistency-service.ts | ✅ | 真实 SQLite 操作 |
| export-service.ts | ✅ | 真实文件操作 |
| settings-service.ts | ⚠️ | Editor 设置走 localStorage |
| context-service.ts | ✅ | 真实数据组装 |
| **ai-service.ts** | ⚠️ | 核心逻辑真实，但有 `mock://` 回退路径 |

---

## 4. 占位符代码详细清单

### 4.1 🔴 `src/api/dev-engine.ts` — 完整 Mock 层

**文件路径**：`F:\NovelForge\src\api\dev-engine.ts`
**行数**：983 行
**风险等级**：🔴 高

这是项目中**最大的占位符代码**，实现了一个完整的 localStorage 假后端：

```typescript
// 所有 Dev* 类都完全模拟真实后端的行为
// 不调用任何 Tauri invoke，全部操作 localStorage
DevProject  → localStorage 中的项目 CRUD
DevChapter  → localStorage 中的章节 CRUD + autosave + reorder + snapshots + volumes
DevCharacter → localStorage 中的角色管理
DevWorld    → localStorage 中的世界设定
DevGlossary → localStorage 中的名词库
DevPlot     → localStorage 中的剧情节点
DevBlueprint → localStorage 中的蓝图步骤
DevAi       → 固定 800ms 延时 + 返回硬编码中文章节文本
DevConsistency → 硬编码的一致性检查结果
DevExport   → 模拟文件下载（实际不写文件）
DevStats    → 从 localStorage 聚合统计
DevSettings → localStorage 中的设置
```

**硬编码的假 AI 输出示例**（dev-engine.ts 中）：
```
「是时候了。」苏铭低声自语。
他将目光从窗外收回，落在桌面上那张泛黄的地图上。
风雨飘摇的东域十七国，像一片被撕裂的枫叶。
他的手指在地图上缓缓划过，最后停在一个偏僻的角落——落霞镇。
...
```
以及：
```
{"issues":[
  {"type":"glossary","severity":"high","chapterId":"ch_01","sourceText":"夜潮","explanation":"锁定名词'夜潮'被误写为'夜朝'","suggestedFix":"将'夜朝'改为'夜潮'"},
  ...
]}
```

**风险**：前端可以在完全不连接 Tauri 后端的模式下开发和运行，意味着 **前端功能可能从未真正与 Rust 后端完整集成过**。

### 4.2 🟡 `src/services/ai-service.ts` — Mock URL 前缀

**文件路径**：`F:\NovelForge\src\services\ai-service.ts`
**风险等级**：🟡 中

```typescript
// 当 baseUrl 以 "mock://" 开头时，绕过所有真实 API 调用
// 返回预定义的硬编码响应
```

这允许在配置中使用 `mock://` 作为 Provider URL 时，完全不调用任何 LLM 服务。

### 4.3 🟡 `src-tauri/src/commands/ai_commands.rs` — legacy_mock_preview

**文件路径**：`F:\NovelForge\src-tauri\src\commands\ai_commands.rs`
**风险等级**：🟡 中

```rust
// 作为 Rust 后端 AI Command 的回退路径
// 当 Provider 服务或配置不可用时，返回此硬编码结果
// 包含完整的段落、对话、环境描写
```

这段代码的存在说明：**AI 功能从未被强制要求连接真实 Provider 才能工作**。

### 4.4 🟡 `src-tauri/src/services/skill_registry.rs` — 空注册表

**文件路径**：`F:\NovelForge\src-tauri\src\services\skill_registry.rs`
**行数**：165 行
**风险等级**：🟡 中

```rust
impl Default for SkillRegistry {
    fn default() -> Self {
        Self { skills: HashMap::new() } // 空的！
    }
}
```

实现了完整的注册/查找/执行框架（165 行），但注册表是空的。Blueprint 文档中设计了 12 个内置 Skill，全部未注册。

### 4.5 🟡 `src/api/settingsApi.ts` — Editor 设置直走 localStorage

**文件路径**：`F:\NovelForge\src\api\settingsApi.ts`
**风险等级**：🟡 中

```typescript
// Provider 和 Model 设置走 Tauri invoke（真实后端）
// 但是 Editor 设置（字体大小、行高、自动保存间隔等）直接读写 localStorage
// 使用 Dev.DevSettings.loadEditor() / saveEditor()
```

设计不一致——同样是"设置"，有的走 Tauri，有的走 localStorage。

### 4.6 🟡 `src/app/providers.tsx` — 空壳

**文件路径**：`F:\NovelForge\src\app\providers.tsx`
**风险等级**：🟡 低

```tsx
export function Providers({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}
```

纯粹的空壳。架构文档中提到了 React Router Provider、Theme Provider 等，但全都没实现。

---

## 5. 前后端集成验证的关键缺口

### 5.1 接口匹配风险

dev-engine 中定义的 API 响应格式可能与 Tauri invoke 返回的真实格式不完全一致。以下是需要重点验证的接口：

| 前端调用 | 使用的 API | 后端是否实现 | 验证状态 |
|---|---|---|---|
| projectApi.ts → invoke('create_project') | ✅ Rust 有实现 | ❌ 未验证 |
| chapterApi.ts → invoke('save_chapter_content') | ✅ Rust 有实现 | ❌ 未验证 |
| characterApi.ts → invoke('list_characters') | ✅ Rust 有实现 | ❌ 未验证 |
| aiApi.ts → invoke('generate_chapter_draft') | ✅ Rust 有实现 | ❌ 未验证 |
| exportApi.ts → invoke('export_chapter_txt') | ✅ Rust 有实现 | ❌ 未验证 |
| consistencyApi.ts → invoke('scan_chapter') | ✅ Rust 有实现 | ❌ 未验证 |

**所有核心接口都有 Rust 后端实现，但没有证据表明前端和真实后端之间进行过端到端集成测试。**

### 5.2 现有测试覆盖

```
tests/
├── chapter-autosave.test.ts        → 前端 autosave 单元测试
├── dev-engine-settings.test.ts     → dev-engine 测试（非真实后端）
├── helpers/temp-workspace.ts       → 测试辅助工具
├── integration/mvp-closed-loop.test.ts → 可能的主闭环集成测试
├── project-and-ai-errors.test.ts   → 错误处理测试
└── project-create-safety.test.ts   → 项目创建安全测试
```

仅有 6 个测试文件，对于一个 13,000+ 行 Rust 后端的项目来说严重不足。更关键的是：**没有端到端测试验证 Tauri invoke → Rust Service → SQLite/FileSystem 的全链路正确性。**

---

## 6. Rust 后端自带的嵌入式测试

只有 `character_service.rs` 包含嵌入式测试：
```rust
#[test]
fn character_create_and_list_succeeds() {
    // ...
}
```

其他 23 个 Service 都没有 Rust 侧的单体测试。

---

## 7. 风险矩阵

| 风险 | 等级 | 影响 | 触发条件 |
|---|---|---|---|
| 前后端接口不匹配 | 🔴 高 | 核心功能无法运行 | 关闭 dev-engine，使用真实 Tauri invoke |
| AI 从未连接真实模型 | 🔴 高 | AI 功能实际上不可用 | 配置真实 API Key 调用 |
| 测试覆盖率严重不足 | 🟡 中 | 回归风险高 | 修改后端逻辑 |
| Skill 注册表为空 | 🟡 中 | AI Skill 功能未真正启用 | 前端调用 list_skills |
| Editor 设置不持久 | 🟡 低 | 用户设置重启丢失 | 使用 localStorage 而非数据库 |
| 范围膨胀 | 🟡 中 | 需决策是否保留超范围功能 | 复盘 MVP 边界 |

---

## 8. 行动建议

按风险优先级：

1. 🔴 **集成测试**：编写 E2E 测试，验证所有核心 API 的前后端通信
2. 🔴 **AI 实连接测试**：至少用一个真实 Provider 测试 AI 流式输出
3. 🟡 **清理占位符**：决定 dev-engine.ts 的去留（保留为开发工具 or 移除）
4. 🟡 **补充 Rust 单测**：为 24 个 Service 编写基础单元测试
5. 🟡 **连接 Editor 设置**：后端已有 SettingsService，只需前端切换调用
6. 🟡 **明确范围**：决定 Git/License/VectorSearch/DOCX 等"超范围"功能是否保留
