import { invokeCommand } from "./tauriClient.js";

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
