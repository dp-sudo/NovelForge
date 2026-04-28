-- NovelForge 项目数据库 v1.0
-- 迁移：0001_init — 基础表结构

CREATE TABLE IF NOT EXISTS schema_migrations (
  version TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  author TEXT,
  genre TEXT,
  narrative_pov TEXT,
  target_words INTEGER DEFAULT 0,
  current_words INTEGER DEFAULT 0,
  project_path TEXT NOT NULL,
  schema_version TEXT NOT NULL,
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
  volume_id TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, chapter_index)
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
  status TEXT NOT NULL DEFAULT 'planning',
  related_characters TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
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
  explanation TEXT NOT NULL,
  suggested_fix TEXT,
  status TEXT NOT NULL DEFAULT 'open',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS character_relationships (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  source_character_id TEXT NOT NULL,
  target_character_id TEXT NOT NULL,
  relationship_type TEXT NOT NULL,
  description TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (source_character_id) REFERENCES characters(id),
  FOREIGN KEY (target_character_id) REFERENCES characters(id)
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ai_requests (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  task_type TEXT NOT NULL,
  provider TEXT,
  model TEXT,
  prompt_preview TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  error_code TEXT,
  error_message TEXT,
  input_tokens INTEGER DEFAULT 0,
  output_tokens INTEGER DEFAULT 0,
  duration_ms INTEGER DEFAULT 0,
  created_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- 项目级 LLM 配置（支持跨项目共享的替代方案）
CREATE TABLE IF NOT EXISTS llm_providers (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  vendor TEXT NOT NULL,
  protocol TEXT NOT NULL,
  base_url TEXT NOT NULL,
  endpoint_path TEXT,
  api_key_secret_ref TEXT,
  auth_mode TEXT NOT NULL DEFAULT 'bearer',
  auth_header_name TEXT,
  anthropic_version TEXT,
  beta_headers TEXT,
  custom_headers TEXT,
  default_model TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  timeout_ms INTEGER NOT NULL DEFAULT 120000,
  connect_timeout_ms INTEGER NOT NULL DEFAULT 15000,
  max_retries INTEGER NOT NULL DEFAULT 2,
  model_refresh_mode TEXT NOT NULL DEFAULT 'registry',
  models_path TEXT,
  last_model_refresh_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS llm_models (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
  display_name TEXT,
  context_window INTEGER NOT NULL,
  max_output_tokens INTEGER,
  supports_streaming INTEGER NOT NULL DEFAULT 0,
  supports_tools INTEGER NOT NULL DEFAULT 0,
  supports_structured_output INTEGER NOT NULL DEFAULT 0,
  vision_support TEXT,
  input_price_per_million REAL,
  output_price_per_million REAL,
  caching_price_per_million REAL,
  max_batch_size INTEGER,
  status TEXT NOT NULL DEFAULT 'active',
  source TEXT NOT NULL DEFAULT 'registry',
  user_overridden INTEGER NOT NULL DEFAULT 0,
  last_seen_at TEXT,
  registry_version INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (provider_id) REFERENCES llm_providers(id)
);

CREATE TABLE IF NOT EXISTS llm_task_routes (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  task_type TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  fallback_provider_id TEXT,
  fallback_model_id TEXT,
  max_retries INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (provider_id) REFERENCES llm_providers(id),
  FOREIGN KEY (fallback_provider_id) REFERENCES llm_providers(id)
);

CREATE TABLE IF NOT EXISTS llm_model_registry_state (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL,
  etag TEXT,
  last_modified TEXT,
  current_version INTEGER NOT NULL DEFAULT 0,
  last_check_at TEXT,
  next_check_at TEXT,
  sync_status TEXT NOT NULL DEFAULT 'pending',
  error_message TEXT,
  FOREIGN KEY (provider_id) REFERENCES llm_providers(id)
);

CREATE TABLE IF NOT EXISTS narrative_obligations (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  obligation_type TEXT NOT NULL,
  description TEXT NOT NULL,
  planted_chapter_id TEXT,
  expected_payoff_chapter_id TEXT,
  actual_payoff_chapter_id TEXT,
  payoff_status TEXT NOT NULL DEFAULT 'open',
  severity TEXT NOT NULL DEFAULT 'medium',
  related_entities TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS snapshots (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  chapter_id TEXT,
  snapshot_type TEXT NOT NULL DEFAULT 'manual',
  title TEXT,
  file_path TEXT NOT NULL,
  note TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE TABLE IF NOT EXISTS volumes (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  description TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- 全文搜索索引（FTS5）
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
  entity_type UNINDEXED,
  entity_id UNINDEXED,
  title,
  body,
  tokenize = 'unicode61'
);

CREATE TABLE IF NOT EXISTS llm_model_refresh_logs (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL,
  refresh_type TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  models_added INTEGER NOT NULL DEFAULT 0,
  models_updated INTEGER NOT NULL DEFAULT 0,
  models_removed INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running',
  error_message TEXT,
  FOREIGN KEY (provider_id) REFERENCES llm_providers(id)
);
