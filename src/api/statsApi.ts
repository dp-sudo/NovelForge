import { invokeCommand } from "./tauriClient.js";

export interface DashboardStats {
  totalWords: number;
  chapterCount: number;
  characterCount: number;
  worldRuleCount: number;
  plotNodeCount: number;
  openIssueCount: number;
  blueprintProgress: number;
}

export async function getDashboardStats(projectRoot: string): Promise<DashboardStats | null> {
  return invokeCommand<DashboardStats>("get_dashboard_stats", { projectRoot });
}
