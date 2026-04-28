# NovelForge 一期统一 AI Pipeline 落地实施文档（可执行版）

## 1. 文档信息
1. 日期：2026-04-28
2. 范围：一期（范围 2）
3. 状态：待实施
4. 目标：将编辑器 9 个 AI 按钮统一收敛到同一后端编排链路，保留旧 command 兼容层
5. 决策输入（已确认）：
   - 架构：中编排层
   - 前后端：同时统一
   - 执行模型：同步返回 `requestId` + 流式事件
   - 兼容：保留旧 command，内部代理到新 pipeline
   - 接入范围：全部 9 个按钮

---

## 2. 当前代码基线（实施前事实）
1. 当前 AI 主入口在 `stream_ai_chapter_task`：
   - `src-tauri/src/commands/ai_commands.rs`
   - `src/api/aiApi.ts`
2. 当前流式事件协议为动态事件名：
   - `ai:stream-chunk:{requestId}`
   - `ai:stream-done:{requestId}`
3. 任务路由 canonical 能力已存在：
   - `src-tauri/src/services/task_routing.rs`
   - `src-tauri/src/commands/settings_commands.rs`
4. 路由唯一索引迁移已存在：
   - `src-tauri/migrations/project/0002_task_route_unique.sql`
   - `src-tauri/src/infra/migrator.rs`
5. 结构化草案已有第一版（关系/戏份/场景）：
   - `src-tauri/src/services/context_service.rs`
   - `src/pages/Editor/EditorPage.tsx`
6. 一期缺失项：
   - 无统一 `run_ai_task_pipeline` 入口
   - 无 pipeline 运行审计表
   - 无结构化草案池（batch/item）

---

## 3. 一期边界与非目标
### 3.1 一期边界（必须实现）
1. 新增统一 command：`run_ai_task_pipeline`、`cancel_ai_task_pipeline`
2. 新增统一后端服务：`AiPipelineService`
3. 编辑器 9 个按钮全部接入统一 pipeline
4. 旧 command 保留并代理到新 pipeline
5. 新增草案池数据模型并接入人工确认落库
6. 统一事件协议与错误协议（含 `phase`、`errorCode`）

### 3.2 非目标（一期不做）
1. 不引入异步 job 队列与任务调度器
2. 不实现自动无审落库
3. 不做复杂同义词图谱/实体合并引擎
4. 不大改现有业务表主键与历史数据结构

---

## 4. 目标架构（落地形态）
### 4.1 统一入口
1. 新增 command：
   - `run_ai_task_pipeline(input) -> requestId`
   - `cancel_ai_task_pipeline(requestId) -> void`
2. 固定事件名：
   - `ai:pipeline:event`
3. 事件负载关键字段：
   - `requestId`
   - `phase`（`validate/context/route/prompt/generate/postprocess/persist/done`）
   - `type`（`start/delta/progress/done/error`）
   - `delta`
   - `errorCode`
   - `message`
   - `recoverable`
   - `meta`

### 4.2 统一编排阶段
1. `validate`：参数与任务前置条件校验
2. `context`：按任务策略聚合上下文
3. `route`：canonical task + route 命中 + fallback/retry 链
4. `prompt`：技能模板优先，PromptBuilder 兜底
5. `generate`：统一流式 LLM 调用
6. `postprocess`：文本/JSON 结果标准化
7. `persist`：审计写入 + 草案池写入
8. `done`：统一完成事件

### 4.3 兼容策略
1. 旧入口保留：
   - `stream_ai_chapter_task`
   - `ai_generate_character`
   - `ai_generate_world_rule`
   - `ai_generate_plot_node`
   - `ai_scan_consistency`
2. 旧入口内部只做 DTO 映射并调用 `run_ai_task_pipeline`
3. 禁止旧入口保留独立业务分支

---

## 5. 数据模型与迁移方案
### 5.1 新增迁移文件
1. `src-tauri/migrations/project/0003_pipeline_draft_pool.sql`

### 5.2 新增表
1. `ai_pipeline_runs`
   - 用途：记录每次 pipeline 运行审计
   - 关键字段：`id/project_id/chapter_id/task_type/ui_action/status/error_code/duration_ms/created_at/completed_at`
2. `structured_draft_batches`
   - 用途：一次抽取批次头
   - 关键字段：`id/run_id/project_id/chapter_id/source_task_type/content_hash/status/created_at/updated_at`
3. `structured_draft_items`
   - 用途：草案池明细（关系/戏份/场景/设定/世界观）
   - 关键字段：`id/batch_id/run_id/project_id/chapter_id/draft_kind/source_label/target_label/normalized_key/confidence/occurrences/evidence_text/payload_json/status/applied_target_*`

### 5.3 索引与约束
1. `idx_sdi_project_chapter_kind (project_id, chapter_id, draft_kind)`
2. `idx_sdi_status_created (status, created_at DESC)`
3. `ux_sdi_project_kind_key_pending (project_id, draft_kind, normalized_key, status)`（用于 pending 去重）

### 5.4 去重键规则（应用层生成）
1. `relationship`：`rel:{min(a,b)}|{max(a,b)}|{relationship_type}`
2. `involvement`：`inv:{chapter_id}|{character}|{involvement_type}`
3. `scene`：`scene:{scene_label}|{scene_type}`
4. `setting`：`setting:{label}|{category}`
5. `worldview`：`worldview:{label}`

---

## 6. 按钮到任务映射（一期执行清单）
1. 续写章节 -> `chapter.continue`
2. 生成章节草稿 -> `chapter.draft`
3. 生成章节计划 -> `chapter.plan`
4. 改写选区 -> `chapter.rewrite`
5. 去 AI 味 -> `prose.naturalize`
6. 创建角色卡 -> `character.create`
7. 创建世界规则 -> `world.create_rule`
8. 创建剧情节点 -> `plot.create_node`
9. 一致性扫描 -> `consistency.scan`

---

## 7. 文件级实施计划（可直接执行）

## 7.1 WP-0 基线冻结
### 目标
1. 固化当前状态，避免实施中误判

### 文件
1. 不改业务代码，仅记录状态

### 操作
1. 记录当前变更
2. 记录关键命令基线结果

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; git status --short"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run typecheck:web"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run typecheck"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo check"
```

---

## 7.2 WP-1 落库迁移（0003）
### 目标
1. 新增 pipeline 审计与草案池数据表

### 文件
1. 新增：`src-tauri/migrations/project/0003_pipeline_draft_pool.sql`
2. 修改：`src-tauri/src/infra/migrator.rs`

### 操作
1. 编写 0003 DDL（全部 `IF NOT EXISTS`）
2. 在 `project_migrations()` 中注册 `0003_pipeline_draft_pool`
3. 新增迁移测试：
   - 表存在
   - 索引存在
   - pending 去重约束生效

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo test -- --nocapture"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo check"
```

---

## 7.3 WP-2 新增 AiPipelineService
### 目标
1. 在 Rust 内部建立统一编排服务

### 文件
1. 新增：`src-tauri/src/services/ai_pipeline_service.rs`
2. 修改：`src-tauri/src/services/mod.rs`
3. 修改：`src-tauri/src/state.rs`
4. 可选微调：`src-tauri/src/services/ai_service.rs`

### 操作
1. 定义输入 DTO、事件 DTO、结果 DTO
2. 实现 8 阶段执行骨架
3. 在 `AppState` 注入 `ai_pipeline_service`
4. 保持 `AiService` 作为 provider 执行器

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo check"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo test -- --nocapture"
```

---

## 7.4 WP-3 Command 收敛与兼容代理
### 目标
1. 新增统一 command，并让旧 command 全部代理

### 文件
1. 修改：`src-tauri/src/commands/ai_commands.rs`
2. 修改：`src-tauri/src/lib.rs`

### 操作
1. 新增 `run_ai_task_pipeline`
2. 新增 `cancel_ai_task_pipeline`
3. 旧 command 改为内部桥接调用统一 pipeline
4. 在 `lib.rs` 注册新 command

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo check"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo test -- --nocapture"
```

---

## 7.5 WP-4 前端统一 API 与事件协议
### 目标
1. 前端改为固定事件通道 + requestId 分发

### 文件
1. 新增：`src/api/pipelineApi.ts`
2. 修改：`src/api/aiApi.ts`
3. 可选：`src/api/tauriClient.ts`（日志字段增强）

### 操作
1. 封装 `runTaskPipeline`、`cancelTaskPipeline`
2. 统一订阅 `ai:pipeline:event`
3. 保留旧 API 方法名，对外兼容，内部转发到 pipelineApi

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run typecheck:web"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run typecheck"
```

---

## 7.6 WP-5 编辑器 9 按钮全量接入
### 目标
1. 所有按钮统一调用 pipeline

### 文件
1. 修改：`src/pages/Editor/EditorPage.tsx`
2. 修改：`src/components/ai/AiCommandBar.tsx`
3. 修改：`src/utils/taskRouting.ts`

### 操作
1. 将按钮调用改为统一 `runTaskPipeline`
2. 统一 `taskType` canonical
3. 统一处理 done/error，透出 `phase + errorCode`
4. 页面卸载和章节切换时调用 cancel

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run typecheck:web"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run typecheck"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run test -- tests/integration/mvp-closed-loop.test.ts"
```

---

## 7.7 WP-6 草案池与人工确认落库闭环
### 目标
1. 生成草案写入草案池，确认后再落正式表

### 文件
1. 修改：`src-tauri/src/services/context_service.rs`
2. 可选新增：`src-tauri/src/services/structured_draft_service.rs`（若希望职责分离）
3. 修改：`src/api/contextApi.ts`
4. 修改：`src/pages/Editor/EditorPage.tsx`

### 操作
1. 抽取后写 `structured_draft_batches/items`
2. `apply_structured_draft` 增加 item 维度参数或批次参数
3. 应用成功回写 item 状态与目标 id
4. 前端状态展示由“临时状态”改为“草案池真实状态”

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo test apply_structured_relationship_creates_and_reuses_relationship -- --nocapture"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo test -- --nocapture"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run typecheck:web"
```

---

## 7.8 WP-7 审计与可诊断性
### 目标
1. 所有任务可追溯 route/phase/error

### 文件
1. 修改：`src-tauri/src/services/ai_pipeline_service.rs`
2. 修改：`src-tauri/src/services/ai_service.rs`
3. 修改：`src/pages/Editor/EditorPage.tsx`

### 操作
1. `ai_pipeline_runs` 全量写入
2. 失败时写 `error_code/error_message/phase`
3. 前端错误提示展示 `phase + errorCode + 建议动作`

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge\src-tauri'; cargo check"
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; npm run typecheck:web"
```

---

## 7.9 WP-8 文档同步（强制）
### 目标
1. 文档反映真实实现

### 文件
1. 修改：`docs/runtime/runtime-process-spec.md`
2. 修改：`docs/api/api-integration-spec.md`
3. 可选：`docs/README.md`（如果新增了文档入口）

### 操作
1. 增加新 command 契约
2. 增加统一事件协议
3. 增加兼容命令代理说明
4. 增加草案池与人工确认流程说明

### 验证命令
```powershell
pwsh -NoProfile -Command "Set-Location 'F:\NovelForge'; rg -n 'run_ai_task_pipeline|cancel_ai_task_pipeline|ai:pipeline:event|structured_draft' docs src-tauri/src/lib.rs src/api -S"
```

---

## 8. 上线前数据校验脚本（SQL）
```sql
-- 1) migration version check
SELECT version FROM schema_migrations ORDER BY version;

-- 2) table check
SELECT name FROM sqlite_master
WHERE type='table'
  AND name IN ('ai_pipeline_runs','structured_draft_batches','structured_draft_items');

-- 3) index check
SELECT name FROM sqlite_master
WHERE type='index'
  AND name IN (
    'idx_sdi_project_chapter_kind',
    'idx_sdi_status_created',
    'ux_sdi_project_kind_key_pending'
  );

-- 4) orphan check
SELECT i.id
FROM structured_draft_items i
LEFT JOIN structured_draft_batches b ON b.id = i.batch_id
WHERE b.id IS NULL
LIMIT 20;

-- 5) pending duplicate check
SELECT project_id, draft_kind, normalized_key, COUNT(*) c
FROM structured_draft_items
WHERE status = 'pending'
GROUP BY project_id, draft_kind, normalized_key
HAVING c > 1;
```

---

## 9. 回滚 Runbook
### 9.1 L1 逻辑回滚（首选）
1. 关闭 feature flag：`pipeline.v1.enabled=false`
2. 前端回到旧入口调用
3. 新表保留，不删数据

### 9.2 L2 数据回滚（保留业务资产）
1. 清空新表：
   - `structured_draft_items`
   - `structured_draft_batches`
   - `ai_pipeline_runs`
2. 保留章节、角色、世界观、剧情等正式表

### 9.3 L3 文件级回滚（最后手段）
1. 用备份覆盖 `project.sqlite` 与 `novelforge.db`
2. 重启应用并保持旧链路

---

## 10. 一期验收门禁（DoD）
1. 数据层：
   - `0003_pipeline_draft_pool` 已执行
   - 新表与索引检查通过
2. 功能层：
   - 9 个按钮全部走统一 pipeline
   - 旧 command 可用且由统一 pipeline 驱动
3. 质量层：
   - 前端类型检查通过
   - Rust 编译与测试通过
   - `mvp-closed-loop.test.ts` 通过
4. 诊断层：
   - 失败可见 `phase + errorCode`
   - callback 警告显著减少（切页/热更场景）
5. 文档层：
   - runtime 与 api 文档已同步新链路

---

## 11. 建议提交节奏
1. `feat(db): add 0003 pipeline draft pool migration`
2. `feat(backend): add unified ai pipeline service and commands`
3. `feat(frontend): route editor ai actions to pipeline event bus`
4. `feat(context): persist structured draft batches and apply workflow`
5. `docs: sync runtime/api specs for ai pipeline v1`

---

## 12. 执行入口（建议顺序）
1. 从 WP-1 开始（迁移先行）
2. 再做 WP-2/WP-3（后端主链路）
3. 再做 WP-4/WP-5（前端接入）
4. 再做 WP-6/WP-7（草案闭环和诊断）
5. 最后 WP-8（文档同步）

> 注：每完成一个 WP 都必须立即执行该 WP 的验证命令，不允许跳过验证后连续堆叠改动。
