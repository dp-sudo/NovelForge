import { withDatabase } from "./service-context.js";
import { getProjectId } from "./service-utils.js";

export interface CollectedContext {
  globalContext: Record<string, unknown>;
  relatedContext: Record<string, unknown>;
  retrievedContext: Record<string, unknown>;
  usedContext: string[];
}

export class ContextService {
  public async collectForChapter(
    projectRoot: string,
    chapterId: string,
    userInstruction: string
  ): Promise<CollectedContext> {
    return withDatabase(projectRoot, (db) => {
      const projectId = getProjectId(db);
      const project = db
        .prepare("SELECT name, genre FROM projects WHERE id = ?")
        .get(projectId) as { name: string; genre: string };

      const chapter = db
        .prepare("SELECT id, title, summary, status FROM chapters WHERE id = ?")
        .get(chapterId) as { id: string; title: string; summary: string; status: string };

      const glossary = db
        .prepare("SELECT term, aliases, locked, banned FROM glossary_terms WHERE project_id = ?")
        .all(projectId) as Array<{ term: string; aliases: string; locked: number; banned: number }>;

      const plotNodes = db
        .prepare(
          `
          SELECT p.title, p.node_type
          FROM plot_nodes p
          JOIN chapter_links cl ON cl.target_id = p.id
          WHERE cl.chapter_id = ? AND cl.target_type = 'plot_node'
          `
        )
        .all(chapterId);

      const characters = db
        .prepare(
          `
          SELECT c.name, c.role_type
          FROM characters c
          JOIN chapter_links cl ON cl.target_id = c.id
          WHERE cl.chapter_id = ? AND cl.target_type = 'character' AND c.is_deleted = 0
          `
        )
        .all(chapterId);

      return {
        globalContext: {
          projectName: project.name,
          genre: project.genre,
          lockedTerms: glossary.filter((item) => item.locked === 1).map((item) => item.term),
          bannedTerms: glossary.filter((item) => item.banned === 1).map((item) => item.term)
        },
        relatedContext: {
          chapter,
          plotNodes,
          characters
        },
        retrievedContext: {
          userInstructionKeywords: userInstruction
            .split(/[\s,，。！!？?]/)
            .map((item) => item.trim())
            .filter((item) => item.length > 1)
            .slice(0, 12)
        },
        usedContext: [
          "project.name",
          "project.genre",
          "glossary.locked+banned",
          "chapter.current",
          "chapter_links.plot_node",
          "chapter_links.character"
        ]
      };
    });
  }
}
