import { invokeCommand } from "./tauriClient.js";
import { runModuleAiTask } from "./moduleAiApi.js";
import type { GlossaryTermInput } from "../domain/types.js";

export interface GlossaryRow {
  id: string;
  project_id: string;
  term: string;
  term_type: string;
  aliases: string[];
  description: string | null;
  locked: boolean;
  banned: boolean;
  preferred_usage: string | null;
  created_at: string;
  updated_at: string;
}

export async function listGlossaryTerms(projectRoot: string): Promise<GlossaryRow[]> {
  return invokeCommand<GlossaryRow[]>("list_glossary_terms", { projectRoot });
}

export async function createGlossaryTerm(input: GlossaryTermInput, projectRoot: string): Promise<string> {
  return invokeCommand<string>("create_glossary_term", { projectRoot, input });
}

export async function aiGenerateGlossaryTerm(projectRoot: string, userDescription: string): Promise<string> {
  return runModuleAiTask({
    projectRoot,
    taskType: "glossary.create_term",
    userInstruction: userDescription,
    persistMode: "formal",
    automationTier: "supervised",
    uiAction: "ai_generate_glossary_term",
  });
}
