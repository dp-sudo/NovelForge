# State Ledger Structured Schema（MVP 第一阶段）

更新时间：2026-05-01

## 1. 目标

在不破坏既有 `story_state` 表结构的前提下，为运行时状态写回补齐可机器消费的结构化语义，优先覆盖：

1. `character.emotion`（人物情绪）
2. `scene.environment`（场景环境）
3. `relationship.temperature`（关系温度）

## 2. 数据落盘策略

`story_state` 表继续沿用既有字段：

1. `subject_type`
2. `subject_id`
3. `scope`
4. `state_kind`
5. `payload_json`

结构化语义写入 `payload_json`，统一补齐以下字段：

1. `schemaVersion`：当前为 `1`
2. `category`：`emotion | scene_environment | relationship_temperature | generic`
3. `value`：结构化业务负载（按 category 区分）

兼容要求：

1. 旧 payload 的原始顶层字段保留，不做破坏性改写。
2. 若旧记录缺少结构化字段，读取时自动补齐 `schemaVersion/category/value`。

## 3. 分类规则（taxonomy）

按 `subject_type + state_kind` 判定：

1. `character + emotion` -> `emotion`
2. `scene + environment` -> `scene_environment`
3. `relationship + temperature` -> `relationship_temperature`
4. 其他组合 -> `generic`

## 4. 最小闭环链路

1. 抽取/编排阶段：技能 `stateWrites` 声明目标状态键。
2. 运行时写回：`runtime_state_writer` 产出结构化 payload 并写入 `story_state`。
3. 上下文消费：`ContextService.get_state_summary()` / `collect_editor_context()` 读取时确保结构化字段可见。

## 5. 当前范围与后续扩展

当前仅保证 MVP 必需的 3 类语义状态闭环。动作、着装、公开信息边界等状态，作为下一阶段扩展分类进入同一 schemaVersion 体系。

