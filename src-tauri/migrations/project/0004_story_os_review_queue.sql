-- Story OS governance tables:
-- 1) ai_story_checkpoints: per-run contract/context/review snapshot
-- 2) ai_review_queue: pending manual polish items (default flow still allows direct persist)

CREATE TABLE IF NOT EXISTS ai_story_checkpoints (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  task_type TEXT NOT NULL,
  authority_layer TEXT NOT NULL,
  state_layer TEXT NOT NULL,
  capability_pack TEXT NOT NULL,
  review_gate TEXT NOT NULL,
  context_manifest_json TEXT NOT NULL DEFAULT '{}',
  review_checklist_json TEXT NOT NULL DEFAULT '[]',
  persisted_records_json TEXT NOT NULL DEFAULT '[]',
  persistence_mode TEXT NOT NULL DEFAULT 'auto_persist',
  status TEXT NOT NULL DEFAULT 'recorded',
  created_at TEXT NOT NULL,
  FOREIGN KEY (run_id) REFERENCES ai_pipeline_runs(id),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_story_checkpoint_project_created
ON ai_story_checkpoints(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_story_checkpoint_chapter_created
ON ai_story_checkpoints(chapter_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ai_review_queue (
  id TEXT PRIMARY KEY,
  checkpoint_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  task_type TEXT NOT NULL,
  title TEXT NOT NULL,
  severity TEXT NOT NULL,
  message TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (checkpoint_id) REFERENCES ai_story_checkpoints(id),
  FOREIGN KEY (run_id) REFERENCES ai_pipeline_runs(id),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_review_queue_project_status
ON ai_review_queue(project_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_review_queue_chapter_status
ON ai_review_queue(chapter_id, status, created_at DESC);
