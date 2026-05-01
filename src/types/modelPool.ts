export type ModelPoolRole = "planner" | "drafter" | "reviewer" | "extractor" | "state";

export interface ModelPoolEntry {
  providerId: string;
  modelId: string;
}

export interface ModelPool {
  id: string;
  displayName: string;
  role: ModelPoolRole | string;
  enabled: boolean;
  entries: ModelPoolEntry[];
  fallbackPoolId?: string;
  createdAt?: string;
  updatedAt?: string;
}

