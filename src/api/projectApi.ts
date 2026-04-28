import { invokeCommand } from "./tauriClient.js";
import type { CreateProjectInput, ProjectJson } from "../domain/types.js";

export interface ValidateProjectInput {
  name: string;
  forceError?: boolean;
}

export interface ValidateProjectOutput {
  normalizedName: string;
  message: string;
}

export interface ProjectOpenResult {
  projectRoot: string;
  project: ProjectJson;
}

export interface RecentProjectItem {
  projectPath: string;
  openedAt: string;
}

export async function validateProjectName(
  input: ValidateProjectInput
): Promise<ValidateProjectOutput> {
  return invokeCommand<ValidateProjectOutput>("validate_project", { input });
}

export async function createProject(input: CreateProjectInput): Promise<ProjectOpenResult> {
  return invokeCommand<ProjectOpenResult>("create_project", { input });
}

export async function openProject(projectRoot: string): Promise<ProjectOpenResult> {
  return invokeCommand<ProjectOpenResult>("open_project", { input: { projectRoot } });
}

export async function listRecentProjects(): Promise<RecentProjectItem[]> {
  return invokeCommand<RecentProjectItem[]>("list_recent_projects");
}

export async function clearRecentProjects(): Promise<void> {
  return invokeCommand<void>("clear_recent_projects");
}
