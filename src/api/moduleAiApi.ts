import { streamTaskPipeline, type RunTaskPipelineInput } from "./pipelineApi.js";

export async function runModuleAiTask(input: RunTaskPipelineInput): Promise<string> {
  let output = "";
  for await (const event of streamTaskPipeline(input, { timeoutMs: 180000 })) {
    if (event.type === "delta" && event.delta) {
      output += event.delta;
      continue;
    }
    if (event.type === "error") {
      throw new Error(event.message || event.errorCode || "AI 任务执行失败");
    }
  }
  return output.trim();
}

export interface ModuleReviewWorkItem {
  id: string;
  key: string;
  title: string;
  severity: string;
  message: string;
  status: string;
}

export interface ModuleReviewChecklistItem {
  key: string;
  title: string;
  severity: string;
  status: string;
  message: string;
}

export interface ModuleAiTaskResult {
  output: string;
  taskContract: Record<string, unknown> | null;
  contextCompilationSnapshot: Record<string, unknown> | null;
  reviewChecklist: ModuleReviewChecklistItem[];
  reviewWorkItems: ModuleReviewWorkItem[];
  checkpointId: string | null;
}

export async function runModuleAiTaskWithMeta(
  input: RunTaskPipelineInput
): Promise<ModuleAiTaskResult> {
  let output = "";
  let taskContract: Record<string, unknown> | null = null;
  let contextCompilationSnapshot: Record<string, unknown> | null = null;
  let reviewChecklist: ModuleReviewChecklistItem[] = [];
  let reviewWorkItems: ModuleReviewWorkItem[] = [];
  let checkpointId: string | null = null;

  for await (const event of streamTaskPipeline(input, { timeoutMs: 180000 })) {
    if (event.type === "delta" && event.delta) {
      output += event.delta;
      continue;
    }
    if (event.meta && typeof event.meta === "object") {
      const meta = event.meta as Record<string, unknown>;
      const contract = meta.taskContract;
      if (contract && typeof contract === "object") {
        taskContract = contract as Record<string, unknown>;
      }
      const snapshot = meta.contextCompilationSnapshot;
      if (snapshot && typeof snapshot === "object") {
        contextCompilationSnapshot = snapshot as Record<string, unknown>;
      }
      const checklist = meta.reviewChecklist;
      if (Array.isArray(checklist)) {
        reviewChecklist = checklist
          .filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object")
          .map((item) => ({
            key: typeof item.key === "string" ? item.key : "review-item",
            title: typeof item.title === "string" ? item.title : "审查项",
            severity: typeof item.severity === "string" ? item.severity : "medium",
            status: typeof item.status === "string" ? item.status : "pending",
            message: typeof item.message === "string" ? item.message : "",
          }));
      }
      const workItems = meta.reviewWorkItems;
      if (Array.isArray(workItems)) {
        reviewWorkItems = workItems
          .filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object")
          .map((item) => ({
            id: typeof item.id === "string" ? item.id : "",
            key: typeof item.key === "string" ? item.key : "review-item",
            title: typeof item.title === "string" ? item.title : "审查工单",
            severity: typeof item.severity === "string" ? item.severity : "medium",
            message: typeof item.message === "string" ? item.message : "",
            status: typeof item.status === "string" ? item.status : "pending",
          }))
          .filter((item) => item.id);
      }
      const cp =
        (typeof meta.checkpointId === "string" && meta.checkpointId) ||
        (typeof meta.storyCheckpointId === "string" && meta.storyCheckpointId) ||
        null;
      if (cp) checkpointId = cp;
    }
    if (event.type === "error") {
      throw new Error(event.message || event.errorCode || "AI 任务执行失败");
    }
  }

  return {
    output: output.trim(),
    taskContract,
    contextCompilationSnapshot,
    reviewChecklist,
    reviewWorkItems,
    checkpointId,
  };
}

