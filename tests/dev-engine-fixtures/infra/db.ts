import fs from "node:fs/promises";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";

import { AppError } from "../../../src/errors/app-error.js";
import { nowIso } from "./time.js";

const SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS schema_migrations (
  version TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  author TEXT,
  genre TEXT,
  target_words INTEGER DEFAULT 0,
  current_words INTEGER DEFAULT 0,
  project_path TEXT NOT NULL,
  schema_version TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS blueprint_steps (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  step_key TEXT NOT NULL,
  title TEXT NOT NULL,
  content TEXT,
  content_path TEXT,
  status TEXT NOT NULL DEFAULT 'not_started',
  ai_generated INTEGER NOT NULL DEFAULT 0,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, step_key)
);

CREATE TABLE IF NOT EXISTS characters (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  name TEXT NOT NULL,
  aliases TEXT,
  role_type TEXT NOT NULL,
  age TEXT,
  gender TEXT,
  identity_text TEXT,
  appearance TEXT,
  motivation TEXT,
  desire TEXT,
  fear TEXT,
  flaw TEXT,
  arc_stage TEXT,
  locked_fields TEXT,
  notes TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS world_rules (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  category TEXT NOT NULL,
  description TEXT NOT NULL,
  constraint_level TEXT NOT NULL DEFAULT 'normal',
  related_entities TEXT,
  examples TEXT,
  contradiction_policy TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS glossary_terms (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  term TEXT NOT NULL,
  term_type TEXT NOT NULL,
  aliases TEXT,
  description TEXT,
  locked INTEGER NOT NULL DEFAULT 0,
  banned INTEGER NOT NULL DEFAULT 0,
  preferred_usage TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, term)
);

CREATE TABLE IF NOT EXISTS plot_nodes (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  node_type TEXT NOT NULL,
  sort_order INTEGER NOT NULL,
  goal TEXT,
  conflict TEXT,
  emotional_curve TEXT,
  status TEXT NOT NULL DEFAULT 'planned',
  related_characters TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chapters (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_index INTEGER NOT NULL,
  title TEXT NOT NULL,
  summary TEXT,
  status TEXT NOT NULL DEFAULT 'drafting',
  target_words INTEGER DEFAULT 0,
  current_words INTEGER DEFAULT 0,
  content_path TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, chapter_index)
);

CREATE TABLE IF NOT EXISTS chapter_links (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT NOT NULL,
  target_type TEXT NOT NULL,
  target_id TEXT NOT NULL,
  relation_type TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS consistency_issues (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  issue_type TEXT NOT NULL,
  severity TEXT NOT NULL,
  chapter_id TEXT,
  source_text TEXT,
  source_start INTEGER,
  source_end INTEGER,
  related_asset_type TEXT,
  related_asset_id TEXT,
  explanation TEXT NOT NULL,
  suggested_fix TEXT,
  status TEXT NOT NULL DEFAULT 'open',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ai_requests (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  task_type TEXT NOT NULL,
  provider TEXT,
  model TEXT,
  prompt_preview TEXT,
  status TEXT NOT NULL,
  error_code TEXT,
  error_message TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
  entity_type,
  entity_id,
  title,
  body,
  tokenize = 'unicode61'
);
`;

export function getDatabasePath(projectRoot: string): string {
  return path.join(projectRoot, "database", "project.sqlite");
}

export async function initializeDatabase(projectRoot: string): Promise<void> {
  const dbPath = getDatabasePath(projectRoot);
  await fs.mkdir(path.dirname(dbPath), { recursive: true });
  const db = new DatabaseSync(dbPath);
  try {
    db.exec(SCHEMA_SQL);
    db.prepare(
      "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?, ?)"
    ).run("0001_init", nowIso());
    db.prepare(
      "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?, ?)"
    ).run("0002_fts", nowIso());
  } finally {
    db.close();
  }
}

export function openDatabase(projectRoot: string): DatabaseSync {
  const dbPath = getDatabasePath(projectRoot);
  try {
    return new DatabaseSync(dbPath);
  } catch (error) {
    throw new AppError({
      code: "DB_OPEN_FAILED",
      message: "数据库打开失败",
      detail: error instanceof Error ? error.message : String(error),
      recoverable: false,
      suggestedAction: "请检查 database/project.sqlite 是否存在并可读写"
    });
  }
}
