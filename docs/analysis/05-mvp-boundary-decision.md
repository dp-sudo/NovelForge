# MVP 边界确认与技术债务决策报告 v1.0

> 生成日期：2026-04-28
> 决策目标：确认哪些"超范围"功能应保留，哪些应标记为 Beta，哪些应移除

---

## 1. 背景

实际代码实现了大量文档中标记为"Beta/P2/v1.0"的功能。需要做出正式决策：
- 哪些功能保留在主闭环中
- 哪些功能标记为"Beta"（前端隐藏入口，后端保留）
- 是否需要更新文档以反映现状

---

## 2. 超范围功能清单

### 2.1 确认保留（建议）

这些功能质量高、有价值，与主闭环关联紧密，建议保留：

| 功能 | 代码位置 | 代码质量 | 保留理由 |
|---|---|---|---|
| **Volume（卷）管理** | chapter_service.rs | ✅ 高质量 | 章节分卷是创作的天然需求，PRD 遗漏了此功能 |
| **Timeline 时间线** | TimelinePage | ✅ 高质量 | 为创作者提供清晰的时间轴视图 |
| **角色关系管理** | character_service.rs | ✅ 高质量 | 角色关系图谱是创作资产管理的核心能力 |
| **项目完整性检查** | integrity_service.rs | ✅ 高质量 | 确保数据不损坏，符合 AGENTS.md 的可恢复要求 |
| **章节导入** | import_service.rs | ✅ 高质量 | P1 功能，对批量导入旧稿非常重要 |
| **DOCX 导出** | export_service.rs | ⚠️ 基础实现 | 用户常见需求，基础实现可用 |
| **Narrative Obligation（伏笔管理）** | narrative_service.rs | ✅ 高质量 | "契诃夫之枪"管理是创造性功能 |

### 2.2 标记为 Beta（建议）

这些功能有价值但不在 MVP 主闭环中，建议前端隐藏入口，后端保留：

| 功能 | 代码位置 | 代码行数 | 建议理由 |
|---|---|---|---|
| **PDF 导出** | export_service.rs | ~100 行 | PDF 排版复杂，当前基础，正式使用需完善 |
| **EPUB 导出** | export_service.rs | ~100 行 | 电子书格式，MVP 核心用户不需要 |
| **Anthropic Adapter** | anthropic.rs | 456 行 | 虽然是高质量实现，但用户通常从 OpenAI-compatible 开始 |
| **Gemini Adapter** | gemini.rs | 374 行 | 同上 |
| **模型注册表热更新** | model_registry_service.rs | 647 行 | 核心逻辑可用，但远程注册表服务尚未部署 |

### 2.3 需决策保留/标记（建议保留但明确范围）

| 功能 | 代码位置 | 代码行数 | 建议 |
|---|---|---|---|
| **Git 版本管理** | git_service.rs | 307 行 | **保留但标记为"高级功能"**。实现质量好（使用 `git2` crate），对写作者有价值的版本回溯能力。建议在 UI 中放在「设置 → 高级」下 |
| **Vector 语义搜索** | vector_service.rs | 458 行 | **保留但标记为"实验性"**。质量好但依赖 AI Provider 生成 embedding，实际效果取决于 Provider。FTS5 搜索已经能满足 MVP 需要 |
| **License 商业授权** | license_service.rs | 217 行 | **保留但不影响 MVP 使用**。目前是离线授权验证框架，没有真实施加限制。未来商业化时激活即可 |
| **自动更新** | tauri.conf.json | 配置项 | **保留**。轻微配置改动，无风险 |

---

## 3. 文档同步计划

根据以上决策，需要更新以下文档：

### P0 — 必须立即更新

| 文档 | 需更新的内容 |
|---|---|
| `AGENTS.md` §6 MVP 范围边界 | 将实际已实现的功能从"非目标"移除，或补充说明 |
| `docs/novel_workbench_windows_lifecycle_blueprint_v_1.md` | PRD §7 更新功能清单，架构文档补充各 Service |

### P1 — 建议更新

| 文档 | 需更新的内容 |
|---|---|
| `docs/novelforge_llm_provider_integration_spec_v_1.md` | 将 Anthropic/Gemini Adapter 从第二阶段提升到已完成 |
| `docs/README.md` | 更新功能概览 |

---

## 4. 待处理的遗留债务

### 4.1 需要解决的问题

| 问题 | 严重程度 | 责任方 |
|---|---|---|
| SkillRegistry 为空（165 行框架但无实际 Skill） | 🟡 中 | 需注册至少 5 个核心 Skill |
| Editor 设置尚未真正连接 Tauri 后端（已修改代码，未测试） | 🟡 低 | UI 集成测试后关闭 |
| FTS5 search_index 表未在 migration 中创建（SearchService 直接在代码中创建） | 🟢 低 | 仅在启动时重建索引，不影响功能 |
| 自动备份（每日首次打开）未实现 | 🟡 中 | 建议在启动流程中触发 |
| 没有 AI 测试夹具 | 🟡 中 | 无法可靠评估 AI 功能质量 |

### 4.2 已解决的问题（本次改动）

| 问题 | 状态 |
|---|---|
| exportApi.ts 的 dev-engine 回退 | ✅ 已移除 |
| settingsApi.ts 的 Dev.DevSettings 依赖 | ✅ 已移除 |
| Editor 设置走 localStorage | ✅ 已改为 Rust 后端持久化 |
| SettingsPage.tsx 使用同步函数调用异步 API | ✅ 已改为 async/await |

---

## 5. 决策选项

请从以下路径中选择：

### 路径 A：严格 MVP（推荐新团队）

- 将 Git/License/VectorSearch/PDF/EPUB 等后端代码保留但前端隐藏入口
- 将所有超范围 Rust 测试标记为 `#[ignore]`
- 集中测试 20 个 P0 功能的主闭环
- 优点：焦点清晰，测试工作量减少 40%
- 缺点：部分高质量代码暂时不可见

### 路径 B：扩展 MVP（推荐当前状态）

- 保留所有已实现功能，但明确分层：
  - **核心层**（P0）：项目/章节/角色/蓝图/导出/检查
  - **增强层**（P1）：Git/Timeline/Volume/Relationships/Import
  - **实验层**（Beta）：VectorSearch/PDF/EPUB/License
- 在 UI 中通过标签区分层级
- 优点：用户能使用更多功能
- 缺点：需要更多测试覆盖

### 路径 C：全保留 + 更新文档

- 接受所有功能为当前范围
- 更新所有文档以反映实际状态
- 为每个功能补充测试
- 优点：代码与文档一致
- 缺点：可能需要 2-3 周文档和测试工作

---

## 6. 建议

**建议选择路径 B（扩展 MVP）**，理由：
1. 代码质量普遍较高，废除已投入的工作不经济
2. 分层策略让用户可以自然地从核心功能过渡到高级功能
3. 测试工作可以通过"核心层优先"的策略分批完成
4. 与 AGENTS.md 的"可运行、可验证、可恢复、可维护"质量底线一致
