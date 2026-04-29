import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";

import { AppError } from "../../../src/errors/app-error.js";
import type { ChapterInput, ChapterRecord } from "../../../src/domain/types.js";
import { chapterPath, buildChapterMarkdown, readTextIfExists, writeFileAtomic } from "../infra/markdown.js";
import { chapterFileName, toPosixRelative } from "../infra/path-utils.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";
import { getProjectId } from "./service-utils.js";

function contentWordCount(content: string): number {
  const text = content.replace(/\s+/g, "");
  return text.length;
}

interface ChapterDbRow {
  id: string;
  chapter_index: number;
  title: string;
  summary: string | null;
  status: string;
  target_words: number;
  current_words: number;
  content_path: string;
  version: number;
  updated_at: string;
}

function mapRow(row: ChapterDbRow): ChapterRecord {
  return {
    id: row.id,
    chapterIndex: row.chapter_index,
    title: row.title,
    summary: row.summary ?? "",
    status: row.status as ChapterRecord["status"],
    targetWords: row.target_words,
    currentWords: row.current_words,
    contentPath: row.content_path,
    version: row.version,
    updatedAt: row.updated_at
  };
}

export class ChapterService {
  public async createChapter(projectRoot: string, input: ChapterInput): Promise<ChapterRecord> {
    return withDatabase(projectRoot, async (db) => {
      const projectId = getProjectId(db);
      const nextIndex =
        ((db.prepare("SELECT MAX(chapter_index) AS maxIndex FROM chapters WHERE project_id = ?").get(
          projectId
        ) as { maxIndex: number | null }).maxIndex ?? 0) + 1;

      const id = randomUUID();
      const createdAt = nowIso();
      const title = input.title.trim();
      if (title.length === 0) {
        throw new AppError({
          code: "CHAPTER_TITLE_REQUIRED",
          message: "章节标题不能为空",
          recoverable: true
        });
      }
      const fileName = chapterFileName(nextIndex);
      const absoluteChapterPath = path.join(projectRoot, "manuscript", "chapters", fileName);
      const relativeChapterPath = toPosixRelative(projectRoot, absoluteChapterPath);

      const markdown = buildChapterMarkdown({
        id,
        index: nextIndex,
        title,
        status: input.status ?? "drafting",
        summary: input.summary ?? "",
        wordCount: 0,
        createdAt,
        updatedAt: createdAt,
        content: ""
      });

      db.exec("BEGIN");
      try {
        await fs.writeFile(absoluteChapterPath, markdown, "utf-8");
        db.prepare(
          `
          INSERT INTO chapters(
            id, project_id, chapter_index, title, summary, status, target_words, current_words, content_path, version, created_at, updated_at
          )
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          `
        ).run(
          id,
          projectId,
          nextIndex,
          title,
          input.summary ?? "",
          input.status ?? "drafting",
          input.targetWords ?? 0,
          0,
          relativeChapterPath,
          1,
          createdAt,
          createdAt
        );
        db.exec("COMMIT");
      } catch (error) {
        db.exec("ROLLBACK");
        await fs.rm(absoluteChapterPath, { force: true });
        throw error;
      }

      return {
        id,
        chapterIndex: nextIndex,
        title,
        summary: input.summary ?? "",
        status: input.status ?? "drafting",
        targetWords: input.targetWords ?? 0,
        currentWords: 0,
        contentPath: relativeChapterPath,
        version: 1,
        updatedAt: createdAt
      };
    });
  }

  public async listChapters(projectRoot: string): Promise<ChapterRecord[]> {
    return withDatabase(projectRoot, (db) => {
      const rows = db
        .prepare(
          `
          SELECT id, chapter_index, title, summary, status, target_words, current_words, content_path, version, updated_at
          FROM chapters
          WHERE project_id = ? AND is_deleted = 0
          ORDER BY chapter_index
          `
        )
        .all(getProjectId(db)) as unknown as ChapterDbRow[];
      return rows.map(mapRow);
    });
  }

  public async getChapterContent(projectRoot: string, chapterId: string): Promise<string> {
    return withDatabase(projectRoot, async (db) => {
      const row = db
        .prepare("SELECT content_path FROM chapters WHERE id = ? AND is_deleted = 0")
        .get(chapterId) as { content_path: string } | undefined;
      if (!row) {
        throw new AppError({
          code: "CHAPTER_NOT_FOUND",
          message: "章节不存在",
          recoverable: true
        });
      }
      const absolutePath = chapterPath(projectRoot, row.content_path);
      const content = await fs.readFile(absolutePath, "utf-8");
      return content;
    });
  }

  public async saveChapterContent(projectRoot: string, chapterId: string, content: string): Promise<void> {
    await withDatabase(projectRoot, async (db) => {
      const row = db
        .prepare(
          `
          SELECT id, chapter_index, title, summary, status, target_words, current_words, content_path, version, updated_at
          FROM chapters
          WHERE id = ? AND is_deleted = 0
          `
        )
        .get(chapterId) as ChapterDbRow | undefined;
      if (!row) {
        throw new AppError({
          code: "CHAPTER_NOT_FOUND",
          message: "章节不存在",
          recoverable: true
        });
      }

      const absolutePath = chapterPath(projectRoot, row.content_path);
      const updatedAt = nowIso();
      const count = contentWordCount(content);

      const markdown = buildChapterMarkdown({
        id: row.id,
        index: row.chapter_index,
        title: row.title,
        status: row.status,
        summary: row.summary ?? "",
        wordCount: count,
        createdAt: row.updated_at,
        updatedAt,
        content
      });

      await writeFileAtomic(absolutePath, markdown);

      db.prepare(
        `
        UPDATE chapters
        SET current_words = ?, version = version + 1, updated_at = ?
        WHERE id = ?
        `
      ).run(count, updatedAt, chapterId);

      const draftsDir = path.join(projectRoot, "manuscript", "drafts");
      const draftPath = path.join(draftsDir, `${path.basename(row.content_path)}.autosave.md`);
      await fs.rm(draftPath, { force: true });
    });
  }

  public async autosaveDraft(projectRoot: string, chapterId: string, content: string): Promise<string> {
    return withDatabase(projectRoot, async (db) => {
      const row = db
        .prepare("SELECT content_path FROM chapters WHERE id = ? AND is_deleted = 0")
        .get(chapterId) as { content_path: string } | undefined;
      if (!row) {
        throw new AppError({
          code: "CHAPTER_NOT_FOUND",
          message: "章节不存在",
          recoverable: true
        });
      }
      const draftPath = path.join(
        projectRoot,
        "manuscript",
        "drafts",
        `${path.basename(row.content_path)}.autosave.md`
      );
      await fs.writeFile(draftPath, content, "utf-8");
      return draftPath;
    });
  }

  public async recoverDraft(projectRoot: string, chapterId: string): Promise<{
    hasNewerDraft: boolean;
    draftContent?: string;
  }> {
    return withDatabase(projectRoot, async (db) => {
      const row = db
        .prepare("SELECT content_path FROM chapters WHERE id = ? AND is_deleted = 0")
        .get(chapterId) as { content_path: string } | undefined;
      if (!row) {
        throw new AppError({
          code: "CHAPTER_NOT_FOUND",
          message: "章节不存在",
          recoverable: true
        });
      }

      const chapterFile = chapterPath(projectRoot, row.content_path);
      const draftPath = path.join(
        projectRoot,
        "manuscript",
        "drafts",
        `${path.basename(row.content_path)}.autosave.md`
      );
      const draftContent = await readTextIfExists(draftPath);
      if (draftContent === undefined) {
        return { hasNewerDraft: false };
      }

      const [chapterStat, draftStat] = await Promise.all([fs.stat(chapterFile), fs.stat(draftPath)]);
      if (draftStat.mtimeMs <= chapterStat.mtimeMs) {
        return { hasNewerDraft: false };
      }

      return { hasNewerDraft: true, draftContent };
    });
  }

  public async reorderChapters(projectRoot: string, orderedChapterIds: string[]): Promise<void> {
    await withDatabase(projectRoot, (db) => {
      db.exec("BEGIN");
      try {
        for (let i = 0; i < orderedChapterIds.length; i += 1) {
          db.prepare("UPDATE chapters SET chapter_index = ?, updated_at = ? WHERE id = ?").run(
            -(i + 1),
            nowIso(),
            orderedChapterIds[i]
          );
        }
        for (let i = 0; i < orderedChapterIds.length; i += 1) {
          db.prepare("UPDATE chapters SET chapter_index = ?, updated_at = ? WHERE id = ?").run(
            i + 1,
            nowIso(),
            orderedChapterIds[i]
          );
        }
        db.exec("COMMIT");
      } catch (error) {
        db.exec("ROLLBACK");
        throw error;
      }
    });
  }

  public async softDeleteChapter(projectRoot: string, chapterId: string): Promise<void> {
    await withDatabase(projectRoot, (db) => {
      db.prepare("UPDATE chapters SET is_deleted = 1, updated_at = ? WHERE id = ?").run(nowIso(), chapterId);
    });
  }
}
