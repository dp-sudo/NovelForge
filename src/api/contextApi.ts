import { invokeCommand } from "./tauriClient.js";

export interface ChapterContext {
  chapter: {
    id: string;
    title: string;
    summary: string;
    status: string;
    targetWords: number;
    currentWords: number;
  };
  characters: Array<{
    id: string;
    name: string;
    roleType: string;
    identityText: string | null;
    motivation: string | null;
    desire: string | null;
    flaw: string | null;
  }>;
  worldRules: Array<{
    id: string;
    title: string;
    category: string;
    description: string;
    constraintLevel: string;
  }>;
  plotNodes: Array<{
    id: string;
    title: string;
    nodeType: string;
    goal: string | null;
    sortOrder: number;
  }>;
  glossary: Array<{
    term: string;
    termType: string;
    locked: boolean;
    banned: boolean;
  }>;
  blueprint: Array<{
    stepKey: string;
    content: string;
  }>;
  assetCandidates: Array<{
    label: string;
    assetType: string;
    occurrences: number;
    confidence: number;
    evidence: string;
  }>;
  previousChapterSummary: string | null;
}

export async function getChapterContext(projectRoot: string, chapterId: string): Promise<ChapterContext> {
  return invokeCommand<ChapterContext>("get_chapter_context", { projectRoot, chapterId });
}
