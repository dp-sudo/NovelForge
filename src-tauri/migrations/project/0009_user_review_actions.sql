-- Add user review trail table for approved/rejected/manual-edit actions.
-- Migration: 0009_user_review_actions

CREATE TABLE IF NOT EXISTS user_review_actions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  draft_item_id TEXT,
  action TEXT NOT NULL,
  reason TEXT,
  detail_json TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_user_review_actions_project_chapter_created
ON user_review_actions(project_id, chapter_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_user_review_actions_entity_created
ON user_review_actions(entity_type, entity_id, created_at DESC);
