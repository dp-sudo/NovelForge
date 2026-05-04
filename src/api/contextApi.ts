import { invokeCommand } from "./tauriClient.js";

export interface ReviewWorkItem {
  id: string;
  runId: string;
  taskType: string;
  title: string;
  severity: string;
  message: string;
  status: string;
  createdAt: string;
}

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
  reviewQueue: ReviewWorkItem[];
  latestCheckpoint: {
    checkpointId: string;
    runId: string;
    taskType: string;
    status: string;
    createdAt: string;
    reviewPendingCount: number;
    reviewTotalCount: number;
  } | null;
  polishSummary: {
    pending: number;
    resolved: number;
    rejected: number;
    total: number;
  };
  previousChapterSummary: string | null;
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

export async function updateReviewQueueItemStatus(
  projectRoot: string,
  itemId: string,
  status: "pending" | "resolved" | "rejected"
): Promise<void> {
  return invokeCommand<void>("update_review_queue_item_status", {
    projectRoot,
    itemId,
    status,
  });
}

export async function listReviewWorkItems(
  projectRoot: string,
  options: {
    chapterId?: string;
    taskType?: string;
    status?: "pending" | "resolved" | "rejected";
    limit?: number;
  } = {}
): Promise<ReviewWorkItem[]> {
  return invokeCommand<ReviewWorkItem[]>("list_review_work_items", {
    projectRoot,
    chapterId: options.chapterId,
    taskType: options.taskType,
    status: options.status,
    limit: options.limit ?? 100,
  });
}
