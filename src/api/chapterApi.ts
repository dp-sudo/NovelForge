import { invokeCommand, logUI } from "./tauriClient.js";
import type { ChapterInput } from "../domain/types.js";
import type { ChapterStatus } from "../domain/constants.js";

// ── Import (no dev-engine fallback needed) ──

export interface ImportFileEntry {
  file_name: string;
  content: string;
}

export interface ImportedChapter {
  id: string;
  title: string;
  chapterIndex: number;
}

export interface ImportResult {
  importedCount: number;
  chapters: ImportedChapter[];
}

export async function importChapterFiles(projectRoot: string, files: ImportFileEntry[]): Promise<ImportResult> {
  return invokeCommand<ImportResult>("import_chapter_files", { input: { projectRoot, files } });
}

// ── Backup ──

export interface BackupResult {
  filePath: string;
  fileSize: number;
  createdAt: string;
}

export interface RestoreResult {
  projectRoot: string;
  filesRestored: number;
}

export async function createBackup(projectRoot: string): Promise<BackupResult> {
  return invokeCommand<BackupResult>("create_backup", { projectRoot });
}

export async function listBackups(projectRoot: string): Promise<BackupResult[]> {
  return invokeCommand<BackupResult[]>("list_backups", { projectRoot });
}

export async function restoreBackup(projectRoot: string, backupPath: string): Promise<RestoreResult> {
  return invokeCommand<RestoreResult>("restore_backup", { projectRoot, backupPath });
}

// ── Search (no dev-engine fallback needed) ──

export interface SearchResult {
  entityType: string;
  entityId: string;
  title: string;
  bodySnippet: string;
  rank: number;
}

export async function searchProject(projectRoot: string, query: string, limit?: number): Promise<SearchResult[]> {
  return invokeCommand<SearchResult[]>("search_project", { projectRoot, query, limit });
}

export async function searchProjectSemantic(projectRoot: string, query: string, limit?: number): Promise<SearchResult[]> {
  return invokeCommand<SearchResult[]>("search_project_semantic", { projectRoot, query, limit });
}

export async function rebuildSearchIndex(projectRoot: string): Promise<number> {
  return invokeCommand<number>("rebuild_search_index", { projectRoot });
}

export async function rebuildVectorIndex(projectRoot: string): Promise<number> {
  return invokeCommand<number>("rebuild_vector_index", { projectRoot });
}

// ── Integrity (no dev-engine fallback needed) ──

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

export async function reorderChapters(projectRoot: string, chapterIds: string[]): Promise<void> {
  return invokeCommand<void>("reorder_chapters", { projectRoot, orderedIds: chapterIds });
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
  // 问题1修复(调用面): 编辑器切章时必须先走正式正文读取，再做草稿恢复决策。
  return invokeCommand<string>("read_chapter_content", {
    projectRoot,
    chapterId,
  });
}

export async function deleteChapter(id: string, projectRoot: string): Promise<void> {
  await invokeCommand<void>("delete_chapter", { projectRoot, input: { id } });
}

// ── Snapshots ──

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

// ── Volumes ──

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
