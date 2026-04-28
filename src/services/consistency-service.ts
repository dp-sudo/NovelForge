import { randomUUID } from "node:crypto";

import type { ConsistencyIssue } from "../domain/types.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";
import { getProjectId } from "./service-utils.js";
import { ChapterService } from "./chapter-service.js";

const AI_STYLE_PATTERNS = [
  "命运的齿轮",
  "这一刻，他明白了",
  "不禁让人",
  "仿佛一切都"
];

export class ConsistencyService {
  private readonly chapterService: ChapterService;

  public constructor(input?: { chapterService?: ChapterService }) {
    this.chapterService = input?.chapterService ?? new ChapterService();
  }

  public async scanChapter(projectRoot: string, chapterId: string): Promise<ConsistencyIssue[]> {
    const chapterMarkdown = await this.chapterService.getChapterContent(projectRoot, chapterId);

    return withDatabase(projectRoot, (db) => {
      const projectId = getProjectId(db);
      const glossaryRows = db
        .prepare("SELECT id, term, locked, banned FROM glossary_terms WHERE project_id = ?")
        .all(projectId) as Array<{ id: string; term: string; locked: number; banned: number }>;

      const issues: ConsistencyIssue[] = [];
      const now = nowIso();

      for (const term of glossaryRows) {
        if (term.banned === 1 && chapterMarkdown.includes(term.term)) {
          issues.push({
            id: randomUUID(),
            issueType: "glossary",
            severity: "high",
            chapterId,
            sourceText: term.term,
            relatedAssetType: "glossary_term",
            relatedAssetId: term.id,
            explanation: `检测到禁用词：${term.term}`,
            suggestedFix: "替换为已锁定名词或删除该词",
            status: "open"
          });
        }
      }

      for (const phrase of AI_STYLE_PATTERNS) {
        if (chapterMarkdown.includes(phrase)) {
          issues.push({
            id: randomUUID(),
            issueType: "prose_style",
            severity: "medium",
            chapterId,
            sourceText: phrase,
            explanation: `检测到可能的 AI 套话：${phrase}`,
            suggestedFix: "改成具体动作、对话或场景细节",
            status: "open"
          });
        }
      }

      const insertStatement = db.prepare(
        `
        INSERT INTO consistency_issues(
          id, project_id, issue_type, severity, chapter_id, source_text, source_start, source_end,
          related_asset_type, related_asset_id, explanation, suggested_fix, status, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `
      );
      for (const issue of issues) {
        insertStatement.run(
          issue.id,
          projectId,
          issue.issueType,
          issue.severity,
          chapterId,
          issue.sourceText,
          issue.sourceStart ?? null,
          issue.sourceEnd ?? null,
          issue.relatedAssetType ?? null,
          issue.relatedAssetId ?? null,
          issue.explanation,
          issue.suggestedFix ?? null,
          issue.status,
          now,
          now
        );
      }

      return issues;
    });
  }

  public async listIssues(projectRoot: string): Promise<Array<Record<string, unknown>>> {
    return withDatabase(projectRoot, (db) =>
      db
        .prepare(
          `
          SELECT *
          FROM consistency_issues
          WHERE project_id = ?
          ORDER BY created_at DESC
          `
        )
        .all(getProjectId(db)) as Array<Record<string, unknown>>
    );
  }

  public async updateIssueStatus(
    projectRoot: string,
    issueId: string,
    status: "open" | "ignored" | "fixed" | "false_positive"
  ): Promise<void> {
    await withDatabase(projectRoot, (db) => {
      db.prepare("UPDATE consistency_issues SET status = ?, updated_at = ? WHERE id = ?").run(
        status,
        nowIso(),
        issueId
      );
    });
  }
}
