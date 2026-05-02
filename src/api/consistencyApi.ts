import { invokeCommand } from "./tauriClient.js";
import { runModuleAiTask } from "./moduleAiApi.js";

export interface ConsistencyIssueRow {
  id: string;
  issueType: string;
  severity: string;
  chapterId: string;
  sourceText: string;
  explanation: string;
  suggestedFix: string;
  status: string;
}

export async function scanChapterConsistency(
  chapterId: string,
  projectRoot: string
): Promise<ConsistencyIssueRow[]> {
  return invokeCommand<ConsistencyIssueRow[]>("scan_chapter_consistency", {
    projectRoot,
    input: { chapterId },
  });
}

export async function scanFullConsistency(projectRoot: string): Promise<ConsistencyIssueRow[]> {
  return invokeCommand<ConsistencyIssueRow[]>("scan_full_consistency", { projectRoot });
}

export async function listConsistencyIssues(projectRoot: string): Promise<ConsistencyIssueRow[]> {
  return invokeCommand<ConsistencyIssueRow[]>("list_consistency_issues", { projectRoot });
}

export async function updateIssueStatus(issueId: string, status: string, projectRoot: string): Promise<void> {
  await invokeCommand<void>("update_issue_status", { projectRoot, issueId, status });
}

export interface AiConsistencyInput {
  projectRoot: string;
  chapterId: string;
  chapterContent: string;
}

export async function aiScanConsistency(input: AiConsistencyInput): Promise<string> {
  return runModuleAiTask({
    projectRoot: input.projectRoot,
    taskType: "consistency.scan",
    chapterId: input.chapterId,
    chapterContent: input.chapterContent,
    persistMode: "derived_review",
    automationTier: "confirm",
    uiAction: "ai_scan_consistency",
  });
}
