import { invokeCommand } from "./tauriClient.js";

export interface SearchResult {
  entityType: string;
  entityId: string;
  title: string;
  bodySnippet: string;
  rank: number;
}

export async function searchProject(projectRoot: string, query: string, limit?: number): Promise<SearchResult[]> {
  return invokeCommand<SearchResult[]>("search_project", { projectRoot, query, limit });
}

export async function rebuildSearchIndex(projectRoot: string): Promise<number> {
  return invokeCommand<number>("rebuild_search_index", { projectRoot });
}
