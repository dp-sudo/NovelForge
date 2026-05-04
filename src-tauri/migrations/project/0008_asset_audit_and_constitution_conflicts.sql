-- Asset Change History & Constitution Conflict Detection
-- Migration: 0008_asset_audit_and_constitution_conflicts

-- ============================================================
-- 1. Asset Change History (资产变更历史)
-- ============================================================

-- 资产变更审计表：记录角色、世界设定等资产的所有变更
CREATE TABLE IF NOT EXISTS asset_change_history (
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

CREATE INDEX IF NOT EXISTS idx_asset_history_asset
ON asset_change_history(project_id, asset_type, asset_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_asset_history_changed_by
ON asset_change_history(project_id, changed_by, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_asset_history_ai_request
ON asset_change_history(ai_request_id);

-- ============================================================
-- 2. Constitution Conflict Detection (宪法冲突检测)
-- ============================================================

-- 宪法规则冲突检测表：记录规则之间的潜在矛盾
CREATE TABLE IF NOT EXISTS constitution_conflicts (
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

CREATE INDEX IF NOT EXISTS idx_constitution_conflicts_project_status
ON constitution_conflicts(project_id, resolution_status, severity, detected_at DESC);

CREATE INDEX IF NOT EXISTS idx_constitution_conflicts_rules
ON constitution_conflicts(rule_id_a, rule_id_b);

-- 宪法规则分类标签表：用于辅助冲突检测
CREATE TABLE IF NOT EXISTS constitution_rule_tags (
  id TEXT PRIMARY KEY,
  rule_id TEXT NOT NULL,
  tag_type TEXT NOT NULL,              -- 'entity', 'temporal', 'constraint', 'theme'
  tag_value TEXT NOT NULL,             -- 标签值，如 'protagonist', 'chapter_5', 'forbidden'
  created_at TEXT NOT NULL,
  FOREIGN KEY (rule_id) REFERENCES story_constitution_rules(id) ON DELETE CASCADE,
  UNIQUE(rule_id, tag_type, tag_value)
);

CREATE INDEX IF NOT EXISTS idx_constitution_tags_rule
ON constitution_rule_tags(rule_id);

CREATE INDEX IF NOT EXISTS idx_constitution_tags_type_value
ON constitution_rule_tags(tag_type, tag_value);
