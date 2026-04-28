import { invokeCommand } from "./tauriClient.js";
import type { BlueprintStepKey, BlueprintStepStatus } from "../domain/constants.js";

export interface BlueprintStepRow {
  id: string;
  projectId: string;
  stepKey: BlueprintStepKey;
  title: string;
  content: string;
  contentPath: string;
  status: BlueprintStepStatus;
  aiGenerated: boolean;
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
  projectRoot: string
): Promise<void> {
  await invokeCommand<void>("save_blueprint_step", { projectRoot, input: { stepKey, content, aiGenerated } });
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

export async function generateBlueprintSuggestion(input: BlueprintSuggestionInput): Promise<string> {
  return invokeCommand<string>("generate_blueprint_suggestion", { input });
}
