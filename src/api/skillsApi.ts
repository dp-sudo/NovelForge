import { invokeCommand } from "./tauriClient.js";

// ── Skill Manifest (matches Rust skill_registry::SkillManifest) ──

export interface SkillManifest {
  id: string;
  name: string;
  description: string;
  version: number;
  source: "builtin" | "user" | "imported";
  category: string;
  tags: string[];
  inputSchema: Record<string, unknown>;
  outputSchema: Record<string, unknown>;
  requiresUserConfirmation: boolean;
  writesToProject: boolean;
  author?: string;
  icon?: string;
  createdAt: string;
  updatedAt: string;
  skillClass?: "workflow" | "capability" | "extractor" | "review" | "policy";
  bundleIds: string[];
  alwaysOn: boolean;
  triggerConditions: string[];
  requiredContexts: string[];
  stateWrites: string[];
  workflowStages: string[];
  postTasks: string[];
  automationTier?: "auto" | "supervised" | "confirm";
  sceneTags: string[];
  affectsLayers: string[];
}

export interface CreateSkillInput {
  id: string;
  name: string;
  description: string;
  category?: string;
  tags?: string[];
  icon?: string;
  body: string;
}

export interface SkillManifestPatch {
  name?: string;
  description?: string;
  category?: string;
  tags?: string[];
  icon?: string;
  skillClass?: SkillManifest["skillClass"] | "";
  bundleIds?: string[];
  alwaysOn?: boolean;
  triggerConditions?: string[];
  requiredContexts?: string[];
  stateWrites?: string[];
  workflowStages?: string[];
  postTasks?: string[];
  automationTier?: SkillManifest["automationTier"] | "";
  sceneTags?: string[];
  affectsLayers?: string[];
}

export interface UpdateSkillInput {
  id: string;
  body?: string;
  manifest?: SkillManifestPatch;
}

// ── Commands ──

export async function listSkills(): Promise<SkillManifest[]> {
  return invokeCommand<SkillManifest[]>("list_skills");
}

export async function getSkill(id: string): Promise<SkillManifest> {
  return invokeCommand<SkillManifest>("get_skill", { id });
}

export async function getSkillContent(id: string): Promise<string> {
  return invokeCommand<string>("get_skill_content", { id });
}

export async function createSkill(input: CreateSkillInput): Promise<SkillManifest> {
  return invokeCommand<SkillManifest>("create_skill", { input });
}

export async function updateSkill(input: UpdateSkillInput): Promise<SkillManifest> {
  return invokeCommand<SkillManifest>("update_skill", { input });
}

export async function deleteSkill(id: string): Promise<void> {
  await invokeCommand<void>("delete_skill", { id });
}

export async function importSkillFile(filePath: string): Promise<SkillManifest> {
  return invokeCommand<SkillManifest>("import_skill_file", { filePath });
}

export async function resetBuiltinSkill(id: string): Promise<SkillManifest> {
  return invokeCommand<SkillManifest>("reset_builtin_skill", { id });
}

export async function refreshSkills(): Promise<SkillManifest[]> {
  return invokeCommand<SkillManifest[]>("refresh_skills");
}
