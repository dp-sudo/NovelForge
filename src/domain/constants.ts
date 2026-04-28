export const BLUEPRINT_STEP_KEYS = [
  "step-01-anchor",
  "step-02-genre",
  "step-03-premise",
  "step-04-characters",
  "step-05-world",
  "step-06-glossary",
  "step-07-plot",
  "step-08-chapters"
] as const;

export type BlueprintStepKey = (typeof BLUEPRINT_STEP_KEYS)[number];

export type BlueprintStepStatus = "not_started" | "in_progress" | "completed";

export type ChapterStatus = "planned" | "drafting" | "revising" | "completed" | "archived";

export type IssueSeverity = "low" | "medium" | "high" | "blocker";

export type IssueStatus = "open" | "ignored" | "fixed" | "false_positive";

export const PROJECT_SCHEMA_VERSION = "1.0.0";
export const PROJECT_APP_MIN_VERSION = "0.1.0";
