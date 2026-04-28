import { invokeCommand } from "./tauriClient.js";
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
