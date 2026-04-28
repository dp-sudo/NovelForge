import { invokeCommand } from "./tauriClient.js";
import * as Dev from "./dev-engine.js";
import type { ExportOptions } from "../domain/types.js";

export interface ExportOutput {
  outputPath: string;
  content?: string;
}

export type ExportFormat = "txt" | "md" | "docx" | "pdf" | "epub";

export async function exportChapter(
  projectRoot: string,
  chapterId: string,
  format: ExportFormat,
  outputPath: string,
  options?: ExportOptions
): Promise<ExportOutput> {
  try {
    return await invokeCommand<ExportOutput>("export_chapter", {
      input: { projectRoot, chapterId, format, outputPath, options }
    });
  } catch {
    return Dev.DevExport.exportChapter(projectRoot, chapterId, format, outputPath, options);
  }
}

export async function exportBook(
  projectRoot: string,
  format: ExportFormat,
  outputPath: string,
  options?: ExportOptions
): Promise<ExportOutput> {
  try {
    return await invokeCommand<ExportOutput>("export_book", {
      input: { projectRoot, format, outputPath, options }
    });
  } catch {
    return Dev.DevExport.exportBook(projectRoot, format, outputPath, options);
  }
}
