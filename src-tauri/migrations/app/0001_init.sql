-- NovelForge 应用级配置数据库 v1.0
-- 迁移：0001_init — 基础配置表（保存于 ~/.novelforge/novelforge.db）

CREATE TABLE IF NOT EXISTS schema_migrations (
  version TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

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
  model_name TEXT NOT NULL,
  display_name TEXT,
  context_window_tokens INTEGER,
  max_output_tokens INTEGER,
  input_modalities TEXT DEFAULT '[]',
  output_modalities TEXT DEFAULT '["text"]',
  supports_streaming INTEGER NOT NULL DEFAULT 0,
  supports_tools INTEGER NOT NULL DEFAULT 0,
  supports_json_object INTEGER NOT NULL DEFAULT 0,
  supports_json_schema INTEGER NOT NULL DEFAULT 0,
  supports_thinking INTEGER NOT NULL DEFAULT 0,
  supports_reasoning_effort INTEGER NOT NULL DEFAULT 0,
  supports_prompt_cache INTEGER NOT NULL DEFAULT 0,
  supports_batch INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'available',
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (provider_id) REFERENCES llm_providers(id)
);

CREATE TABLE IF NOT EXISTS llm_model_refresh_logs (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL,
  refresh_type TEXT NOT NULL,
  status TEXT NOT NULL,
  models_added INTEGER NOT NULL DEFAULT 0,
  models_updated INTEGER NOT NULL DEFAULT 0,
  models_removed INTEGER NOT NULL DEFAULT 0,
  error_message TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (provider_id) REFERENCES llm_providers(id)
);

CREATE TABLE IF NOT EXISTS llm_task_routes (
  id TEXT PRIMARY KEY,
  task_type TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
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
  registry_version TEXT,
  registry_updated_at TEXT,
  last_checked_at TEXT,
  last_applied_at TEXT,
  source TEXT NOT NULL DEFAULT 'bundled',
  signature_valid INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  error_message TEXT
);
