import { invokeCommand, logUI } from "./tauriClient.js";
import type { ChapterInput } from "../domain/types.js";
import type { ChapterStatus } from "../domain/constants.js";


export interface ChapterRecord {
  id: string;
  chapterIndex: number;
  title: string;
  summary: string;
  status: ChapterStatus;
  targetWords: number;
  currentWords: number;
  contentPath: string;
  volumeId?: string | null;
  version: number;
  updatedAt: string;
}

export interface SaveChapterOutput {
  currentWords: number;
  version: number;
  updatedAt: string;
}

export interface RecoverDraftResult {
  hasNewerDraft: boolean;
  draftContent?: string;
}

function createClientRequestId(prefix: string): string {
  const randomPart = typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return `${prefix}-${randomPart}`;
}

export async function listChapters(projectRoot: string): Promise<ChapterRecord[]> {
  return invokeCommand<ChapterRecord[]>("list_chapters", { projectRoot });
}

export async function createChapter(input: ChapterInput, projectRoot: string): Promise<ChapterRecord> {
  return invokeCommand<ChapterRecord>("create_chapter", { input: { projectRoot, input } });
}

export async function saveChapterContent(
  chapterId: string,
  content: string,
  projectRoot: string
): Promise<SaveChapterOutput> {
  const requestId = createClientRequestId("save");
  logUI("SAVE.START", `requestId=${requestId} chapterId=${chapterId}`);
  try {
    const result = await invokeCommand<SaveChapterOutput>("save_chapter_content", {
      input: { projectRoot, chapterId, content, requestId }
    });
    logUI("SAVE.DONE", `requestId=${requestId} chapterId=${chapterId} version=${result.version}`);
    return result;
  } catch (error) {
    logUI("SAVE.ERROR", `requestId=${requestId} chapterId=${chapterId}`);
    throw error;
  }
}

export async function autosaveDraft(
  chapterId: string,
  content: string,
  projectRoot: string
): Promise<string> {
  const requestId = createClientRequestId("autosave");
  logUI("AUTOSAVE.START", `requestId=${requestId} chapterId=${chapterId}`);
  try {
    const result = await invokeCommand<string>("autosave_draft", {
      input: { projectRoot, chapterId, content, requestId }
    });
    logUI("AUTOSAVE.DONE", `requestId=${requestId} chapterId=${chapterId}`);
    return result;
  } catch (error) {
    logUI("AUTOSAVE.ERROR", `requestId=${requestId} chapterId=${chapterId}`);
    throw error;
  }
}

export async function recoverDraft(chapterId: string, projectRoot: string): Promise<RecoverDraftResult> {
  return invokeCommand<RecoverDraftResult>("recover_draft", {
    input: { projectRoot, chapterId }
  });
}

export async function readChapterContent(chapterId: string, projectRoot: string): Promise<string> {
  return invokeCommand<string>("read_chapter_content", {
    projectRoot,
    chapterId,
  });
}

export async function deleteChapter(id: string, projectRoot: string): Promise<void> {
  await invokeCommand<void>("delete_chapter", { projectRoot, input: { id } });
}

export interface SnapshotRecord {
  id: string;
  chapterId: string | null;
  snapshotType: string;
  title: string | null;
  filePath: string;
  note: string | null;
  createdAt: string;
}

export async function createSnapshot(projectRoot: string, chapterId: string, title?: string, note?: string): Promise<SnapshotRecord> {
  return invokeCommand<SnapshotRecord>("create_snapshot", { projectRoot, chapterId, title, note });
}

export async function listSnapshots(projectRoot: string, chapterId?: string): Promise<SnapshotRecord[]> {
  return invokeCommand<SnapshotRecord[]>("list_snapshots", { projectRoot, chapterId });
}

export async function readSnapshotContent(projectRoot: string, snapshotId: string): Promise<string> {
  return invokeCommand<string>("read_snapshot_content", { projectRoot, snapshotId });
}

export interface VolumeRecord {
  id: string;
  title: string;
  sortOrder: number;
  description: string | null;
  chapterCount: number;
  createdAt: string;
  updatedAt: string;
}

export async function listVolumes(projectRoot: string): Promise<VolumeRecord[]> {
  return invokeCommand<VolumeRecord[]>("list_volumes", { projectRoot });
}

export async function createVolume(projectRoot: string, title: string, description?: string): Promise<string> {
  return invokeCommand<string>("create_volume", { projectRoot, input: { title, description } });
}

export async function deleteVolume(projectRoot: string, id: string): Promise<void> {
  return invokeCommand<void>("delete_volume", { projectRoot, id });
}

export async function assignChapterVolume(projectRoot: string, chapterId: string, volumeId?: string): Promise<void> {
  return invokeCommand<void>("assign_chapter_volume", { projectRoot, chapterId, volumeId });
}
