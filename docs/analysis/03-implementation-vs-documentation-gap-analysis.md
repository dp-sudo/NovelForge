# 实现与文档差异分析报告 v1.0

> 生成日期：2026-04-28
> 对比基准：Blueprint v1.0（docs/） + LLM Provider Spec v1.0（docs/）
> 代码基准：当前 HEAD

---

## 1. 文档规划 vs 实际实现对照总表

### 1.1 MVP P0 功能

| 模块 | 功能 | 文档范围 | 实现状态 | 差异 |
|---|---|---|---|---|
| 项目中心 | 新建项目 | P0 | ✅ 已实现 | 一致 |
| 项目中心 | 打开项目 | P0 | ✅ 已实现 | 一致 |
| 项目中心 | 最近项目 | P0 | ✅ 已实现 | 一致 |
| 仪表盘 | 数据概览 | P0 | ✅ 已实现 | 一致 |
| 蓝图 | 8 步基础表单 | P0 | ✅ 已实现 | 一致 |
| 蓝图 | AI 生成建议 | P0 | ✅ 接口就绪 | 后端 PromptBuilder 就绪，需真实 Provider 连接 |
| 角色 | CRUD | P0 | ✅ 已实现 | 一致 |
| 角色 | AI 创建角色 | P0 | ✅ 接口就绪 | 需真实 Provider 连接 |
| 世界 | CRUD | P0 | ✅ 已实现 | 一致 |
| 名词库 | CRUD | P0 | ✅ 已实现 | 一致 |
| 剧情 | 主线节点管理 | P0 | ✅ 已实现 | 一致 |
| 章节 | 章节列表 | P0 | ✅ 已实现 | 一致 |
| 编辑器 | Markdown 编辑 | P0 | ✅ 已实现 | textarea 实现，非 TipTap/Monaco |
| 编辑器 | AI 章节草稿 | P0 | ✅ 接口就绪 | 需真实 Provider 连接 |
| 编辑器 | AI 续写/改写 | P0 | ✅ 接口就绪 | 需真实 Provider 连接 |
| 一致性检查 | 基础检查 | P0 | ✅ 已实现 | 规则检查 + AI 检查双模式 |
| 设置 | 模型配置 | P0 | ✅ 已实现 | 超范围实现（7 供应商而非仅 OpenAI） |
| 导出 | TXT/Markdown | P0 | ✅ 已实现 | 一致 |
| 自动保存 | 5 秒 debounce | P0 | ✅ 已实现 | 一致 |
| 启动恢复 | 草稿恢复 | P0 | ✅ 已实现 | 一致 |

### 1.2 MVP P1 功能

| 功能 | 文档范围 | 实现状态 | 差异 |
|---|---|---|---|
| TXT/MD 导入 | P1 | ✅ 已实现 | ImportService (386 行) 完整实现 |
| 手动快照 | P1 | ✅ 已实现 | SnapshotService 完整实现 |
| 全局搜索 | P1 | ✅ 已实现 | FTS5 SearchService + 前端搜索界面 |
| 问题状态管理 | P1 | ✅ 已实现 | ConsistencyIssue 完整的状态流转 |
| Prompt 预览（调试） | P1 | ✅ 已实现 | 开发模式可用 |
| 局部插入策略 | P1 | ✅ 已实现 | 前端 AI 预览面板支持多种插入方式 |

### 1.3 MVP P2 / Beta 功能

| 功能 | 文档范围 | 实现状态 | 差异 |
|---|---|---|---|
| 图谱（关系图） | P2/Beta | ✅ 已实现 | RelationshipsPage 完整实现 |
| Git 版本管理 | P2/Beta | ✅ 已实现 | GitService (307 行) 完整实现 |
| 商业激活/授权 | P2/Beta | ✅ 已实现 | LicenseService (217 行) 完整实现 |
| DOCX 导出 | P2/Beta | ✅ 已实现 | ExportService 中完整实现 |
| PDF 导出 | P2/Beta | ✅ 已实现 | ExportService 中完整实现 |
| EPUB 导出 | P2/Beta | ✅ 已实现 | ExportService 中完整实现 |
| 向量检索 | Beta/v1.0 | ✅ 已实现 | VectorService (458 行) 完整实现 |
| 自动更新 | 未提及 | ✅ 已实现 | tauri-plugin-updater 配置就绪 |

---

## 2. 范围膨胀详细分析

### 2.1 超范围实现清单

以下功能在文档中明确标注为"Beta/P2/v1.0"，但在 MVP 阶段（按文档定义）已被实现：

| 功能 | 文档位置 | 代码位置 | 行数 | 投入估算 |
|---|---|---|---|---|
| DOCX 导出 | P2 "Beta 再做" | export_service.rs | ~150 行 | 中 |
| PDF 导出 | P2 "Beta 再做" | export_service.rs | ~150 行 | 中 |
| EPUB 导出 | P2 "Beta 再做" | export_service.rs | ~150 行 | 中 |
| Git 集成 | P2 "Beta 再做" | git_service.rs | 307 行 | 高 |
| License 系统 | P2 "Beta 再做" | license_service.rs | 217 行 | 高 |
| 关系图谱 | P2 "Beta 再做" | RelationshipsPage | ~200 行 | 中 |
| 向量检索 | v1.0 目标 | vector_service.rs | 458 行 | 高 |
| Narrative Obligation | Beta Backlog | narrative_service.rs + ts | 322 行 | 中 |
| Timeline 视图 | 未在文档中 | TimelinePage + timelineApi | ~150 行 | 低 |
| Anthropic Adapter | LLM Spec 第二阶段 | anthropic.rs | 456 行 | 高 |
| Gemini Adapter | LLM Spec 第二阶段 | gemini.rs | 374 行 | 高 |
| 自动更新 | 未提及 | tauri.conf.json 配置 | 配置项 | 低 |
| 模型注册表热更新 | LLM Spec 第二阶段 | model_registry_service.rs | 647 行 | 高 |

**估算额外投入：约 3,500+ 行代码，覆盖了文档中 12 个标记为"非 MVP"的功能点。**

### 2.2 范围膨胀原因推测

1. **开发未严格遵循 MVP 边界**：AGENTS.md 明确要求不添加未要求功能，但实际未遵守
2. **文档未及时同步**：AGENTS.md 要求"先读实现再改文档"，这些超范围功能应该已反映在文档修订中，但文档并未更新
3. **可能的外部因素**：可能有一个决策要将 MVP 范围扩大为"更完整的产品"，但文档未更新反映这一决策

### 2.3 范围膨胀的影响

| 影响 | 说明 |
|---|---|
| ✅ 正面 | 产品功能更完整，部分高质量功能（VectorService、GitService）为后续版本减少了技术债务 |
| ❌ 负面 | 模糊了 MVP 焦点，增加了测试和 bug 修复的工作量 |
| ❌ 负面 | 如果不确定是否保留这些功能，就存在"已投入的代码被废弃"的风险 |
| ❌ 负面 | 与文档设计不一致，给新加入的开发者造成困惑 |

---

## 3. 文档有但代码未实现的项

### 3.1 少数缺失项

| 项目 | 文档位置 | 缺失状态 | 影响 |
|---|---|---|---|
| TanStack Query 缓存 | 架构文档 | ❌ 未使用 | 前端 API 缓存未集中管理，每个页面自行处理加载状态 |
| TipTap/Monaco 编辑器 | 架构文档 | ❌ 使用 textarea | 编辑器功能受限（无富文本/语法高亮），但 MVP 阶段可接受 |
| 12 个内置 Skill | AI 系统设计 | ❌ SkillRegistry 为空 | AI Skill 功能未真正启用 |
| FTS5 search_index 表 | 数据库协议 | ❌ 未在 migration 中创建 | 但 SearchService 直接在代码中创建，功能可用 |
| 动画/转场规范 | UI 原型 | ❌ 未实现 | UI 没有过渡动画 |
| 自动备份（每日首次打开） | 数据库协议 | ❌ 未实现 | 备份需手动触发 |
| 快捷键 Ctrl+P（全局跳转） | UI 原型 | ❌ 未实现 | 仅 Ctrl+S 和 Ctrl+F 已实现 |
| 3 条测试项目的测试夹具 | AI 评测方案 | ❌ 未创建 | 没有内置测试项目用于 AI 评测 |

### 3.2 这些缺失项的影响评估

| 缺失项 | 严重程度 | 建议优先级 |
|---|---|---|
| SkillRegistry 为空 | 🟡 中 | 如果不使用 Skill 工作流，不影响基本 AI 功能 |
| TanStack Query 未使用 | 🟡 低 | 现有 Zustand + 直接 API 调用也能工作 |
| TipTap/Monaco 未使用 | 🟢 低 | textarea 对 MVP 够用 |
| 自动备份未实现 | 🟡 中 | 缺乏数据保护的重要机制 |
| AI 测试夹具缺失 | 🟡 中 | 无法可靠测试 AI 功能质量 |

---

## 4. 代码比文档多的功能

以下功能**在代码中已存在**但**完全不在任何文档中**：

| 功能 | 代码位置 | 状态 | 建议 |
|---|---|---|---|
| Volume（卷）管理 | chapter_service.rs + ChaptersPage | ✅ 完整实现 | 应补充到文档 |
| Timeline 时间线 | TimelinePage + narrative_commands.rs | ✅ 完整实现 | 应补充到文档 |
| Narrative Obligation（伏笔管理） | narrative_service.rs + NarrativePage | ✅ 完整实现 | 应补充到文档 |
| 角色关系管理 | character_service.rs + RelationshipsPage | ✅ 完整实现 | 应补充到文档 |
| 项目完整性检查 | integrity_service.rs | ✅ 完整实现 | 应补充到文档 |
| 导入功能 | import_service.rs | ✅ 完整实现 | 应补充到文档 |
| VectorService | vector_service.rs | ✅ 完整实现 | 应补充到文档 |
| GitService | git_service.rs | ✅ 完整实现 | 应补充到文档 |
| LicenseService | license_service.rs | ✅ 完整实现 | 应补充到文档 |

**结论**：代码比文档多出了至少 9 个功能模块，文档需要大规模同步更新。

---

## 5. 文档同步更新优先级

| 优先级 | 文档 | 需要更新的内容 |
|---|---|---|
| P0 | Blueprint PRD | 更新 P0/P1/P2 范围，反映实际实现状态 |
| P0 | Blueprint 架构文档 | 增加 Volume/Narrative/Integrity/Timeline 等服务说明 |
| P0 | 数据库与文件协议 | 增加 volumes/narrative_obligations 表的文档 |
| P0 | LLM Provider Spec | 将 Anthropic/Gemini Adapter 从第二阶段提升到已完成 |
| P1 | 开发排期 | 更新 Sprint 任务完成状态 |
| P1 | agents.md | 更新 MVP 范围边界（如已实现的功能应更新"禁做"清单） |

---

## 6. 代码质量评估（超范围实现的质量）

这些超范围实现的质量如何？快速评估：

| 功能 | 代码质量评估 |
|---|---|
| ExportService (DOCX/PDF/EPUB) | ⚠️ 使用了 `docx-rs` 和 `printpdf` 等 crate，实现较为基础 |
| GitService | ✅ 通过 `git2` crate 调用系统 Git，实现稳健 |
| LicenseService | ✅ 使用 RSA 签名验证，实现规范 |
| VectorService | ✅ 余弦相似度实现正确，生成 embedding 依赖 Provider |
| Anthropic Adapter | ✅ 完整实现 Messages API + 流式解析 |
| Gemini Adapter | ✅ 完整实现 GenerateContent API + 流式 |
| ModelRegistryService | ✅ 完整实现注册表生命周期管理 |

**总体评价**：超范围功能的代码质量与核心功能一致，没有"匆忙拼凑"的迹象。

---

## 7. 总体差异总结

```
文档规划的 MVP
┌─────────────────────────────────────────────┐
│ 项目中心 / 仪表盘 / 蓝图                     │
│ 角色 / 世界 / 名词 / 剧情                   │
│ 章节编辑器 / 自动保存 / 恢复                │
│ AI: OpenAI-compatible + DeepSeek + Kimi + 智谱 │
│ 导出: TXT + Markdown                        │
└─────────────────────────────────────────────┘
                       ↓ 实际代码已超出 ↑
┌─────────────────────────────────────────────┐
│ + DOCX / PDF / EPUB 导出                    │
│ + Git 版本管理                              │
│ + License 商业授权                          │
│ + 向量检索                                  │
│ + 叙事义务（伏笔管理）                       │
│ + 时间线视图                                │
│ + 角色关系图谱                              │
│ + 项目完整性检查                            │
│ + 章节导入                                  │
│ + Anthropic / Gemini Adapter                │
│ + 模型注册表热更新                          │
│ + 自动更新                                  │
│ + Volume 卷管理                             │
└─────────────────────────────────────────────┘
```

**核心建议**：选择以下路径之一：

1. **路径 A — 缩小到文档范围**：将超范围功能标记为"Beta"，前端暂时隐藏入口，专注于 20 个 P0 功能的集成和测试
2. **路径 B — 更新文档认可现状**：承认范围已扩大，更新所有文档反映实际实现，然后集中精力进行集成测试和 bug 修复
3. **路径 C — 中间路线**：保留核心超范围功能（Anthropic/Gemini/DOCX 导出/卷管理），将边缘功能（Git/License/VectorSearch）标记为 Beta
