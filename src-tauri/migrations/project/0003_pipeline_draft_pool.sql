-- Add AI pipeline run audit and structured draft pool tables.
-- Migration: 0003_pipeline_draft_pool

CREATE TABLE IF NOT EXISTS ai_pipeline_runs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  task_type TEXT NOT NULL,
  ui_action TEXT,
  status TEXT NOT NULL DEFAULT 'running',
  phase TEXT,
  error_code TEXT,
  error_message TEXT,
  duration_ms INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE TABLE IF NOT EXISTS structured_draft_batches (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  chapter_id TEXT NOT NULL,
  source_task_type TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (run_id) REFERENCES ai_pipeline_runs(id),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE TABLE IF NOT EXISTS structured_draft_items (
  id TEXT PRIMARY KEY,
  batch_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  chapter_id TEXT NOT NULL,
  draft_kind TEXT NOT NULL,
  source_label TEXT NOT NULL,
  target_label TEXT,
  normalized_key TEXT NOT NULL,
  confidence REAL,
  occurrences INTEGER NOT NULL DEFAULT 1,
  evidence_text TEXT,
  payload_json TEXT NOT NULL DEFAULT '{}',
  status TEXT NOT NULL DEFAULT 'pending',
  applied_target_type TEXT,
  applied_target_id TEXT,
  applied_target_field TEXT,
  applied_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (batch_id) REFERENCES structured_draft_batches(id),
  FOREIGN KEY (run_id) REFERENCES ai_pipeline_runs(id),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_sdi_project_chapter_kind
ON structured_draft_items(project_id, chapter_id, draft_kind);

CREATE INDEX IF NOT EXISTS idx_sdi_status_created
ON structured_draft_items(status, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS ux_sdi_project_kind_key_pending
ON structured_draft_items(project_id, draft_kind, normalized_key, status);
