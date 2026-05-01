-- Add model pool routing compatibility layer.
-- Migration: 0004_model_pools

CREATE TABLE IF NOT EXISTS llm_model_pools (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  role TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  entries_json TEXT NOT NULL DEFAULT '[]',
  fallback_pool_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_llm_model_pools_role
ON llm_model_pools(role);

ALTER TABLE llm_task_routes ADD COLUMN model_pool_id TEXT;
ALTER TABLE llm_task_routes ADD COLUMN fallback_model_pool_id TEXT;

CREATE INDEX IF NOT EXISTS idx_llm_task_routes_model_pool_id
ON llm_task_routes(model_pool_id);

CREATE INDEX IF NOT EXISTS idx_llm_task_routes_fallback_model_pool_id
ON llm_task_routes(fallback_model_pool_id);

UPDATE llm_task_routes
SET model_pool_id = CASE
  WHEN task_type IN ('chapter.plan', 'blueprint.generate_step') THEN 'planner'
  WHEN task_type IN ('chapter.draft', 'chapter.continue', 'chapter.rewrite', 'prose.naturalize') THEN 'drafter'
  WHEN task_type IN ('consistency.scan', 'timeline.review', 'relationship.review', 'dashboard.review', 'export.review') THEN 'reviewer'
  WHEN task_type IN ('character.create', 'world.create_rule', 'plot.create_node', 'glossary.create_term') THEN 'extractor'
  WHEN task_type IN ('narrative.create_obligation') THEN 'state'
  ELSE NULL
END
WHERE model_pool_id IS NULL OR TRIM(model_pool_id) = '';

