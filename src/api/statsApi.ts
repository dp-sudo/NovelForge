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
  blueprintProgress: number;
  completedBlueprintCount: number;
  totalBlueprintSteps: number;
  completedChapterCount?: number;
  recentChapters?: DashboardRecentChapter[];
}

export async function getDashboardStats(projectRoot: string): Promise<DashboardStats | null> {
  const raw = await invokeCommand<Omit<DashboardStats, "blueprintProgress">>("get_dashboard_stats", { projectRoot });
  const totalSteps = raw.totalBlueprintSteps > 0 ? raw.totalBlueprintSteps : 8;
  const blueprintProgress = Math.round((raw.completedBlueprintCount / totalSteps) * 100);
  return {
    ...raw,
    blueprintProgress,
  };
}
