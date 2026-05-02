import {
  type RunTaskPipelineInput,
} from "./pipelineApi.js";

export type BookStageKey =
  | "blueprint-anchor"
  | "blueprint-genre"
  | "blueprint-premise"
  | "blueprint-characters"
  | "blueprint-world"
  | "blueprint-glossary"
  | "blueprint-plot"
  | "blueprint-chapters"
  | "character-seed"
  | "world-seed"
  | "plot-seed"
  | "glossary-seed"
  | "narrative-seed"
  | "chapter-plan";

export type BookStage = {
  key: BookStageKey;
  label: string;
  request: RunTaskPipelineInput;
};

export interface RunBookGenerationInput {
  projectRoot: string;
  ideaPrompt: string;
  chapterId?: string;
}

export interface ChapterPlanChapterCandidate {
  id: string;
  chapterIndex: number;
  status: string;
  targetWords: number;
  currentWords: number;
}

export type ChapterPlanSelectionStrategy =
  | "user_specified"
  | "next_porous"
  | "window_drift"
  | "arc_anchor"
  | "none";

export interface ResolveChapterPlanChapterInput {
  chapters: ChapterPlanChapterCandidate[];
  explicitChapterId?: string;
  activeChapterId?: string;
  windowPlanningHorizon?: number;
  driftThreshold?: number;
}

export interface ResolveChapterPlanChapterResult {
  chapterId: string | null;
  strategy: ChapterPlanSelectionStrategy;
}

function normalizeChapterId(chapterId?: string): string | undefined {
  const trimmed = chapterId?.trim();
  return trimmed ? trimmed : undefined;
}

function normalizeWindowPlanningHorizon(raw?: number): number {
  const value = Number.isFinite(raw) ? Math.trunc(raw as number) : 10;
  return Math.max(1, Math.min(50, value || 10));
}

function isPlannableChapterStatus(status: string): boolean {
  return status !== "completed" && status !== "archived";
}

function computeChapterDrift(chapter: ChapterPlanChapterCandidate): number {
  if (chapter.targetWords <= 0) {
    return chapter.currentWords > 0 ? 1 : 0;
  }
  return Math.abs(chapter.currentWords - chapter.targetWords) / chapter.targetWords;
}

function sortChapterCandidates(chapters: ChapterPlanChapterCandidate[]): ChapterPlanChapterCandidate[] {
  return [...chapters].sort((a, b) => a.chapterIndex - b.chapterIndex);
}

function resolveWindowChapters(
  chapters: ChapterPlanChapterCandidate[],
  activeChapterId?: string,
  horizon = 10,
): ChapterPlanChapterCandidate[] {
  if (chapters.length === 0) {
    return [];
  }
  const active = activeChapterId
    ? chapters.find((chapter) => chapter.id === activeChapterId)
    : undefined;
  if (!active) {
    return chapters.slice(0, horizon);
  }
  const minIndex = active.chapterIndex + 1;
  const maxIndex = active.chapterIndex + horizon;
  return chapters.filter((chapter) => chapter.chapterIndex >= minIndex && chapter.chapterIndex <= maxIndex);
}

function resolveArcAnchorCandidate(
  candidates: ChapterPlanChapterCandidate[],
): ChapterPlanChapterCandidate | null {
  if (candidates.length === 0) {
    return null;
  }
  const preferred = candidates
    .filter((chapter) => chapter.status === "revising" || chapter.status === "drafting")
    .sort((a, b) => b.chapterIndex - a.chapterIndex)[0];
  return preferred ?? null;
}

export function resolveChapterPlanChapterSelection(
  input: ResolveChapterPlanChapterInput,
): ResolveChapterPlanChapterResult {
  const chapters = sortChapterCandidates(input.chapters);
  if (chapters.length === 0) {
    return { chapterId: null, strategy: "none" };
  }

  const explicit = normalizeChapterId(input.explicitChapterId);
  if (explicit && chapters.some((chapter) => chapter.id === explicit)) {
    return { chapterId: explicit, strategy: "user_specified" };
  }

  const activeChapterId = normalizeChapterId(input.activeChapterId);
  const horizon = normalizeWindowPlanningHorizon(input.windowPlanningHorizon);
  const windowChapters = resolveWindowChapters(chapters, activeChapterId, horizon);
  const nextPorous = windowChapters.find((chapter) => isPlannableChapterStatus(chapter.status));
  if (nextPorous) {
    return { chapterId: nextPorous.id, strategy: "next_porous" };
  }

  const driftThreshold = input.driftThreshold ?? 0.35;
  const driftRanked = (windowChapters.length > 0 ? windowChapters : chapters)
    .map((chapter) => ({ chapter, drift: computeChapterDrift(chapter) }))
    .filter((item) => item.drift >= driftThreshold)
    .sort((a, b) => b.drift - a.drift);

  if (driftRanked.length > 0) {
    const anchor = resolveArcAnchorCandidate(driftRanked.map((item) => item.chapter));
    if (anchor) {
      return { chapterId: anchor.id, strategy: "arc_anchor" };
    }
    return { chapterId: driftRanked[0]?.chapter.id ?? null, strategy: "window_drift" };
  }

  const fallback = chapters.find((chapter) => isPlannableChapterStatus(chapter.status)) ?? chapters[0];
  return { chapterId: fallback?.id ?? null, strategy: "none" };
}

export function selectPromotionStage(
  stages: BookStage[],
  preferredStageKey?: BookStageKey | string,
): BookStage | null {
  if (stages.length === 0) {
    return null;
  }
  const normalizedPreferred = typeof preferredStageKey === "string" ? preferredStageKey.trim() : "";
  if (normalizedPreferred) {
    const matched = stages.find((stage) => stage.key === normalizedPreferred);
    if (matched) {
      return matched;
    }
  }
  return stages[0] ?? null;
}

function buildCharacterSeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 个核心角色，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "name": "",
  "roleType": "主角",
  "age": "",
  "gender": "",
  "identityText": "",
  "appearance": "",
  "motivation": "",
  "desire": "",
  "fear": "",
  "flaw": "",
  "arcStage": "",
  "notes": ""
}`,
  ].join("\n");
}

function buildWorldSeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 条世界规则，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "title": "",
  "category": "世界规则",
  "description": "",
  "constraintLevel": "normal",
  "examples": "",
  "contradictionPolicy": ""
}`,
  ].join("\n");
}

function buildPlotSeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 个主线剧情节点，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "title": "",
  "nodeType": "开端",
  "goal": "",
  "conflict": "",
  "emotionalCurve": "",
  "status": "规划中",
  "relatedCharacters": []
}`,
  ].join("\n");
}

function buildGlossarySeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 条核心名词，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "term": "",
  "termType": "术语",
  "aliases": [],
  "description": "",
  "locked": false,
  "banned": false
}`,
  ].join("\n");
}

function buildNarrativeSeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 条叙事义务，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "obligationType": "明线伏笔",
  "description": "",
  "plantedChapterId": "",
  "expectedPayoffChapterId": "",
  "actualPayoffChapterId": "",
  "payoffStatus": "open",
  "severity": "medium",
  "relatedEntities": []
}`,
  ].join("\n");
}

export function buildPromotionStages(input: RunBookGenerationInput): BookStage[] {
  const base = input.ideaPrompt.trim();
  const chapterId = normalizeChapterId(input.chapterId);
  const stages: BookStage[] = [
    {
      key: "character-seed",
      label: "角色: 核心角色草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "character.create",
        userInstruction: buildCharacterSeedInstruction(base),
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
    {
      key: "world-seed",
      label: "设定: 世界规则草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "world.create_rule",
        userInstruction: buildWorldSeedInstruction(base),
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
    {
      key: "plot-seed",
      label: "剧情: 主线节点草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "plot.create_node",
        userInstruction: buildPlotSeedInstruction(base),
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
    {
      key: "glossary-seed",
      label: "名词: 核心术语草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "glossary.create_term",
        userInstruction: buildGlossarySeedInstruction(base),
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
    {
      key: "narrative-seed",
      label: "叙事: 义务草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "narrative.create_obligation",
        userInstruction: buildNarrativeSeedInstruction(base),
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
  ];

  if (chapterId) {
    stages.push({
      key: "chapter-plan",
      label: "章节: 章节计划",
      request: {
        projectRoot: input.projectRoot,
        taskType: "chapter.plan",
        chapterId,
        userInstruction: base,
        persistMode: "formal",
        automationTier: "confirm",
      },
    });
  }
  return stages;
}
