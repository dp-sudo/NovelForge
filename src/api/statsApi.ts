import { invokeCommand } from "./tauriClient.js";

export interface DashboardRecentChapter {
  id: string;
  title: string;
  updatedAt: string;
}

export interface DashboardStats {
  totalWords: number;
  chapterCount: number;
  characterCount: number;
  worldRuleCount: number;
  plotNodeCount: number;
  openIssueCount: number;
  blueprintProgress?: number; // Optional as we now compute it on client
  completedBlueprintCount: number;
  totalBlueprintSteps: number;
  completedChapterCount?: number;
  recentChapters?: DashboardRecentChapter[];
}

export async function getDashboardStats(projectRoot: string): Promise<DashboardStats | null> {
  return invokeCommand<DashboardStats>("get_dashboard_stats", { projectRoot });
}
