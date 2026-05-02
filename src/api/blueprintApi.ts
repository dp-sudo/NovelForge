import { invokeCommand } from "./tauriClient.js";
import { runModuleAiTask } from "./moduleAiApi.js";
import type { BlueprintStepKey, BlueprintStepStatus } from "../domain/constants.js";
import { parseBlueprintContent, type BlueprintCertaintyZones } from "../domain/types.js";
import { listChapters } from "./chapterApi.js";
import { getProjectAiStrategy } from "./settingsApi.js";

export interface BlueprintStepRow {
  id: string;
  projectId: string;
  stepKey: BlueprintStepKey;
  title: string;
  content: string;
  contentPath: string;
  status: BlueprintStepStatus;
  aiGenerated: boolean;
  certaintyZones?: BlueprintCertaintyZones;
  completedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export async function listBlueprintSteps(projectRoot: string): Promise<BlueprintStepRow[]> {
  return invokeCommand<BlueprintStepRow[]>("list_blueprint_steps", { projectRoot });
}

export async function saveBlueprintStep(
  stepKey: BlueprintStepKey,
  content: string,
  aiGenerated = false,
  projectRoot: string,
  certaintyZones?: BlueprintCertaintyZones
): Promise<void> {
  await invokeCommand<void>("save_blueprint_step", {
    projectRoot,
    input: { stepKey, content, aiGenerated, certaintyZones },
  });
}

export async function markBlueprintCompleted(stepKey: BlueprintStepKey, projectRoot: string): Promise<void> {
  await invokeCommand<void>("mark_blueprint_completed", { projectRoot, stepKey });
}

export async function resetBlueprintStep(stepKey: BlueprintStepKey, projectRoot: string): Promise<void> {
  await invokeCommand<void>("reset_blueprint_step", { projectRoot, stepKey });
}

export interface BlueprintSuggestionInput {
  projectRoot: string;
  stepKey: string;
  stepTitle: string;
  userInstruction: string;
}

export interface WindowPlanningData {
  volumeStructure: string;
  chapterGoals: string[];
  currentVolumeProgress: number;
  plannedChapterCount: number;
  windowPlanningHorizon: number;
}

function splitTextList(raw: string): string[] {
  return raw
    .split(/\r?\n|[;；]/)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function clampHorizon(raw: number): number {
  const normalized = Number.isFinite(raw) ? Math.trunc(raw) : 10;
  return Math.max(1, Math.min(50, normalized || 10));
}

export async function getWindowPlanningData(projectRoot: string): Promise<WindowPlanningData> {
  const [steps, chapters, strategy] = await Promise.all([
    listBlueprintSteps(projectRoot),
    listChapters(projectRoot),
    getProjectAiStrategy(projectRoot),
  ]);

  const chapterStep = steps.find((step) => step.stepKey === "step-08-chapters");
  const chapterFields = parseBlueprintContent("step-08-chapters", chapterStep?.content ?? "");
  const chapterGoals = splitTextList(chapterFields.chapterGoals ?? "");
  const chapterList = splitTextList(chapterFields.chapterList ?? "");
  const plannedChapterCount = chapterList.length > 0 ? chapterList.length : chapterGoals.length;
  const completedChapterCount = chapters.filter((chapter) => chapter.status === "completed").length;
  const progressBase = plannedChapterCount > 0 ? plannedChapterCount : chapters.length;
  const currentVolumeProgress = progressBase > 0
    ? Math.min(100, Math.round((completedChapterCount / progressBase) * 100))
    : 0;

  return {
    volumeStructure: (chapterFields.volumeStructure ?? "").trim(),
    chapterGoals: chapterGoals.slice(0, clampHorizon(strategy.windowPlanningHorizon)),
    currentVolumeProgress,
    plannedChapterCount,
    windowPlanningHorizon: clampHorizon(strategy.windowPlanningHorizon),
  };
}

export async function generateBlueprintSuggestion(input: BlueprintSuggestionInput): Promise<string> {
  return runModuleAiTask({
    projectRoot: input.projectRoot,
    taskType: "blueprint.generate_step",
    userInstruction: input.userInstruction,
    blueprintStepKey: input.stepKey,
    blueprintStepTitle: input.stepTitle,
    persistMode: "formal",
    automationTier: "supervised",
    uiAction: "generate_blueprint_suggestion",
  });
}
