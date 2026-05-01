-- Add promotion policy table for unified promotion validation.
-- Migration: 0005_promotion_policies

CREATE TABLE IF NOT EXISTS promotion_policies (
  id TEXT PRIMARY KEY,
  target_type TEXT NOT NULL,
  source_kind TEXT NOT NULL DEFAULT 'any',
  policy_mode TEXT NOT NULL DEFAULT 'allow',
  require_reason INTEGER NOT NULL DEFAULT 0,
  enabled INTEGER NOT NULL DEFAULT 1,
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(target_type, source_kind)
);

INSERT OR IGNORE INTO promotion_policies (
  id, target_type, source_kind, policy_mode, require_reason, enabled, notes, created_at, updated_at
) VALUES
  ('promotion-policy-character-any', 'character', 'any', 'allow', 0, 1, '默认允许角色晋升', '2026-05-02T00:00:00Z', '2026-05-02T00:00:00Z'),
  ('promotion-policy-world-rule-any', 'world_rule', 'any', 'allow', 0, 1, '默认允许世界规则晋升', '2026-05-02T00:00:00Z', '2026-05-02T00:00:00Z'),
  ('promotion-policy-plot-node-any', 'plot_node', 'any', 'allow', 0, 1, '默认允许剧情节点晋升', '2026-05-02T00:00:00Z', '2026-05-02T00:00:00Z'),
  ('promotion-policy-glossary-term-any', 'glossary_term', 'any', 'allow', 0, 1, '默认允许术语晋升', '2026-05-02T00:00:00Z', '2026-05-02T00:00:00Z'),
  ('promotion-policy-character-relationship-any', 'character_relationship', 'any', 'allow', 0, 1, '默认允许关系晋升', '2026-05-02T00:00:00Z', '2026-05-02T00:00:00Z'),
  ('promotion-policy-involvement-any', 'involvement', 'any', 'allow', 0, 1, '默认允许戏份晋升', '2026-05-02T00:00:00Z', '2026-05-02T00:00:00Z'),
  ('promotion-policy-scene-any', 'scene', 'any', 'allow', 0, 1, '默认允许场景晋升', '2026-05-02T00:00:00Z', '2026-05-02T00:00:00Z');
