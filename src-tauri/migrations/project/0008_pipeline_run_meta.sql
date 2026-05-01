-- Add pipeline run meta JSON for route/model-pool decision tracing.
-- Migration: 0008_pipeline_run_meta

ALTER TABLE ai_pipeline_runs ADD COLUMN meta_json TEXT;

