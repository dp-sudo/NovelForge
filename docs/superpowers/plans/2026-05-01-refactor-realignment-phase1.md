# S21 Refactor Realignment (Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不破坏现有新功能闭环的前提下，完成“新逻辑优先、旧逻辑收束”的第一阶段重构，让核心规则从超大文件中拆出并可独立验证。

**Architecture:** 本阶段不改产品能力边界，只重构实现边界。后端先把 `pipeline` 规则（持久化策略/冻结区守卫/运行态状态回写）模块化；前端拆分三大超大页面中的高风险子域（确定性分区、AI 流式驱动、设置页路由/模型管理），降低冲突面并保留原行为。所有改动均以现有契约测试为守门。

**Tech Stack:** Rust (Tauri service), TypeScript/React, Node test runner (`node --import tsx --test`), Cargo tests.

---

## Scope Check

`docs/11.md` 覆盖多个独立子系统。本计划只做 Phase 1（高冲突高风险链路）：

1. Pipeline 规则模块化（后端）
2. 页面超大文件拆分（前端）
3. 兼容桥收束（前后端）

Phase 2（状态语义细化、场景推断强化、能力池分层）另起计划，避免一次性跨太多子系统导致回归难控。

## File Structure

本阶段预期创建/修改文件如下：

- Create: `src-tauri/src/services/ai_pipeline/persist_policy.rs`
- Create: `src-tauri/src/services/ai_pipeline/freeze_guard.rs`
- Create: `src-tauri/src/services/ai_pipeline/runtime_state_writer.rs`
- Create: `src/pages/Blueprint/components/CertaintyZonesEditor.tsx`
- Create: `src/pages/Blueprint/components/BookPipelinePanel.tsx`
- Create: `src/pages/Blueprint/utils/certaintyZones.ts`
- Create: `src/pages/Editor/hooks/usePipelineStream.ts`
- Create: `src/pages/Editor/components/EditorContextPanel.tsx`
- Create: `src/pages/Settings/components/ModelRoutingPanel.tsx`
- Create: `src/pages/Settings/components/DataOpsPanel.tsx`
- Modify: `src-tauri/src/services/ai_pipeline/mod.rs`
- Modify: `src-tauri/src/services/ai_pipeline/orchestrator.rs`
- Modify: `src-tauri/src/services/ai_pipeline/task_handlers.rs`
- Modify: `src/pages/Blueprint/BlueprintPage.tsx`
- Modify: `src/pages/Editor/EditorPage.tsx`
- Modify: `src/pages/Settings/SettingsPage.tsx`
- Modify: `src/api/pipelineApi.ts`
- Modify: `src-tauri/src/commands/ai_commands.rs`
- Modify: `src-tauri/src/commands/settings_commands.rs`
- Test: `tests/integration/blueprint-certainty-zones-contracts.test.ts`
- Test: `tests/integration/pipeline-persist-policy-contracts.test.ts`
- Test: `tests/integration/skill-orchestration-contracts.test.ts`
- Test: `tests/integration/settings-routing-provider-guard.test.ts`
- Test: `tests/integration/editor-chapter-load-flow.test.ts`

### Task 1: 抽离 Pipeline 持久化策略与冻结区守卫（后端）

**Files:**
- Create: `src-tauri/src/services/ai_pipeline/persist_policy.rs`
- Create: `src-tauri/src/services/ai_pipeline/freeze_guard.rs`
- Modify: `src-tauri/src/services/ai_pipeline/mod.rs`
- Modify: `src-tauri/src/services/ai_pipeline/orchestrator.rs`
- Test: `src-tauri/src/services/ai_pipeline/persist_policy.rs` (inline unit tests)
- Test: `src-tauri/src/services/ai_pipeline/freeze_guard.rs` (inline unit tests)

- [ ] **Step 1: 写失败测试（先锁定当前行为）**

```rust
#[test]
fn parse_persist_mode_accepts_contract_values() {
    assert_eq!(parse_persist_mode("none"), Some(PersistMode::None));
    assert_eq!(parse_persist_mode("formal"), Some(PersistMode::Formal));
    assert_eq!(parse_persist_mode("derived_review"), Some(PersistMode::DerivedReview));
}

#[test]
fn detect_freeze_conflict_blocks_mutation_keyword() {
    let zones = CertaintyZones {
        frozen: vec!["终局真相".to_string()],
        promised: vec![],
        exploratory: vec![],
    };
    assert!(detect_freeze_conflict("请重写终局真相", &zones).is_some());
}
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parse_persist_mode_accepts_contract_values`  
Expected: FAIL（`persist_policy`/`freeze_guard` 尚未创建或函数未导出）

- [ ] **Step 3: 最小实现新模块**

```rust
// src-tauri/src/services/ai_pipeline/persist_policy.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistMode { None, Formal, DerivedReview }

pub fn parse_persist_mode(raw: &str) -> Option<PersistMode> { /* ... */ }
pub fn infer_legacy_persist_mode(canonical_task: &str) -> PersistMode { /* ... */ }
pub fn should_persist_task_output(canonical_task: &str, mode: PersistMode) -> bool { /* ... */ }
pub fn is_derived_review_task(canonical_task: &str) -> bool { /* ... */ }
```

```rust
// src-tauri/src/services/ai_pipeline/freeze_guard.rs
#[derive(Debug, Clone)]
pub struct FreezeConflict { pub matched_zone: String }

pub fn detect_freeze_conflict(user_instruction: &str, zones: &CertaintyZones) -> Option<FreezeConflict> { /* ... */ }
pub fn freeze_conflict_error(conflict: &FreezeConflict) -> AppErrorDto { /* ... */ }
```

- [ ] **Step 4: 在 orchestrator 中替换本地实现为模块调用**

```rust
use crate::services::ai_pipeline::persist_policy::{
    infer_legacy_persist_mode, parse_persist_mode, should_persist_task_output, PersistMode,
};
use crate::services::ai_pipeline::freeze_guard::{detect_freeze_conflict, freeze_conflict_error};
```

- [ ] **Step 5: 回归测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`  
Expected: PASS（至少 `ai_pipeline` 相关单测通过）

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/ai_pipeline/persist_policy.rs src-tauri/src/services/ai_pipeline/freeze_guard.rs src-tauri/src/services/ai_pipeline/mod.rs src-tauri/src/services/ai_pipeline/orchestrator.rs
git commit -m "refactor(ai-pipeline): extract persist policy and freeze guard modules"
```

### Task 2: 抽离运行态状态回写策略（后端）

**Files:**
- Create: `src-tauri/src/services/ai_pipeline/runtime_state_writer.rs`
- Modify: `src-tauri/src/services/ai_pipeline/mod.rs`
- Modify: `src-tauri/src/services/ai_pipeline/task_handlers.rs`
- Test: `src-tauri/src/services/ai_pipeline/runtime_state_writer.rs` (inline unit tests)

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn runtime_state_write_policy_respects_persist_mode_contract() {
    assert!(!should_persist_runtime_state_writes("chapter.plan", "none", "chapter_confirmed", true, true));
    assert!(should_persist_runtime_state_writes("chapter.plan", "formal", "chapter_confirmed", true, true));
}
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_state_write_policy_respects_persist_mode_contract`  
Expected: FAIL（`runtime_state_writer` 尚未实现）

- [ ] **Step 3: 实现状态回写策略与输入构造**

```rust
pub fn should_persist_runtime_state_writes(
    canonical_task: &str,
    persist_mode: &str,
    state_write_policy: &str,
    has_chapter_id: bool,
    is_promotion_action: bool,
) -> bool { /* ... */ }

pub fn build_runtime_story_state_input(
    canonical_task: &str,
    state_write_key: &str,
    request_id: &str,
    output_preview: &str,
) -> Option<StoryStateInput> { /* ... */ }
```

- [ ] **Step 4: TaskHandlers 改为委托调用**

```rust
use crate::services::ai_pipeline::runtime_state_writer::{
    build_runtime_story_state_input, should_persist_runtime_state_writes,
};
```

- [ ] **Step 5: 回归测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml persist_task_output_with_runtime_state_writes_records_story_state_entries`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/ai_pipeline/runtime_state_writer.rs src-tauri/src/services/ai_pipeline/mod.rs src-tauri/src/services/ai_pipeline/task_handlers.rs
git commit -m "refactor(ai-pipeline): extract runtime state writer policy from task handlers"
```

### Task 3: 拆分 Blueprint 页面中的确定性分区与编排面板（前端）

**Files:**
- Create: `src/pages/Blueprint/components/CertaintyZonesEditor.tsx`
- Create: `src/pages/Blueprint/components/BookPipelinePanel.tsx`
- Create: `src/pages/Blueprint/utils/certaintyZones.ts`
- Modify: `src/pages/Blueprint/BlueprintPage.tsx`
- Test: `tests/integration/blueprint-certainty-zones-contracts.test.ts`
- Test: `tests/integration/book-pipeline-orchestration.test.ts`

- [ ] **Step 1: 写失败测试（组件拆分后关键文案仍在）**

```ts
test("Blueprint 页面仍暴露确定性分区与一键编排入口", async () => {
  const page = await readRepoFile("src/pages/Blueprint/BlueprintPage.tsx");
  assert.match(page, /CertaintyZonesEditor/);
  assert.match(page, /BookPipelinePanel/);
});
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `node --import tsx --test --test-isolation=none tests/integration/blueprint-certainty-zones-contracts.test.ts tests/integration/book-pipeline-orchestration.test.ts`  
Expected: FAIL（组件尚未拆分）

- [ ] **Step 3: 抽取确定性分区工具与编辑组件**

```tsx
// CertaintyZonesEditor.tsx
export function CertaintyZonesEditor(props: {
  zones: BlueprintCertaintyZones;
  onChange: (next: BlueprintCertaintyZones) => void;
}) {
  // 保持原字段名与交互文案：冻结区/承诺区/探索区
}
```

- [ ] **Step 4: 抽取一键编排面板**

```tsx
// BookPipelinePanel.tsx
export function BookPipelinePanel(props: {
  running: boolean;
  status: string | null;
  logs: string[];
  onRun: () => void;
  onCancel: () => void;
}) { /* ... */ }
```

- [ ] **Step 5: 页面接线与回归测试**

Run: `node --import tsx --test --test-isolation=none tests/integration/blueprint-certainty-zones-contracts.test.ts tests/integration/book-pipeline-orchestration.test.ts`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/pages/Blueprint/components/CertaintyZonesEditor.tsx src/pages/Blueprint/components/BookPipelinePanel.tsx src/pages/Blueprint/utils/certaintyZones.ts src/pages/Blueprint/BlueprintPage.tsx
git commit -m "refactor(blueprint): extract certainty zones editor and book pipeline panel"
```

### Task 4: 拆分 Editor 页面的 AI 流式驱动与上下文侧栏（前端）

**Files:**
- Create: `src/pages/Editor/hooks/usePipelineStream.ts`
- Create: `src/pages/Editor/components/EditorContextPanel.tsx`
- Modify: `src/pages/Editor/EditorPage.tsx`
- Test: `tests/integration/editor-chapter-load-flow.test.ts`

- [ ] **Step 1: 写失败测试（页面应引用新 hook/组件）**

```ts
test("Editor 页面拆分流式驱动与上下文面板", async () => {
  const page = await readRepoFile("src/pages/Editor/EditorPage.tsx");
  assert.match(page, /usePipelineStream/);
  assert.match(page, /EditorContextPanel/);
});
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `node --import tsx --test --test-isolation=none tests/integration/editor-chapter-load-flow.test.ts`  
Expected: FAIL（尚未拆分）

- [ ] **Step 3: 抽取流式 hook（保持原错误映射与取消语义）**

```ts
export function usePipelineStream() {
  // 暴露 start/cancel，内部维护 requestId 与 stream 生命周期
  // 保持现有 PIPELINE_* 错误码映射策略
}
```

- [ ] **Step 4: 抽取上下文侧栏组件**

```tsx
export function EditorContextPanel(props: {
  context: ChapterContext | null;
  // 继续透传草案确认与资产采纳回调
}) { /* ... */ }
```

- [ ] **Step 5: 回归测试**

Run: `node --import tsx --test --test-isolation=none tests/integration/editor-chapter-load-flow.test.ts tests/state-summary-feedback.test.ts`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/pages/Editor/hooks/usePipelineStream.ts src/pages/Editor/components/EditorContextPanel.tsx src/pages/Editor/EditorPage.tsx
git commit -m "refactor(editor): extract pipeline stream hook and context panel"
```

### Task 5: 拆分 Settings 超大页并收束兼容桥入口（前后端）

**Files:**
- Create: `src/pages/Settings/components/ModelRoutingPanel.tsx`
- Create: `src/pages/Settings/components/DataOpsPanel.tsx`
- Modify: `src/pages/Settings/SettingsPage.tsx`
- Modify: `src/api/pipelineApi.ts`
- Modify: `src-tauri/src/commands/ai_commands.rs`
- Modify: `src-tauri/src/commands/settings_commands.rs`
- Test: `tests/integration/settings-routing-provider-guard.test.ts`
- Test: `tests/integration/pipeline-persist-policy-contracts.test.ts`

- [ ] **Step 1: 写失败测试（`autoPersist` 仅保留兼容桥，新增调用必须显式策略）**

```ts
test("核心入口继续显式 persistMode + automationTier", async () => {
  const api = await readRepoFile("src/api/pipelineApi.ts");
  assert.match(api, /persistMode/);
  assert.match(api, /automationTier/);
});
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `node --import tsx --test --test-isolation=none tests/integration/settings-routing-provider-guard.test.ts tests/integration/pipeline-persist-policy-contracts.test.ts`  
Expected: FAIL（尚未完成拆分/桥接收束）

- [ ] **Step 3: 抽取模型路由面板与数据运维面板**

```tsx
// ModelRoutingPanel.tsx
export function ModelRoutingPanel(/* props */) { /* provider + model + task route */ }

// DataOpsPanel.tsx
export function DataOpsPanel(/* props */) { /* backup + integrity + vector + git */ }
```

- [ ] **Step 4: 兼容桥收束**

```ts
// pipelineApi.ts
// 保留 autoPersist 输入字段，但仅做 legacy bridge 推导；新调用统一传 persistMode/automationTier
```

```rust
// ai_commands.rs / settings_commands.rs
// compatibility-only 命令继续可用，但记录 deprecated usage，并在日志中打 source
```

- [ ] **Step 5: 回归测试**

Run: `node --import tsx --test --test-isolation=none tests/integration/settings-routing-provider-guard.test.ts tests/integration/pipeline-persist-policy-contracts.test.ts tests/integration/skill-orchestration-contracts.test.ts`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/pages/Settings/components/ModelRoutingPanel.tsx src/pages/Settings/components/DataOpsPanel.tsx src/pages/Settings/SettingsPage.tsx src/api/pipelineApi.ts src-tauri/src/commands/ai_commands.rs src-tauri/src/commands/settings_commands.rs
git commit -m "refactor(settings): split settings page and tighten compatibility bridge boundaries"
```

### Task 6: 全链路验证与文档同步

**Files:**
- Modify: `docs/22.md`
- Modify: `docs/architecture/windows-desktop-architecture.md`
- Modify: `docs/runtime/runtime-process-spec.md`
- Modify: `docs/api/api-integration-spec.md`

- [ ] **Step 1: 跑本阶段关键契约测试**

Run: `node --import tsx --test --test-isolation=none tests/integration/book-pipeline-orchestration.test.ts tests/integration/blueprint-backfill-accuracy.test.ts tests/integration/blueprint-certainty-zones-contracts.test.ts tests/integration/pipeline-persist-policy-contracts.test.ts tests/integration/skill-orchestration-contracts.test.ts tests/integration/settings-routing-provider-guard.test.ts tests/integration/editor-chapter-load-flow.test.ts tests/state-summary-feedback.test.ts`  
Expected: PASS（36+）

- [ ] **Step 2: 跑类型检查**

Run: `npm run typecheck`  
Expected: PASS

- [ ] **Step 3: 跑 Rust 单测**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`  
Expected: PASS

- [ ] **Step 4: 同步文档（只写已实现行为）**

```md
- 更新 command/DTO 变化
- 更新 pipeline 阶段与策略边界
- 更新确定性分区与兼容桥状态
```

- [ ] **Step 5: Commit**

```bash
git add docs/22.md docs/architecture/windows-desktop-architecture.md docs/runtime/runtime-process-spec.md docs/api/api-integration-spec.md
git commit -m "docs: sync refactor phase1 architecture/runtime/api and progress ledger"
```

## Self-Review

1. Spec coverage：Phase 1 覆盖了 `11.md` 中“新逻辑优先 + 冲突旧逻辑收束”的关键链路（策略规则、页面耦合、兼容桥）。Phase 2 的“状态语义深化与场景编排升级”明确延期，不混入本计划。  
2. Placeholder scan：本计划无 `TODO/TBD/later` 占位符；每个代码步骤均给出文件与最小代码结构。  
3. Type consistency：统一使用 `PersistMode`、`automationTier`、`stateWrites/affectsLayers`、`certaintyZones` 命名，保持与现有契约一致。

