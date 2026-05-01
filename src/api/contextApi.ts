import { invokeCommand } from "./tauriClient.js";
import { listChapters, type ChapterRecord } from "./chapterApi.js";

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

export async function materializeChapterStructuredDrafts(
  projectRoot: string,
  chapterId: string,
): Promise<ChapterContext> {
  return invokeCommand<ChapterContext>("materialize_chapter_structured_drafts", {
    projectRoot,
    chapterId,
  });
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

export interface RejectStructuredDraftResult {
  draftItemId: string;
  draftItemStatus: string;
  batchStatus: string;
}

export interface ReviewTrailRecord {
  id: string;
  projectId: string;
  chapterId: string | null;
  entityType: string;
  entityId: string;
  draftItemId: string | null;
  action: string;
  reason: string | null;
  detail: Record<string, unknown> | null;
  createdAt: string;
}

export interface SummaryFeedbackData {
  keyVariableDelta: string[];
  driftWarnings: string[];
  assetPromotionCount: number;
  stateUpdateCount: number;
}

export function summarizeStateDeltaForFeedback(
  context: Pick<ChapterContext, "stateSummary">,
): string[] {
  return context.stateSummary
    .slice(0, 6)
    .map((item) => {
      if (item.subjectType === "window" && item.stateKind === "progress") {
        const chapterId = typeof item.payload.chapterId === "string" ? item.payload.chapterId : item.subjectId;
        const wordCount = typeof item.payload.wordCount === "number" ? item.payload.wordCount : null;
        if (wordCount !== null) {
          return `窗口进度更新：${chapterId}（${wordCount} 字）`;
        }
        return `窗口进度更新：${chapterId}`;
      }
      if (item.subjectType === "relationship" && item.stateKind === "relationship") {
        const sourceLabel = typeof item.payload.sourceLabel === "string" ? item.payload.sourceLabel : item.subjectId;
        const targetLabel = typeof item.payload.targetLabel === "string" ? item.payload.targetLabel : "未知对象";
        const relationshipType = typeof item.payload.relationshipType === "string"
          ? item.payload.relationshipType
          : "互动";
        return `关系状态更新：${sourceLabel} ↔ ${targetLabel}（${relationshipType}）`;
      }
      if (item.subjectType === "character" && item.stateKind === "involvement") {
        const characterLabel = typeof item.payload.characterLabel === "string"
          ? item.payload.characterLabel
          : item.subjectId;
        const involvementType = typeof item.payload.involvementType === "string"
          ? item.payload.involvementType
          : "一般戏份";
        return `角色戏份更新：${characterLabel}（${involvementType}）`;
      }
      if (item.subjectType === "scene" && item.stateKind === "scene") {
        const sceneLabel = typeof item.payload.sceneLabel === "string" ? item.payload.sceneLabel : item.subjectId;
        const sceneType = typeof item.payload.sceneType === "string" ? item.payload.sceneType : "场景";
        return `场景状态更新：${sceneLabel}（${sceneType}）`;
      }
      return `${item.subjectType}:${item.subjectId} -> ${item.stateKind}`;
    });
}

function collectDriftWarnings(chapters: ChapterRecord[], plannedChapterCount: number): string[] {
  const warnings: string[] = [];

  for (const chapter of chapters) {
    if (chapter.targetWords <= 0 || chapter.currentWords <= 0) continue;
    const delta = Math.abs(chapter.currentWords - chapter.targetWords) / chapter.targetWords;
    if (delta >= 0.35) {
      warnings.push(
        `第 ${chapter.chapterIndex} 章字数偏差 ${Math.round(delta * 100)}%（目标 ${chapter.targetWords}，当前 ${chapter.currentWords}）`,
      );
    }
  }

  if (plannedChapterCount > 0 && chapters.length > plannedChapterCount) {
    warnings.push(`实际章节数 ${chapters.length} 已超过蓝图计划 ${plannedChapterCount}`);
  } else if (plannedChapterCount > 0 && plannedChapterCount - chapters.length >= 3) {
    warnings.push(`当前章节数 ${chapters.length} 低于蓝图计划 ${plannedChapterCount}，窗口执行存在滞后`);
  }

  return warnings;
}

function selectSummaryChapter(chapters: ChapterRecord[]): ChapterRecord | null {
  if (chapters.length === 0) {
    return null;
  }
  const sorted = [...chapters].sort((a, b) => b.chapterIndex - a.chapterIndex);
  const active = sorted.find((chapter) => chapter.status === "drafting" || chapter.status === "revising");
  if (active) {
    return active;
  }
  const latestNonArchived = sorted.find((chapter) => chapter.status !== "archived");
  return latestNonArchived ?? sorted[0] ?? null;
}

export async function getSummaryFeedback(
  projectRoot: string,
  plannedChapterCount = 0,
): Promise<SummaryFeedbackData> {
  const chapters = await listChapters(projectRoot);
  if (chapters.length === 0) {
    return {
      keyVariableDelta: [],
      driftWarnings: [],
      assetPromotionCount: 0,
      stateUpdateCount: 0,
    };
  }

  const summaryChapter = selectSummaryChapter(chapters);
  if (!summaryChapter) {
    return {
      keyVariableDelta: [],
      driftWarnings: [],
      assetPromotionCount: 0,
      stateUpdateCount: 0,
    };
  }

  try {
    const context = await getChapterContext(projectRoot, summaryChapter.id);
    const assetPromotionCount =
      [...context.characters, ...context.worldRules, ...context.plotNodes, ...context.glossary]
        .filter((entity) => entity.sourceKind !== "user_input")
        .length;

    return {
      keyVariableDelta: summarizeStateDeltaForFeedback(context),
      driftWarnings: collectDriftWarnings(chapters, plannedChapterCount),
      assetPromotionCount,
      stateUpdateCount: context.stateSummary.length,
    };
  } catch {
    return {
      keyVariableDelta: [],
      driftWarnings: collectDriftWarnings(chapters, plannedChapterCount),
      assetPromotionCount: 0,
      stateUpdateCount: 0,
    };
  }
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
  input: ApplyStructuredDraftInput,
  reason?: string
): Promise<ApplyStructuredDraftResult> {
  return invokeCommand<ApplyStructuredDraftResult>("apply_structured_draft", {
    projectRoot,
    chapterId,
    input,
    reason: reason || null,
  });
}

export async function rejectStructuredDraft(
  projectRoot: string,
  chapterId: string,
  draftItemId: string,
  reason?: string
): Promise<RejectStructuredDraftResult> {
  return invokeCommand<RejectStructuredDraftResult>("reject_structured_draft", {
    projectRoot,
    chapterId,
    draftItemId,
    reason: reason || null,
  });
}

export async function getReviewTrail(
  projectRoot: string,
  entityType: string,
  entityId: string,
): Promise<ReviewTrailRecord[]> {
  return invokeCommand<ReviewTrailRecord[]>("get_review_trail", {
    projectRoot,
    entityType,
    entityId,
  });
}
