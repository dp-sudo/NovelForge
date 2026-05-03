import { invokeCommand } from "./tauriClient.js";

// --- Types ---

export interface CharacterStateEntry {
  id: string;
  snapshotId: string;
  characterId: string;
  location: string | null;
  emotionalState: string | null;
  arcProgress: string | null;
  knowledgeGained: string | null;
  relationshipsChanged: string | null;
  statusNotes: string | null;
}

export interface PlotStateEntry {
  id: string;
  snapshotId: string;
  plotNodeId: string | null;
  progressStatus: string;
  tensionLevel: number | null;
  openThreads: string | null;
}

export interface WorldStateEntry {
  id: string;
  snapshotId: string;
  worldRuleId: string | null;
  stateDescription: string;
  changedInChapter: boolean;
}

export interface StoryStateSnapshot {
  id: string;
  projectId: string;
  chapterId: string;
  snapshotType: string;
  notes: string | null;
  createdAt: string;
  characterStates: CharacterStateEntry[];
  plotStates: PlotStateEntry[];
  worldStates: WorldStateEntry[];
}

export interface StateSnapshotSummary {
  snapshotId: string;
  chapterId: string;
  snapshotType: string;
  characterCount: number;
  plotCount: number;
  worldCount: number;
}

export interface CreateSnapshotInput {
  chapterId: string;
  snapshotType?: string;
  notes?: string;
  characterStates: Array<{
    characterId: string;
    location?: string;
    emotionalState?: string;
    arcProgress?: string;
    knowledgeGained?: string;
    relationshipsChanged?: string;
    statusNotes?: string;
  }>;
  plotStates: Array<{
    plotNodeId?: string;
    progressStatus: string;
    tensionLevel?: number;
    openThreads?: string;
  }>;
  worldStates: Array<{
    worldRuleId?: string;
    stateDescription: string;
    changedInChapter?: boolean;
  }>;
}

// --- API calls ---

export async function createStateSnapshot(
  projectRoot: string,
  input: CreateSnapshotInput
): Promise<string> {
  return invokeCommand<string>("create_state_snapshot", {
    projectRoot,
    input,
  });
}

export async function getLatestStateSnapshot(
  projectRoot: string,
  chapterId: string
): Promise<StoryStateSnapshot | null> {
  return invokeCommand<StoryStateSnapshot | null>(
    "get_latest_state_snapshot",
    { projectRoot, chapterId }
  );
}

export async function listStateSnapshots(
  projectRoot: string
): Promise<StateSnapshotSummary[]> {
  return invokeCommand<StateSnapshotSummary[]>("list_state_snapshots", {
    projectRoot,
  });
}

export async function deleteStateSnapshot(
  projectRoot: string,
  snapshotId: string
): Promise<void> {
  return invokeCommand<void>("delete_state_snapshot", {
    projectRoot,
    snapshotId,
  });
}

export async function getStatePromptText(
  projectRoot: string,
  chapterId: string
): Promise<string> {
  return invokeCommand<string>("get_state_prompt_text", {
    projectRoot,
    chapterId,
  });
}
