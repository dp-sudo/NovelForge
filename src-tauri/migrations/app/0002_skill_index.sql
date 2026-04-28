CREATE TABLE IF NOT EXISTS skill_index (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  category TEXT NOT NULL DEFAULT 'utility',
  source TEXT NOT NULL DEFAULT 'user',
  version INTEGER NOT NULL DEFAULT 1,
  tags TEXT DEFAULT '[]',
  is_enabled INTEGER NOT NULL DEFAULT 1,
  file_path TEXT NOT NULL,
  file_hash TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
