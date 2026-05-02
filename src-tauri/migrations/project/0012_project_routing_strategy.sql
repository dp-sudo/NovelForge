-- Persist selected routing strategy at project level.
-- Migration: 0012_project_routing_strategy

ALTER TABLE projects ADD COLUMN routing_strategy_id TEXT;
