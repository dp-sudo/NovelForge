-- Add feedback event lifecycle fields.
-- Migration: 0011_feedback_event_lifecycle

ALTER TABLE feedback_events ADD COLUMN resolved_at TEXT;
ALTER TABLE feedback_events ADD COLUMN resolved_by TEXT;
ALTER TABLE feedback_events ADD COLUMN resolution_note TEXT;
