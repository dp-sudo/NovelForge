import { invokeCommand } from "./tauriClient.js";
import type { BlueprintStepRow } from "./blueprintApi.js";
import type { ChapterContext } from "./contextApi.js";
import type { FeedbackEvent } from "./statsApi.js";
import type { NarrativeObligation } from "./narrativeApi.js";
import type { ConsistencyIssueRow } from "./consistencyApi.js";
import type { ChapterRecord, VolumeRecord } from "./chapterApi.js";

export interface CommandCenterStats {
  totalWords: number;
  chapterCount: number;
  characterCount: number;
  worldRuleCount: number;
  plotNodeCount: number;
  openIssueCount: number;
  completedChapterCount: number;
  completedBlueprintCount: number;
  totalBlueprintSteps: number;
  blueprintProgress: number;
}

export interface CommandCenterGlossaryTerm {
  id: string;
  term: string;
  termType: string;
  locked: boolean;
  banned: boolean;
}

export interface CommandCenterWorldRule {
  id: string;
  title: string;
  category: string;
  constraintLevel?: string;
  constraint_level?: string;
}

export interface CommandCenterPlotNode {
  id: string;
  title: string;
  nodeType?: string;
  node_type?: string;
  status?: string;
}

export interface CommandCenterCharacter {
  id: string;
  name: string;
  roleType?: string;
  role_type?: string;
}

export interface CommandCenterConstitution {
  blueprintSteps: BlueprintStepRow[];
  obligations: NarrativeObligation[];
  lockedTerms: CommandCenterGlossaryTerm[];
  bannedTerms: CommandCenterGlossaryTerm[];
  strongRules: CommandCenterWorldRule[];
}

export interface CommandCenterWindowPlanning {
  volumeStructure: string;
  chapterGoals: string[];
  currentVolumeProgress: number;
  plannedChapterCount: number;
  windowPlanningHorizon: number;
}

export interface CommandCenterProductionQueue {
  activeChapterId?: string | null;
  chapters: ChapterRecord[];
  volumes: VolumeRecord[];
  windowPlanning: CommandCenterWindowPlanning;
  nextActions: string[];
}

export interface CommandCenterReviewQueue {
  feedbackEvents: FeedbackEvent[];
  consistencyIssues: ConsistencyIssueRow[];
  driftWarnings: string[];
  openFeedbackCount: number;
  acknowledgedFeedbackCount: number;
  openIssueCount: number;
  highSeverityIssueCount: number;
  stateUpdateCount: number;
  assetPromotionCount: number;
}

export interface CommandCenterAssetAuthority {
  characterCount: number;
  worldRuleCount: number;
  glossaryCount: number;
  plotNodeCount: number;
  previewCharacters: CommandCenterCharacter[];
  previewWorldRules: CommandCenterWorldRule[];
  previewGlossary: CommandCenterGlossaryTerm[];
  previewPlotNodes: CommandCenterPlotNode[];
}

export interface CommandCenterWorkspace {
  chapterId: string;
  chapterIndex: number;
  chapterTitle: string;
  chapterStatus: string;
  targetWords: number;
  currentWords: number;
  context: ChapterContext;
}

export interface CommandCenterSnapshot {
  stats: CommandCenterStats;
  constitution: CommandCenterConstitution;
  productionQueue: CommandCenterProductionQueue;
  reviewQueue: CommandCenterReviewQueue;
  assetAuthority: CommandCenterAssetAuthority;
  workspace?: CommandCenterWorkspace | null;
}

export async function getCommandCenterSnapshot(
  projectRoot: string,
  chapterId?: string | null,
): Promise<CommandCenterSnapshot> {
  return invokeCommand<CommandCenterSnapshot>("get_command_center_snapshot", {
    projectRoot,
    chapterId: chapterId || null,
  });
}
