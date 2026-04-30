-- Add project-level AI strategy profile as runtime authority source.
-- Migration: 0004_ai_strategy_profile

ALTER TABLE projects ADD COLUMN ai_strategy_profile TEXT;
