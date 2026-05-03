# NovelForge 长篇小说生产操作系统（AI 主推进版）

## 1. 定位

NovelForge 的核心不再是“让 AI 写更多”，而是让 AI 在正确机制下持续推进生产：

- 正确的故事权威层（Story Authority Layer）
- 正确的状态层（State Layer）
- 正确的场景能力包（Capability Pack）
- 正确的审查闭环（Review Gate）

该设计已落地到任务契约字段 `taskContract`，并在 AI Pipeline 事件中持续输出。

## 2. 五层机制

### 2.1 故事宪法层（Story Constitution）

- 承载对象：蓝图步骤（`blueprint_steps`）、锁定/禁用名词、叙事视角、写作风格。
- 作用：定义不可随意突破的创作边界，是所有生成任务的最高约束来源。

### 2.2 正式资产层（Formal Assets）

- 承载对象：角色、世界规则、剧情节点、名词、叙事义务。
- 作用：作为稳定资产库，提供跨章节可复用、可审计的事实与约束。

### 2.3 动态状态层（Dynamic State）

- 承载对象：章节正文、章节摘要、章节关联、结构化草稿池、运行中上下文。
- 作用：承接每次生成的短周期变化，并在进入正式资产前保留人审入口。

### 2.4 上下文编译层（Context Compiler）

- 承载能力：`ContextService.collect_*` + `PromptBuilder`。
- 作用：把宪法层 + 正式资产层 + 动态状态层编译成任务级 prompt 上下文，避免“无状态写作”。

### 2.5 人工审查与精修层（Human Review Gate）

- 承载能力：编辑器预览、差异查看、结构化候选采纳、各类 review/scan 任务。
- 作用：关键结果不直接视为最终真相，必须可审、可改、可追踪。

## 3. 任务执行契约（Task Contract）

每个 `taskType` 都映射到统一契约：

- `authorityLayer`
- `stateLayer`
- `capabilityPack`
- `reviewGate`

当前约定：

- `chapter.*`, `prose.naturalize`：
  - `scene_execution` + `dynamic_scene_state` + `scene-production-pack`
  - `reviewGate = manual_required`
- `character.create`, `world.create_rule`, `plot.create_node`, `glossary.create_term`, `narrative.create_obligation`：
  - `formal_assets` + `asset_state` + `asset-building-pack`
  - `reviewGate = manual_recommended`
- `blueprint.generate_step`：
  - `story_constitution` + `constitution_state` + `blueprint-planning-pack`
  - `reviewGate = manual_recommended`
- `consistency.scan`, `*.review`：
  - `review_audit` + `review_state` + `review-guard-pack`
  - `reviewGate = manual_required`

## 4. Pipeline 机制对齐

在 `validate -> compile_context -> route -> compose_prompt -> generate -> postprocess -> review -> persist -> checkpoint` 过程中：

- `validate`：输出 `taskType + taskContract`，明确本次生成处于哪一层权威与状态。
- `compile_context`：输出 `contextCompilationSnapshot`（来源、裁剪、优先级、冲突策略、token 预算）。
- `route`：输出模型路由 + `taskContract`，保证“任务能力包”和“模型路由”关联可审计。
- `generate`：输出 `authorityLayer + capabilityPack`，让生成阶段显式声明能力边界。
- `review`：输出 `reviewChecklist + reviewWorkItems`，统一审查清单与工单实体。
- `persist`：默认直落库并并行写入审查闭环元数据。
- `checkpoint`：输出 `checkpointId`，形成可回放节点。

当前实现同时写入 v2 治理表族：

- `story_os_v2_run_ledger`
- `story_os_v2_context_snapshots`
- `story_os_v2_review_work_items`
- `story_os_v2_polish_actions`

兼容层保留原表 `ai_story_checkpoints` / `ai_review_queue` 供旧读路径兜底。

这意味着系统从“是否落库”升级为“落库后是否进入精修队列”的闭环。

## 5. 扩展原则

新增任何 AI 任务时，必须先定义契约再接入能力：

1. 该任务属于哪个故事权威层？
2. 该任务读写哪个状态层？
3. 该任务使用哪个能力包？
4. 该任务的审查策略是什么（`manual_required` / `manual_recommended` / `auto_allowed`）？

若无法回答以上四点，则不应接入生产链路。
