import { invokeCommand } from "./tauriClient.js";
import { runModuleAiTask } from "./moduleAiApi.js";
import type { WorldRuleInput } from "../domain/types.js";

export interface WorldRow {
  id: string;
  project_id: string;
  title: string;
  category: string;
  description: string;
  constraint_level: string;
  related_entities: string[];
  examples: string | null;
  contradiction_policy: string | null;
  is_deleted: number;
  created_at: string;
  updated_at: string;
}

export async function listWorldRules(projectRoot: string): Promise<WorldRow[]> {
  return invokeCommand<WorldRow[]>("list_world_rules", { projectRoot });
}

export async function createWorldRule(input: WorldRuleInput, projectRoot: string): Promise<string> {
  return invokeCommand<string>("create_world_rule", { projectRoot, input });
}

export async function deleteWorldRule(id: string, projectRoot: string): Promise<void> {
  await invokeCommand<void>("delete_world_rule", { projectRoot, id });
}

export async function aiGenerateWorldRule(projectRoot: string, userDescription: string): Promise<string> {
  return runModuleAiTask({
    projectRoot,
    taskType: "world.create_rule",
    userInstruction: userDescription,
    autoPersist: true,
    persistMode: "formal",
    automationTier: "supervised",
    uiAction: "ai_generate_world_rule",
  });
}
