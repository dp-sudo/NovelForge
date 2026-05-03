# Story OS v2 深改验收报告（本次提交）

## 1. 功能完成度

- Pipeline 阶段已切换为：
  - `validate -> compile_context -> route -> compose_prompt -> generate -> postprocess -> review -> persist -> checkpoint`
- 事件元数据已增强：
  - `taskContract`
  - `contextCompilationSnapshot`
  - `reviewChecklist`
  - `reviewWorkItems`
  - `checkpointId`
- 编辑器支持：
  - 审查工单三态更新（已处理/驳回/回待办）
  - 最近 checkpoint 摘要
  - 精修状态聚合
- Timeline / Relationships / Dashboard / Export 页面：
  - 显示审查待办聚合
  - 审阅结果显示契约提示、审查清单、审查工单
  - 可直接更新工单状态

## 2. 数据与迁移

- 新增迁移：`0005_story_os_v2_governance.sql`
- 新增 v2 治理表族（run ledger/context snapshots/review work items/polish actions）
- `integrity_service` 已将目标版本切换到 `0005_story_os_v2_governance`

## 3. 工程验证

- 前端：
  - `npm run typecheck` 通过
  - `npm run typecheck:web` 通过
  - `npm run build:web` 通过
- 后端：
  - `cargo test task_routing::tests -- --nocapture` 通过
  - `cargo test integrity_service::tests::fresh_project_is_healthy -- --nocapture` 通过
  - `cargo test ai_pipeline_service::tests::touch_pipeline_phase_updates_audit_phase -- --nocapture` 通过

## 4. 未完成/受限项

- Node 测试在当前沙箱环境触发 `spawn EPERM`，`npm test` 未能在本环境完成全量通过验证。
- Book pipeline API 已接入 v2 元数据透传；当前项目无独立 BookPipeline 页面，UI 侧未新增专页。

## 5. 已知限制

- 仍保留 `ai_story_checkpoints` / `ai_review_queue` 兼容写入路径，供旧读路径兜底。
- v2 治理表目前以“新增并并行写入”方式切换，未对历史 v1 治理数据做离线回填。
