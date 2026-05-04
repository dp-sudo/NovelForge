import { streamTaskPipeline, extractPipelineMeta, type RunTaskPipelineInput, type PipelineReviewChecklistItem, type PipelineReviewWorkItem } from "./pipelineApi.js";

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

export interface ModuleAiTaskResult {
  output: string;
  taskContract: Record<string, unknown> | null;
  contextCompilationSnapshot: Record<string, unknown> | null;
  reviewChecklist: PipelineReviewChecklistItem[];
  reviewWorkItems: PipelineReviewWorkItem[];
  checkpointId: string | null;
}

export async function runModuleAiTaskWithMeta(
  input: RunTaskPipelineInput
): Promise<ModuleAiTaskResult> {
  let output = "";
  let taskContract: Record<string, unknown> | null = null;
  let contextCompilationSnapshot: Record<string, unknown> | null = null;
  let reviewChecklist: PipelineReviewChecklistItem[] = [];
  let reviewWorkItems: PipelineReviewWorkItem[] = [];
  let checkpointId: string | null = null;

  for await (const event of streamTaskPipeline(input, { timeoutMs: 180000 })) {
    if (event.type === "delta" && event.delta) {
      output += event.delta;
      continue;
    }
    if (event.meta && typeof event.meta === "object") {
      const extracted = extractPipelineMeta(event.meta);
      if (extracted.taskContract) taskContract = extracted.taskContract;
      if (extracted.contextCompilationSnapshot) contextCompilationSnapshot = extracted.contextCompilationSnapshot;
      if (extracted.reviewChecklist.length > 0) reviewChecklist = extracted.reviewChecklist;
      if (extracted.reviewWorkItems.length > 0) reviewWorkItems = extracted.reviewWorkItems;
      if (extracted.checkpointId) checkpointId = extracted.checkpointId;
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

