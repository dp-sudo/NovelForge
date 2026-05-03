-- Story OS v2 governance tables.
-- Keep legacy audit tables for compatibility, route new governance writes here.

CREATE TABLE IF NOT EXISTS story_os_v2_run_ledger (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  task_type TEXT NOT NULL,
  ui_action TEXT,
  authority_layer TEXT NOT NULL,
  state_layer TEXT NOT NULL,
  capability_pack TEXT NOT NULL,
  review_gate TEXT NOT NULL,
  provider_id TEXT,
  model_id TEXT,
  status TEXT NOT NULL DEFAULT 'running',
  phase TEXT NOT NULL DEFAULT 'validate',
  error_code TEXT,
  error_message TEXT,
  output_text_length INTEGER NOT NULL DEFAULT 0,
  persisted_records_json TEXT NOT NULL DEFAULT '[]',
  review_attention_count INTEGER NOT NULL DEFAULT 0,
  requires_human_review INTEGER NOT NULL DEFAULT 0,
  checkpoint_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_story_os_v2_run_ledger_project_created
ON story_os_v2_run_ledger(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_story_os_v2_run_ledger_project_phase
ON story_os_v2_run_ledger(project_id, phase, status);

CREATE TABLE IF NOT EXISTS story_os_v2_context_snapshots (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  task_type TEXT NOT NULL,
  compile_strategy TEXT NOT NULL,
  sources_json TEXT NOT NULL DEFAULT '{}',
  trimming_json TEXT NOT NULL DEFAULT '{}',
  priority_json TEXT NOT NULL DEFAULT '[]',
  conflict_resolution_json TEXT NOT NULL DEFAULT '{}',
  token_budget_json TEXT NOT NULL DEFAULT '{}',
  compiled_manifest_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  FOREIGN KEY (run_id) REFERENCES story_os_v2_run_ledger(id),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_story_os_v2_context_snapshots_run
ON story_os_v2_context_snapshots(run_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_story_os_v2_context_snapshots_project
ON story_os_v2_context_snapshots(project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS story_os_v2_review_work_items (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  checkpoint_id TEXT,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  task_type TEXT NOT NULL,
  checklist_key TEXT NOT NULL,
  title TEXT NOT NULL,
  severity TEXT NOT NULL,
  message TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  reviewer_note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (run_id) REFERENCES story_os_v2_run_ledger(id),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_story_os_v2_review_work_project_status
ON story_os_v2_review_work_items(project_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_story_os_v2_review_work_task_status
ON story_os_v2_review_work_items(project_id, task_type, status, created_at DESC);

CREATE TABLE IF NOT EXISTS story_os_v2_polish_actions (
  id TEXT PRIMARY KEY,
  work_item_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  action_type TEXT NOT NULL,
  from_status TEXT,
  to_status TEXT,
  note TEXT,
  operator TEXT NOT NULL DEFAULT 'human',
  created_at TEXT NOT NULL,
  FOREIGN KEY (work_item_id) REFERENCES story_os_v2_review_work_items(id),
  FOREIGN KEY (run_id) REFERENCES story_os_v2_run_ledger(id),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_story_os_v2_polish_actions_work_item
ON story_os_v2_polish_actions(work_item_id, created_at DESC);
