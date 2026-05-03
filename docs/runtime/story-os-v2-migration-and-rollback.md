# Story OS v2 迁移与回退说明

## 1. 迁移目标

本次迁移把 AI 治理主链路切换到 v2 表族：

- `story_os_v2_run_ledger`
- `story_os_v2_context_snapshots`
- `story_os_v2_review_work_items`
- `story_os_v2_polish_actions`

兼容保留：

- `ai_pipeline_runs`
- `ai_story_checkpoints`
- `ai_review_queue`

## 2. 执行方式

项目库打开时自动执行 `schema_migrations`：

- `0005_story_os_v2_governance.sql`

迁移为幂等建表（`CREATE TABLE IF NOT EXISTS`），可重复执行。

## 3. 数据策略

- 核心创作资产（章节/角色/世界/剧情/名词/叙事义务）不迁移、不改写。
- v1 AI 治理历史不强制搬迁；v2 运行后生成新治理记录。
- 审查状态更新会同时写入 `story_os_v2_polish_actions`。

## 4. 回退步骤（应急）

1. 停止应用，备份项目目录（含 `database/project.sqlite`）。
2. 重新部署上一版本程序。
3. 若必须清理 v2 数据，仅删除以下表内容（不建议删除核心资产）：
   - `story_os_v2_run_ledger`
   - `story_os_v2_context_snapshots`
   - `story_os_v2_review_work_items`
   - `story_os_v2_polish_actions`
4. 应用仍可通过兼容层读取 `ai_story_checkpoints` / `ai_review_queue`。

## 5. 风险点

- 若直接删除 v2 表结构，会触发完整性检查错误。
- 若跳过 `0005` 迁移，`run_ai_task_pipeline` 的 v2 审查链路将不可用。
