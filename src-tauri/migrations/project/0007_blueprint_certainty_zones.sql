-- 迁移：0007_blueprint_certainty_zones — Blueprint 确定性分区显式字段
ALTER TABLE blueprint_steps
  ADD COLUMN certainty_zones_json TEXT NOT NULL DEFAULT '';
