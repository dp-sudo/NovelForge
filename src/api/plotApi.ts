import { invokeCommand } from "./tauriClient.js";
import { runModuleAiTask } from "./moduleAiApi.js";
import type { PlotNodeInput } from "../domain/types.js";

export interface PlotRow {
  id: string;
  project_id: string;
  title: string;
  node_type: string;
  sort_order: number;
  goal: string | null;
  conflict: string | null;
  emotional_curve: string | null;
  status: string;
  related_characters: string[];
  created_at: string;
  updated_at: string;
}

export async function listPlotNodes(projectRoot: string): Promise<PlotRow[]> {
  return invokeCommand<PlotRow[]>("list_plot_nodes", { projectRoot });
}

export async function createPlotNode(input: PlotNodeInput, projectRoot: string): Promise<string> {
  return invokeCommand<string>("create_plot_node", { projectRoot, input });
}

export async function reorderPlotNodes(orderedIds: string[], projectRoot: string): Promise<void> {
  await invokeCommand<void>("reorder_plot_nodes", { projectRoot, orderedIds });
}

export async function aiGeneratePlotNode(projectRoot: string, userDescription: string): Promise<string> {
  return runModuleAiTask({
    projectRoot,
    taskType: "plot.create_node",
    userInstruction: userDescription,
    autoPersist: true,
    persistMode: "formal",
    automationTier: "supervised",
    uiAction: "ai_generate_plot_node",
  });
}
