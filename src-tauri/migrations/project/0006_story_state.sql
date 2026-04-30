-- Track dynamic story state ledger entries.
-- Migration: 0006_story_state

CREATE TABLE IF NOT EXISTS story_state (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  subject_type TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  scope TEXT NOT NULL,
  state_kind TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  source_chapter_id TEXT,
  status TEXT NOT NULL DEFAULT 'active',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_story_state_lookup
  ON story_state(project_id, subject_type, subject_id, state_kind, status);
