-- 迁移：0007_blueprint_certainty_zones — Blueprint 确定性分区显式字段
-- 兼容历史库：若 0001 在老库中被跳过标记，先兜底创建 blueprint_steps。
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

ALTER TABLE blueprint_steps
  ADD COLUMN certainty_zones_json TEXT NOT NULL DEFAULT '';
