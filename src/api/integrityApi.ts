import { invokeCommand } from "./tauriClient.js";

export interface IntegrityIssue {
  severity: string;
  category: string;
  message: string;
  detail: string | null;
  autoFixable: boolean;
}

export interface IntegritySummary {
  chaptersOk: number;
  chaptersMissing: number;
  orphanDrafts: number;
  schemaVersion: string;
}

export interface IntegrityReport {
  status: string;
  issues: IntegrityIssue[];
  summary: IntegritySummary;
}

export async function checkProjectIntegrity(projectRoot: string): Promise<IntegrityReport> {
  return invokeCommand<IntegrityReport>("check_project_integrity", { projectRoot });
}
