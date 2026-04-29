import fs from "node:fs/promises";
import path from "node:path";

import { AppError } from "../../../src/errors/app-error.js";
import type { ExportOptions } from "../../../src/domain/types.js";
import { withDatabase } from "./service-context.js";
import { getProjectId } from "./service-utils.js";

interface ChapterExportRow {
  id: string;
  chapter_index: number;
  title: string;
  summary: string | null;
  content_path: string;
}

type ExportFormat = "txt" | "md" | "docx" | "pdf" | "epub";

function stripFrontmatter(content: string): string {
  if (!content.startsWith("---\n")) {
    return content;
  }
  const end = content.indexOf("\n---\n", 4);
  if (end === -1) {
    return content;
  }
  return content.slice(end + 5).trim();
}

function renderChapter(
  chapter: ChapterExportRow,
  body: string,
  options: ExportOptions,
  format: "txt" | "md"
): string {
  const lines: string[] = [];
  if (options.includeChapterTitle ?? true) {
    lines.push(format === "md" ? `# ${chapter.title}` : chapter.title);
  }
  if (options.includeChapterSummary && chapter.summary) {
    lines.push(format === "md" ? `> ${chapter.summary}` : `摘要：${chapter.summary}`);
  }
  lines.push(body.trim());
  return lines.join("\n\n");
}

export class ExportService {
  public async exportChapter(
    projectRoot: string,
    chapterId: string,
    format: ExportFormat,
    outputPath: string,
    options: ExportOptions = {}
  ): Promise<string> {
    const chapter = await withDatabase(projectRoot, (db) => {
      const row = db
        .prepare(
          `
          SELECT id, chapter_index, title, summary, content_path
          FROM chapters
          WHERE project_id = ? AND id = ? AND is_deleted = 0
          `
        )
        .get(getProjectId(db), chapterId) as ChapterExportRow | undefined;
      if (!row) {
        throw new AppError({
          code: "CHAPTER_NOT_FOUND",
          message: "章节不存在",
          recoverable: true
        });
      }
      return row;
    });

    const chapterFile = path.join(projectRoot, chapter.content_path);
    const content = await fs.readFile(chapterFile, "utf-8");
    const body = stripFrontmatter(content);
    const rendered = renderChapter(chapter, body, options, format === "md" ? "md" : "txt");
    await fs.mkdir(path.dirname(outputPath), { recursive: true });
    if (format === "txt" || format === "md") {
      await fs.writeFile(outputPath, rendered, "utf-8");
    } else {
      await fs.writeFile(outputPath, `${format.toUpperCase()} EXPORT\n\n${rendered}`, "utf-8");
    }
    return outputPath;
  }

  public async exportBook(
    projectRoot: string,
    format: ExportFormat,
    outputPath: string,
    options: ExportOptions = {}
  ): Promise<string> {
    const chapters = await withDatabase(projectRoot, (db) =>
      db
        .prepare(
          `
          SELECT id, chapter_index, title, summary, content_path
          FROM chapters
          WHERE project_id = ? AND is_deleted = 0
          ORDER BY chapter_index
          `
        )
        .all(getProjectId(db)) as unknown as ChapterExportRow[]
    );
    if (chapters.length === 0) {
      throw new AppError({
        code: "EXPORT_EMPTY_BOOK",
        message: "当前项目没有可导出章节",
        recoverable: true
      });
    }

    const chunks: string[] = [];
    for (const chapter of chapters) {
      const chapterFile = path.join(projectRoot, chapter.content_path);
      const content = await fs.readFile(chapterFile, "utf-8");
      const body = stripFrontmatter(content);
      chunks.push(renderChapter(chapter, body, options, format === "md" ? "md" : "txt"));
    }

    const divider = format === "md" ? "\n\n---\n\n" : "\n\n====================\n\n";
    const output = chunks.join(divider);
    await fs.mkdir(path.dirname(outputPath), { recursive: true });
    if (format === "txt" || format === "md") {
      await fs.writeFile(outputPath, output, "utf-8");
    } else {
      await fs.writeFile(outputPath, `${format.toUpperCase()} EXPORT\n\n${output}`, "utf-8");
    }
    return outputPath;
  }
}
