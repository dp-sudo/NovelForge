import { invokeCommand } from "./tauriClient.js";

export interface NarrativeObligation {
  id: string;
  projectId: string;
  obligationType: string;
  description: string;
  plantedChapterId?: string | null;
  expectedPayoffChapterId?: string | null;
  actualPayoffChapterId?: string | null;
  payoffStatus: string;
  severity: string;
  relatedEntities?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateNarrativeObligationInput {
  obligationType: string;
  description: string;
  plantedChapterId?: string;
  expectedPayoffChapterId?: string;
  actualPayoffChapterId?: string;
  payoffStatus?: string;
  severity?: string;
  relatedEntities?: string;
}

export async function listNarrativeObligations(projectRoot: string): Promise<NarrativeObligation[]> {
  return invokeCommand<NarrativeObligation[]>("list_narrative_obligations", { projectRoot });
}

export async function createNarrativeObligation(
  projectRoot: string,
  input: CreateNarrativeObligationInput,
): Promise<string> {
  return invokeCommand<string>("create_narrative_obligation", { projectRoot, input });
}

export async function updateObligationStatus(
  projectRoot: string,
  id: string,
  status: string,
): Promise<void> {
  return invokeCommand<void>("update_obligation_status", { projectRoot, id, status });
}

export async function deleteNarrativeObligation(projectRoot: string, id: string): Promise<void> {
  return invokeCommand<void>("delete_narrative_obligation", { projectRoot, id });
}

export async function aiGenerateNarrativeObligation(
  projectRoot: string,
  userDescription: string,
): Promise<string> {
  return invokeCommand<string>("ai_generate_narrative_obligation", {
    input: { projectRoot, userDescription },
  });
}
