-- Track provenance for promoted or AI-generated formal assets.
-- Migration: 0005_entity_provenance

CREATE TABLE IF NOT EXISTS entity_provenance (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_ref TEXT,
  request_id TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entity_provenance_project_entity
  ON entity_provenance(project_id, entity_type, entity_id);
