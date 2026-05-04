# 资产变更历史 & 宪法冲突检测

本文档说明 NovelForge 新增的两个功能模块的设计与实现。

## 一、资产变更历史 (Asset Change History)

### 功能概述

资产变更历史功能为角色、世界设定、剧情节点等核心资产提供完整的变更审计轨迹。每次资产被创建、修改或删除时，系统都会自动记录：

- **谁**修改了资产（用户、AI 或系统）
- **何时**进行了修改
- **修改了什么**字段
- **修改前后**的值对比
- **为什么**修改（变更原因/上下文）
- 如果是 AI 修改，记录**任务类型**和**请求 ID**

### 数据库设计

#### `asset_change_history` 表

```sql
CREATE TABLE asset_change_history (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  asset_type TEXT NOT NULL,           -- 'character', 'world_rule', 'plot_node', etc.
  asset_id TEXT NOT NULL,              -- 对应资产的 ID
  asset_name TEXT NOT NULL,            -- 资产名称（冗余存储，便于查询）
  change_type TEXT NOT NULL,           -- 'create', 'update', 'delete'
  changed_by TEXT NOT NULL,            -- 'user', 'ai', 'system'
  ai_task_type TEXT,                   -- 如果是 AI 修改，记录任务类型
  ai_request_id TEXT,                  -- 关联到 ai_requests 表
  field_name TEXT,                     -- 被修改的字段名（update 时）
  old_value TEXT,                      -- 修改前的值
  new_value TEXT,                      -- 修改后的值
  change_reason TEXT,                  -- 变更原因/上下文
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (ai_request_id) REFERENCES ai_requests(id)
);
```

#### 索引

- `idx_asset_history_asset`: 按项目、资产类型、资产 ID 和时间查询
- `idx_asset_history_changed_by`: 按变更来源查询
- `idx_asset_history_ai_request`: 关联 AI 请求

### 前端 API

#### `assetHistoryApi.ts`

提供以下接口：

- `listAssetChangeHistory()` - 获取变更历史列表（支持筛选）
- `getAssetHistory()` - 获取特定资产的变更历史
- `getAssetHistorySummary()` - 获取统计摘要
- `recordAssetChange()` - 记录变更（通常由后端自动调用）
- `compareAssetVersions()` - 比较资产的两个版本

### 前端页面

#### `AssetHistoryPage.tsx`

功能特性：

1. **统计卡片**：显示总变更数、变更类型分布、变更来源分布
2. **筛选器**：按资产类型、变更来源、日期范围筛选
3. **变更列表**：
   - 显示每条变更的详细信息
   - 对于 update 操作，并排显示修改前后的值对比
   - 高亮显示变更类型和来源
   - 显示 AI 任务类型（如果适用）

### 使用场景

1. **审计追踪**：了解资产是如何演变的
2. **问题排查**：当发现资产数据异常时，追溯变更历史
3. **AI 行为分析**：查看 AI 对资产做了哪些修改
4. **版本对比**：比较资产在不同时间点的状态
5. **回滚参考**：为手动回滚提供数据参考

### 后端集成要点

后端服务需要在以下时机自动记录变更：

1. **CharacterService**：创建、更新、删除角色时
2. **WorldService**：创建、更新、删除世界规则时
3. **PlotService**：创建、更新、删除剧情节点时
4. **GlossaryService**：创建、更新、删除术语时
5. **AI Pipeline**：AI 生成或修改资产时，关联 `ai_request_id`

---

## 二、宪法冲突检测 (Constitution Conflict Detection)

### 功能概述

宪法冲突检测功能用于识别和管理多条宪法规则之间的潜在矛盾。系统支持：

- **自动检测**：通过 AI 分析规则之间的逻辑冲突
- **手动标记**：用户可以手动标记冲突
- **冲突分类**：直接矛盾、逻辑不一致、时间冲突等
- **解决追踪**：记录冲突的处理状态和解决方案

### 数据库设计

#### `constitution_conflicts` 表

```sql
CREATE TABLE constitution_conflicts (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  rule_id_a TEXT NOT NULL,             -- 第一条规则
  rule_id_b TEXT NOT NULL,             -- 第二条规则
  conflict_type TEXT NOT NULL,         -- 'direct_contradiction', 'logical_inconsistency', 'temporal_conflict'
  severity TEXT NOT NULL DEFAULT 'medium', -- 'low', 'medium', 'high'
  explanation TEXT NOT NULL,           -- 冲突说明
  ai_detected INTEGER NOT NULL DEFAULT 0, -- 是否由 AI 检测
  resolution_status TEXT NOT NULL DEFAULT 'open', -- 'open', 'acknowledged', 'resolved', 'false_positive'
  resolution_note TEXT,                -- 解决方案或说明
  detected_at TEXT NOT NULL,
  resolved_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (rule_id_a) REFERENCES story_constitution_rules(id),
  FOREIGN KEY (rule_id_b) REFERENCES story_constitution_rules(id),
  UNIQUE(rule_id_a, rule_id_b)         -- 防止重复记录同一对规则的冲突
);
```

#### `constitution_rule_tags` 表

用于辅助冲突检测的标签系统：

```sql
CREATE TABLE constitution_rule_tags (
  id TEXT PRIMARY KEY,
  rule_id TEXT NOT NULL,
  tag_type TEXT NOT NULL,              -- 'entity', 'temporal', 'constraint', 'theme'
  tag_value TEXT NOT NULL,             -- 标签值，如 'protagonist', 'chapter_5', 'forbidden'
  created_at TEXT NOT NULL,
  FOREIGN KEY (rule_id) REFERENCES story_constitution_rules(id) ON DELETE CASCADE,
  UNIQUE(rule_id, tag_type, tag_value)
);
```

### 冲突类型

1. **直接矛盾 (direct_contradiction)**
   - 示例：规则 A 要求"主角必须在第 5 章前知道真相"，规则 B 要求"主角不得在第 10 章前知道真相"

2. **逻辑不一致 (logical_inconsistency)**
   - 示例：规则 A 要求"所有魔法都需要咒语"，规则 B 允许"无声施法"

3. **时间冲突 (temporal_conflict)**
   - 示例：规则 A 要求"第 3 章必须发生战斗"，规则 B 要求"前 5 章不得出现暴力场景"

### 前端 API

#### `constitutionConflictApi.ts`

提供以下接口：

- `listConstitutionConflicts()` - 列出所有冲突（支持状态筛选）
- `detectConstitutionConflicts()` - 运行 AI 驱动的冲突检测
- `updateConflictResolution()` - 更新冲突解决状态
- `deleteConflict()` - 删除冲突记录
- `getRuleTags()` / `addRuleTag()` / `deleteRuleTag()` - 管理规则标签
- `getConflictSummary()` - 获取冲突统计摘要

### 前端页面

#### `ConstitutionConflictsPage.tsx`

功能特性：

1. **统计卡片**：
   - 总冲突数
   - 待处理冲突数
   - 严重程度分布
   - 冲突类型分布

2. **冲突检测按钮**：触发 AI 驱动的全量冲突检测

3. **状态筛选器**：按处理状态筛选冲突

4. **冲突列表**：
   - 显示两条冲突规则的完整内容
   - 显示冲突说明
   - 显示严重程度和状态标签
   - 对于待处理的冲突，提供"处理冲突"按钮

5. **处理对话框**：
   - 选择处理状态（已确认、已解决、误报）
   - 填写解决方案或说明

### 使用场景

1. **规则质量保证**：在创作初期发现规则矛盾
2. **规则维护**：当规则数量增多时，定期检测冲突
3. **AI 辅助审查**：利用 AI 发现人工难以察觉的逻辑冲突
4. **团队协作**：多人编写规则时，避免相互矛盾
5. **规则重构**：在修改规则前，了解可能的影响

### 后端集成要点

#### AI 冲突检测逻辑

后端需要实现 `detect_constitution_conflicts` 命令：

1. 获取所有活动的宪法规则
2. 构建提示词，要求 AI 分析规则之间的冲突
3. AI 返回冲突列表，包含：
   - 冲突的两条规则 ID
   - 冲突类型
   - 严重程度
   - 冲突说明
4. 将检测结果写入 `constitution_conflicts` 表
5. 返回检测摘要

#### 标签系统

标签系统用于辅助冲突检测：

- **实体标签 (entity)**：标记规则涉及的角色、地点等
- **时间标签 (temporal)**：标记规则涉及的章节、时间点
- **约束标签 (constraint)**：标记规则的约束类型（必须、禁止、建议）
- **主题标签 (theme)**：标记规则涉及的主题（暴力、爱情、魔法等）

通过标签，可以快速定位可能冲突的规则对，提高检测效率。

---

## 三、导航与访问

### 侧边栏菜单

两个新功能已添加到侧边栏导航：

- **宪法冲突** (⚠️) - 位于"宪法"之后
- **变更历史** (📋) - 位于"宪法冲突"之后

### 命令面板

按 `Ctrl+P` 打开命令面板，可以快速搜索：

- "宪法冲突检测" / "conflicts" / "冲突" / "矛盾" / "检测"
- "资产变更历史" / "history" / "历史" / "变更" / "审计" / "audit"

---

## 四、开发状态

### 已完成

- ✅ 数据库迁移文件 (`0008_asset_audit_and_constitution_conflicts.sql`)
- ✅ 前端 API 接口 (`assetHistoryApi.ts`, `constitutionConflictApi.ts`)
- ✅ 前端页面组件 (`AssetHistoryPage.tsx`, `ConstitutionConflictsPage.tsx`)
- ✅ 路由配置（`uiStore.ts`, `router.tsx`, `AppShell.tsx`）
- ✅ 导航菜单（`Sidebar.tsx`, `CommandPalette.tsx`）
- ✅ TypeScript 类型检查通过

### 待实现（后端）

需要在 Rust 后端实现以下内容：

1. **数据库迁移执行**
   - 运行 `0008_asset_audit_and_constitution_conflicts.sql`

2. **资产变更历史服务**
   - 在各资产服务中集成变更记录逻辑
   - 实现 `asset_history_service.rs`
   - 实现 Tauri 命令：
     - `list_asset_change_history`
     - `get_asset_history`
     - `get_asset_history_summary`
     - `record_asset_change`
     - `compare_asset_versions`

3. **宪法冲突检测服务**
   - 实现 `constitution_conflict_service.rs`
   - 实现 AI 驱动的冲突检测逻辑
   - 实现 Tauri 命令：
     - `list_constitution_conflicts`
     - `detect_constitution_conflicts`
     - `update_conflict_resolution`
     - `delete_constitution_conflict`
     - `get_rule_tags`
     - `add_rule_tag`
     - `delete_rule_tag`
     - `get_conflict_summary`

4. **AI Pipeline 集成**
   - 在 AI 修改资产时自动记录变更历史
   - 关联 `ai_request_id` 到变更记录

---

## 五、测试建议

### 资产变更历史测试

1. 创建一个角色，验证 `create` 记录
2. 修改角色的多个字段，验证 `update` 记录和值对比
3. 删除角色，验证 `delete` 记录
4. 使用 AI 生成角色，验证 `changed_by='ai'` 和 `ai_task_type`
5. 测试筛选器：按资产类型、来源、日期范围筛选
6. 测试统计摘要的准确性

### 宪法冲突检测测试

1. 创建两条明显矛盾的规则
2. 运行冲突检测，验证能否识别
3. 测试不同冲突类型的识别
4. 测试冲突处理流程：确认 → 解决 → 误报
5. 测试标签系统（如果实现）
6. 测试统计摘要的准确性

---

## 六、未来优化方向

### 资产变更历史

1. **可视化时间线**：以时间轴形式展示资产演变
2. **一键回滚**：支持将资产恢复到历史版本
3. **变更对比视图**：更直观的 diff 展示
4. **批量操作审计**：记录批量修改操作
5. **导出审计报告**：生成 PDF/Excel 格式的审计报告

### 宪法冲突检测

1. **实时检测**：在创建/修改规则时立即检测冲突
2. **智能建议**：AI 提供冲突解决建议
3. **规则依赖图**：可视化规则之间的关系
4. **冲突预警**：在 Pipeline 执行前检查相关规则冲突
5. **规则模板**：提供常见规则模板，减少冲突

---

## 七、总结

这两个功能模块为 NovelForge 提供了更强的**可审计性**和**规则一致性保障**：

- **资产变更历史**确保每一次修改都有迹可循，特别是 AI 的自动修改
- **宪法冲突检测**确保故事的最高权威层（宪法规则）内部保持一致

它们共同提升了系统的**可信度**和**可维护性**，使长篇创作过程更加可控。
