import { create } from "zustand";
import type { ProjectJson } from "../domain/types.js";

export interface ProjectState {
  currentProjectPath: string | null;
  currentProject: ProjectJson | null;
  recentProjects: Array<{ projectPath: string; openedAt: string }>;
  stats: {
    totalWords: number;
    chapterCount: number;
    characterCount: number;
    worldRuleCount: number;
    plotNodeCount: number;
    openIssueCount: number;
    blueprintProgress: number;
  } | null;
  setCurrentProject: (path: string, project: ProjectJson) => void;
  clearCurrentProject: () => void;
  setRecentProjects: (
    projects: Array<{ projectPath: string; openedAt: string }>
  ) => void;
  setStats: (stats: ProjectState["stats"]) => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  currentProjectPath: null,
  currentProject: null,
  recentProjects: [],
  stats: null,
  setCurrentProject: (path, project) =>
    set({ currentProjectPath: path, currentProject: project }),
  clearCurrentProject: () =>
    set({
      currentProjectPath: null,
      currentProject: null,
      stats: null
    }),
  setRecentProjects: (projects) => set({ recentProjects: projects }),
  setStats: (stats) => set({ stats })
}));
