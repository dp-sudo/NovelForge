-- Add feedback rule table for structured bottom-up feedback triggers.
-- Migration: 0006_feedback_rules

CREATE TABLE IF NOT EXISTS feedback_rules (
  id TEXT PRIMARY KEY,
  rule_type TEXT NOT NULL UNIQUE,
  threshold_value INTEGER NOT NULL DEFAULT 0,
  enabled INTEGER NOT NULL DEFAULT 1,
  suggestion_template TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

ALTER TABLE llm_task_routes ADD COLUMN post_tasks_json TEXT NOT NULL DEFAULT '[]';

INSERT OR IGNORE INTO feedback_rules (
  id, rule_type, threshold_value, enabled, suggestion_template, created_at, updated_at
) VALUES
  (
    'feedback-rule-character-overflow',
    'character_overflow',
    10,
    1,
    '角色数量超过阈值，建议回收边缘角色并强化核心角色关系。',
    '2026-05-02T00:00:00Z',
    '2026-05-02T00:00:00Z'
  ),
  (
    'feedback-rule-relationship-complexity',
    'relationship_complexity',
    30,
    1,
    '关系网络复杂度升高，建议拆分卷级关系主线并补充关系图约束。',
    '2026-05-02T00:00:00Z',
    '2026-05-02T00:00:00Z'
  ),
  (
    'feedback-rule-foreshadow-unfulfilled',
    'foreshadow_unfulfilled',
    1,
    1,
    '检测到伏笔兑现风险，建议新增回收章节计划或调整窗口目标。',
    '2026-05-02T00:00:00Z',
    '2026-05-02T00:00:00Z'
  );
