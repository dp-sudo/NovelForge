-- Add feedback event ledger and post-task audit column.
-- Migration: 0010_feedback_events

CREATE TABLE IF NOT EXISTS feedback_events (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  event_type TEXT NOT NULL,
  rule_type TEXT NOT NULL,
  severity TEXT NOT NULL DEFAULT 'info',
  condition_summary TEXT NOT NULL,
  suggested_action TEXT,
  context_json TEXT,
  status TEXT NOT NULL DEFAULT 'open',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_feedback_events_project_status_created
ON feedback_events(project_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_feedback_events_rule_created
ON feedback_events(rule_type, created_at DESC);

ALTER TABLE ai_pipeline_runs ADD COLUMN post_task_results TEXT;
