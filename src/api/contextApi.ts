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
    sourceKind: string;
    sourceRef?: string | null;
    sourceRequestId?: string | null;
  }>;
  worldRules: Array<{
    id: string;
    title: string;
    category: string;
    description: string;
    constraintLevel: string;
    sourceKind: string;
    sourceRef?: string | null;
    sourceRequestId?: string | null;
  }>;
  plotNodes: Array<{
    id: string;
    title: string;
    nodeType: string;
    goal: string | null;
    sortOrder: number;
    sourceKind: string;
    sourceRef?: string | null;
    sourceRequestId?: string | null;
  }>;
  glossary: Array<{
    id: string;
    term: string;
    termType: string;
    locked: boolean;
    banned: boolean;
    sourceKind: string;
    sourceRef?: string | null;
    sourceRequestId?: string | null;
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
  relationshipDrafts: Array<{
    id: string;
    batchId: string;
    status: string;
    sourceLabel: string;
    targetLabel: string;
    relationshipType: string;
    confidence: number;
    evidence: string;
  }>;
  involvementDrafts: Array<{
    id: string;
    batchId: string;
    status: string;
    characterLabel: string;
    involvementType: string;
    occurrences: number;
    confidence: number;
    evidence: string;
  }>;
  sceneDrafts: Array<{
    id: string;
    batchId: string;
    status: string;
    sceneLabel: string;
    sceneType: string;
    confidence: number;
    evidence: string;
  }>;
  previousChapterSummary: string | null;
  stateSummary: Array<{
    subjectType: string;
    subjectId: string;
    stateKind: string;
    payload: Record<string, unknown>;
  }>;
}

export async function getChapterContext(projectRoot: string, chapterId: string): Promise<ChapterContext> {
  return invokeCommand<ChapterContext>("get_chapter_context", { projectRoot, chapterId });
}

export interface ApplyAssetCandidateInput {
  label: string;
  assetType: string;
  evidence?: string;
  targetKind?: "character" | "world_rule" | "plot_node" | "glossary_term";
}

export interface ApplyAssetCandidateResult {
  action: "created" | "reused";
  targetType: string;
  targetId: string;
  linkCreated: boolean;
  label: string;
}

export interface ApplyStructuredDraftInput {
  draftItemId?: string;
  draftKind: "relationship" | "involvement" | "scene";
  sourceLabel: string;
  targetLabel?: string;
  relationshipType?: string;
  involvementType?: string;
  sceneType?: string;
  evidence?: string;
}

export interface ApplyStructuredDraftResult {
  action: "created" | "reused";
  draftKind: string;
  draftItemId: string | null;
  draftItemStatus: string | null;
  primaryTargetId: string;
  secondaryTargetId: string | null;
}

export async function applyAssetCandidate(
  projectRoot: string,
  chapterId: string,
  input: ApplyAssetCandidateInput
): Promise<ApplyAssetCandidateResult> {
  return invokeCommand<ApplyAssetCandidateResult>("apply_asset_candidate", {
    projectRoot,
    chapterId,
    input,
  });
}

export async function applyStructuredDraft(
  projectRoot: string,
  chapterId: string,
  input: ApplyStructuredDraftInput
): Promise<ApplyStructuredDraftResult> {
  return invokeCommand<ApplyStructuredDraftResult>("apply_structured_draft", {
    projectRoot,
    chapterId,
    input,
  });
}
