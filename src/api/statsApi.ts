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

export interface FeedbackEvent {
  id: string;
  projectId: string;
  chapterId?: string | null;
  eventType: string;
  ruleType: string;
  severity: string;
  conditionSummary: string;
  suggestedAction?: string | null;
  context?: Record<string, unknown> | null;
  status: string;
  resolvedAt?: string | null;
  resolvedBy?: string | null;
  resolutionNote?: string | null;
  createdAt: string;
  updatedAt: string;
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

export async function getFeedbackEvents(projectRoot: string): Promise<FeedbackEvent[]> {
  return invokeCommand<FeedbackEvent[]>("get_feedback_events", { projectRoot });
}

export async function acknowledgeFeedbackEvent(
  projectRoot: string,
  eventId: string,
): Promise<FeedbackEvent> {
  return invokeCommand<FeedbackEvent>("acknowledge_feedback_event", { projectRoot, eventId });
}

export async function resolveFeedbackEvent(
  projectRoot: string,
  eventId: string,
  note: string,
): Promise<FeedbackEvent> {
  return invokeCommand<FeedbackEvent>("resolve_feedback_event", { projectRoot, eventId, note });
}

export async function ignoreFeedbackEvent(
  projectRoot: string,
  eventId: string,
  reason: string,
): Promise<FeedbackEvent> {
  return invokeCommand<FeedbackEvent>("ignore_feedback_event", { projectRoot, eventId, reason });
}
