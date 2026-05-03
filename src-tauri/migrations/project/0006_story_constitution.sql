-- Story Constitution: formalized rules extracted from blueprint and manual input.
-- Rules serve as the highest-authority constraints for AI generation.

CREATE TABLE IF NOT EXISTS story_constitution_rules (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  source_step_key TEXT,
  rule_type TEXT NOT NULL,
  rule_content TEXT NOT NULL,
  enforcement_level TEXT NOT NULL DEFAULT 'must',
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_constitution_rules_project
ON story_constitution_rules(project_id, is_active, rule_type);

CREATE TABLE IF NOT EXISTS constitution_violations (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  run_id TEXT,
  chapter_id TEXT,
  rule_id TEXT NOT NULL,
  violation_text TEXT NOT NULL,
  severity TEXT NOT NULL DEFAULT 'warning',
  resolution_status TEXT NOT NULL DEFAULT 'open',
  resolution_note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (rule_id) REFERENCES story_constitution_rules(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_constitution_violations_project_status
ON constitution_violations(project_id, resolution_status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_constitution_violations_chapter
ON constitution_violations(chapter_id, resolution_status, created_at DESC);
