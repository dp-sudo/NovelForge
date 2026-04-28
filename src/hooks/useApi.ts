import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "./query-keys.js";
import { listRecentProjects, type RecentProjectItem } from "../api/projectApi.js";
import { listChapters, listVolumes, type ChapterRecord, type VolumeRecord } from "../api/chapterApi.js";
import { listCharacters, type CharacterRow } from "../api/characterApi.js";
import { listWorldRules, type WorldRow } from "../api/worldApi.js";
import { listPlotNodes, type PlotRow } from "../api/plotApi.js";
import { listGlossaryTerms, type GlossaryRow } from "../api/glossaryApi.js";
import { scanFullConsistency, type ConsistencyIssueRow } from "../api/consistencyApi.js";
import { listProviders, listTaskRoutes, loadEditorSettings, saveEditorSettings } from "../api/settingsApi.js";
import type { LlmProviderConfig, TaskRoute, EditorSettingsData } from "../api/settingsApi.js";
import { getDashboardStats, type DashboardStats } from "../api/statsApi.js";
import { listBlueprintSteps, type BlueprintStepRow } from "../api/blueprintApi.js";

// ── Recent projects ──

export function useRecentProjects() {
  return useQuery({
    queryKey: queryKeys.project.recent(),
    queryFn: listRecentProjects,
  });
}

// ── Dashboard stats ──

export function useDashboardStats(projectRoot: string | null) {
  return useQuery({
    queryKey: queryKeys.project.stats(projectRoot ?? ""),
    queryFn: () => getDashboardStats(projectRoot!),
    enabled: !!projectRoot,
  });
}

// ── Blueprint steps ──

export function useBlueprintSteps(projectRoot: string | null) {
  return useQuery({
    queryKey: queryKeys.blueprint.all(projectRoot ?? ""),
    queryFn: () => listBlueprintSteps(projectRoot!),
    enabled: !!projectRoot,
  });
}

// ── Chapters ──

export function useChapters(projectRoot: string | null) {
  return useQuery({
    queryKey: queryKeys.chapter.list(projectRoot ?? ""),
    queryFn: () => listChapters(projectRoot!),
    enabled: !!projectRoot,
  });
}

export function useVolumes(projectRoot: string | null) {
  return useQuery({
    queryKey: queryKeys.chapter.volumes(projectRoot ?? ""),
    queryFn: () => listVolumes(projectRoot!),
    enabled: !!projectRoot,
  });
}

// ── Characters ──

export function useCharacters(projectRoot: string | null) {
  return useQuery({
    queryKey: queryKeys.character.all(projectRoot ?? ""),
    queryFn: () => listCharacters(projectRoot!),
    enabled: !!projectRoot,
  });
}

// ── World rules ──

export function useWorldRules(projectRoot: string | null) {
  return useQuery({
    queryKey: queryKeys.world.all(projectRoot ?? ""),
    queryFn: () => listWorldRules(projectRoot!),
    enabled: !!projectRoot,
  });
}

// ── Plot nodes ──

export function usePlotNodes(projectRoot: string | null) {
  return useQuery({
    queryKey: queryKeys.plot.all(projectRoot ?? ""),
    queryFn: () => listPlotNodes(projectRoot!),
    enabled: !!projectRoot,
  });
}

// ── Glossary terms ──

export function useGlossaryTerms(projectRoot: string | null) {
  return useQuery({
    queryKey: ["glossary", projectRoot],
    queryFn: () => listGlossaryTerms(projectRoot!),
    enabled: !!projectRoot,
  });
}

// ── Consistency issues ──

export function useConsistencyIssues(projectRoot: string | null) {
  return useQuery({
    queryKey: queryKeys.consistency.all(projectRoot ?? ""),
    queryFn: () => scanFullConsistency(projectRoot!),
    enabled: !!projectRoot,
  });
}

// ── Settings ──

export function useProviders() {
  return useQuery({
    queryKey: queryKeys.settings.providers(),
    queryFn: listProviders,
    staleTime: 60_000,
  });
}

export function useTaskRoutes() {
  return useQuery({
    queryKey: queryKeys.settings.taskRoutes(),
    queryFn: listTaskRoutes,
    staleTime: 60_000,
  });
}

export function useEditorSettings() {
  return useQuery({
    queryKey: queryKeys.settings.editor(),
    queryFn: loadEditorSettings,
    staleTime: 30_000,
  });
}

export function useSaveEditorSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (settings: EditorSettingsData) => saveEditorSettings(settings),
    onSuccess: () => { qc.invalidateQueries({ queryKey: queryKeys.settings.editor() }); },
  });
}

// ── Invalidation helper for mutations ──

export function useInvalidateProject(root: string) {
  const qc = useQueryClient();
  return {
    chapters: () => qc.invalidateQueries({ queryKey: queryKeys.chapter.list(root) }),
    characters: () => qc.invalidateQueries({ queryKey: queryKeys.character.all(root) }),
    world: () => qc.invalidateQueries({ queryKey: queryKeys.world.all(root) }),
    plot: () => qc.invalidateQueries({ queryKey: queryKeys.plot.all(root) }),
    glossary: () => qc.invalidateQueries({ queryKey: ["glossary", root] }),
    blueprint: () => qc.invalidateQueries({ queryKey: queryKeys.blueprint.all(root) }),
    consistency: () => qc.invalidateQueries({ queryKey: queryKeys.consistency.all(root) }),
    all: () => qc.invalidateQueries({ queryKey: ["project", root] }),
  };
}
