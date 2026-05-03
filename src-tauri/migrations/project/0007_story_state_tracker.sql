-- Story State Tracker: continuous state snapshots across chapters.
-- Tracks character state, plot progress, and world state per chapter.

CREATE TABLE IF NOT EXISTS story_state_snapshots (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT NOT NULL,
  snapshot_type TEXT NOT NULL DEFAULT 'post_chapter',
  notes TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_state_snapshots_project_chapter
ON story_state_snapshots(project_id, chapter_id, created_at DESC);

CREATE TABLE IF NOT EXISTS character_state_entries (
  id TEXT PRIMARY KEY,
  snapshot_id TEXT NOT NULL,
  character_id TEXT NOT NULL,
  location TEXT,
  emotional_state TEXT,
  arc_progress TEXT,
  knowledge_gained TEXT,
  relationships_changed TEXT,
  status_notes TEXT,
  FOREIGN KEY (snapshot_id) REFERENCES story_state_snapshots(id),
  FOREIGN KEY (character_id) REFERENCES characters(id)
);

CREATE INDEX IF NOT EXISTS idx_char_state_snapshot
ON character_state_entries(snapshot_id);

CREATE TABLE IF NOT EXISTS plot_state_entries (
  id TEXT PRIMARY KEY,
  snapshot_id TEXT NOT NULL,
  plot_node_id TEXT,
  progress_status TEXT NOT NULL DEFAULT 'not_started',
  tension_level INTEGER,
  open_threads TEXT,
  FOREIGN KEY (snapshot_id) REFERENCES story_state_snapshots(id)
);

CREATE INDEX IF NOT EXISTS idx_plot_state_snapshot
ON plot_state_entries(snapshot_id);

CREATE TABLE IF NOT EXISTS world_state_entries (
  id TEXT PRIMARY KEY,
  snapshot_id TEXT NOT NULL,
  world_rule_id TEXT,
  state_description TEXT NOT NULL,
  changed_in_chapter INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (snapshot_id) REFERENCES story_state_snapshots(id)
);

CREATE INDEX IF NOT EXISTS idx_world_state_snapshot
ON world_state_entries(snapshot_id);
