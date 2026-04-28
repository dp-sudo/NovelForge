import { invokeCommand } from "./tauriClient.js";

export interface TimelineEntry {
  chapterId: string;
  chapterIndex: number;
  title: string;
  summary: string;
  status: string;
  volumeId: string | null;
  volumeTitle: string | null;
  updatedAt: string;
}

export async function listTimelineEntries(projectRoot: string): Promise<TimelineEntry[]> {
  return invokeCommand<TimelineEntry[]>("list_timeline_entries", { projectRoot });
}

