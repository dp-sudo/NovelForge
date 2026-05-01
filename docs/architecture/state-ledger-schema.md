# State Ledger Structured Schema（MVP 第三阶段）

更新时间：2026-05-02

## 1. 目标

在不破坏既有 `story_state` 表结构的前提下，统一落地可机器消费的结构化状态 taxonomy，并覆盖第三阶段高阶分类闭环：

1. `character.emotion`（人物情绪）
2. `scene.environment`（场景环境）
3. `relationship.temperature`（关系温度）
4. `character.action`（角色动作状态）
5. `character.appearance`（角色外观变化）
6. `character.knowledge`（角色已知信息边界）
7. `scene.danger_level`（场景危险等级）
8. `scene.spatial_constraint`（场景空间约束）

## 2. 数据落盘策略

`story_state` 表沿用既有字段：

1. `subject_type`
2. `subject_id`
3. `scope`
4. `state_kind`
5. `payload_json`

结构化语义写入 `payload_json`，统一补齐：

1. `schemaVersion`：当前为 `1`
2. `category`：
   - `emotion`
   - `scene_environment`
   - `relationship_temperature`
   - `character_action`
   - `character_appearance`
   - `character_knowledge`
   - `scene_danger_level`
   - `scene_spatial_constraint`
   - `generic`
3. `value`：结构化业务负载（按 category 区分）

兼容要求：

1. 旧 payload 顶层字段保留，不做破坏性改写。
2. 若旧记录缺少结构化字段，读取时自动补齐 `schemaVersion/category/value`。

## 3. 分类规则（taxonomy）

按 `subject_type + state_kind` 判定：

1. `character + emotion` -> `emotion`
2. `scene + environment` -> `scene_environment`
3. `relationship + temperature` -> `relationship_temperature`
4. `character + action` -> `character_action`
5. `character + appearance` -> `character_appearance`
6. `character + knowledge` -> `character_knowledge`
7. `scene + danger_level` -> `scene_danger_level`
8. `scene + spatial_constraint` -> `scene_spatial_constraint`
9. 其他组合 -> `generic`

## 4. 写回与消费闭环

1. 抽取/编排阶段：技能 `stateWrites` 声明目标状态键。
2. 运行时写回：`runtime_state_writer` 为高阶分类输出结构化 payload，并写入 `story_state`。
3. 上下文消费：
   - `ContextService.get_state_summary()`
   - `ContextService.collect_editor_context()`
   在读取时均可见新 taxonomy 结构化字段。
4. 编辑器可见性：`EditorContextPanel` 至少在 JSON 预览中可见高阶状态。

## 5. 第三阶段完成边界

当前已完成分类定义、写回、抽取声明、上下文消费与编辑器可见性闭环。后续优化方向仅为抽取精度增强（如语义模型增强），不影响现有结构化 schema 稳定性。
