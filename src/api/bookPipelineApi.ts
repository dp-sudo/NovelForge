import {
  cancelTaskPipeline,
  streamTaskPipeline,
  extractPipelineMeta,
  type AiPipelineEvent,
  type RunTaskPipelineInput,
  type PipelineReviewWorkItem,
  type PipelineReviewChecklistItem,
} from "./pipelineApi.js";

export type BookStageKey =
  | "blueprint-anchor"
  | "blueprint-genre"
  | "blueprint-premise"
  | "character-seed"
  | "world-seed"
  | "plot-seed"
  | "chapter-plan";

export type BookStage = {
  key: BookStageKey;
  label: string;
  request: RunTaskPipelineInput;
};

export interface RunBookGenerationInput {
  projectRoot: string;
  ideaPrompt: string;
  chapterId?: string;
}

export type BookPipelineEvent =
  | {
    type: "stage-start";
    sessionId: string;
    stageKey: BookStageKey;
    stageLabel: string;
  }
  | {
    type: "stage-delta";
    sessionId: string;
    stageKey: BookStageKey;
    stageLabel: string;
    event: AiPipelineEvent;
    meta: {
      taskContract: Record<string, unknown> | null;
      contextCompilationSnapshot: Record<string, unknown> | null;
      reviewChecklist: PipelineReviewChecklistItem[];
      reviewWorkItems: PipelineReviewWorkItem[];
      checkpointId: string | null;
    };
  }
  | {
    type: "stage-done";
    sessionId: string;
    stageKey: BookStageKey;
    stageLabel: string;
    requestId: string;
    checkpointId: string | null;
    reviewWorkItems: PipelineReviewWorkItem[];
  }
  | {
    type: "stage-error";
    sessionId: string;
    stageKey: BookStageKey;
    stageLabel: string;
    requestId?: string;
    errorCode?: string;
    message: string;
  };

export function buildBookStages(input: RunBookGenerationInput): BookStage[] {
  const base = input.ideaPrompt.trim();
  const stages: BookStage[] = [
    {
      key: "blueprint-anchor",
      label: "蓝图: 灵感定锚",
      request: {
        projectRoot: input.projectRoot,
        taskType: "blueprint.generate_step",
        userInstruction: base,
        blueprintStepKey: "step-01-anchor",
        blueprintStepTitle: "灵感定锚",
        autoPersist: true,
      },
    },
    {
      key: "blueprint-genre",
      label: "蓝图: 类型策略",
      request: {
        projectRoot: input.projectRoot,
        taskType: "blueprint.generate_step",
        userInstruction: base,
        blueprintStepKey: "step-02-genre",
        blueprintStepTitle: "类型策略",
        autoPersist: true,
      },
    },
    {
      key: "blueprint-premise",
      label: "蓝图: 故事母题",
      request: {
        projectRoot: input.projectRoot,
        taskType: "blueprint.generate_step",
        userInstruction: base,
        blueprintStepKey: "step-03-premise",
        blueprintStepTitle: "故事母题",
        autoPersist: true,
      },
    },
    {
      key: "character-seed",
      label: "角色: 核心角色草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "character.create",
        userInstruction: base,
        autoPersist: true,
      },
    },
    {
      key: "world-seed",
      label: "设定: 世界规则草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "world.create_rule",
        userInstruction: base,
        autoPersist: true,
      },
    },
    {
      key: "plot-seed",
      label: "剧情: 主线节点草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "plot.create_node",
        userInstruction: base,
        autoPersist: true,
      },
    },
  ];
  if (input.chapterId) {
    stages.push({
      key: "chapter-plan",
      label: "章节: 章节计划",
      request: {
        projectRoot: input.projectRoot,
        taskType: "chapter.plan",
        chapterId: input.chapterId,
        userInstruction: base,
        autoPersist: false,
      },
    });
  }
  return stages;
}

function createSessionId(): string {
  const randomPart = typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return `book-pipeline-${randomPart}`;
}

export async function* streamBookGenerationPipeline(
  input: RunBookGenerationInput,
  signal?: AbortSignal,
): AsyncGenerator<BookPipelineEvent> {
  const sessionId = createSessionId();
  const stages = buildBookStages(input);

  for (const stage of stages) {
    if (signal?.aborted) {
      yield {
        type: "stage-error",
        sessionId,
        stageKey: stage.key,
        stageLabel: stage.label,
        message: "全书编排任务已取消",
      };
      return;
    }

    yield {
      type: "stage-start",
      sessionId,
      stageKey: stage.key,
      stageLabel: stage.label,
    };

    let latestRequestId: string | undefined;
    let latestCheckpointId: string | null = null;
    let latestReviewWorkItems: PipelineReviewWorkItem[] = [];
    try {
      for await (const event of streamTaskPipeline(stage.request, { timeoutMs: 180000 })) {
        latestRequestId = event.requestId;
        const extracted = extractPipelineMeta(event.meta);
        if (extracted.checkpointId) {
          latestCheckpointId = extracted.checkpointId;
        }
        if (extracted.reviewWorkItems.length > 0) {
          latestReviewWorkItems = extracted.reviewWorkItems;
        }
        if (signal?.aborted) {
          if (latestRequestId) {
            await cancelTaskPipeline(latestRequestId, "book_pipeline_abort");
          }
          yield {
            type: "stage-error",
            sessionId,
            stageKey: stage.key,
            stageLabel: stage.label,
            requestId: latestRequestId,
            message: "全书编排任务已取消",
          };
          return;
        }
        if (event.type === "error") {
          yield {
            type: "stage-error",
            sessionId,
            stageKey: stage.key,
            stageLabel: stage.label,
            requestId: event.requestId,
            errorCode: event.errorCode,
            message: event.message ?? event.errorCode ?? "编排阶段执行失败",
          };
          return;
        }
        yield {
          type: "stage-delta",
          sessionId,
          stageKey: stage.key,
          stageLabel: stage.label,
          event,
          meta: {
            taskContract: extracted.taskContract,
            contextCompilationSnapshot: extracted.contextCompilationSnapshot,
            reviewChecklist: extracted.reviewChecklist,
            reviewWorkItems: extracted.reviewWorkItems,
            checkpointId: extracted.checkpointId,
          },
        };
      }
      if (latestRequestId) {
        yield {
          type: "stage-done",
          sessionId,
          stageKey: stage.key,
          stageLabel: stage.label,
          requestId: latestRequestId,
          checkpointId: latestCheckpointId,
          reviewWorkItems: latestReviewWorkItems,
        };
      }
    } catch (error) {
      yield {
        type: "stage-error",
        sessionId,
        stageKey: stage.key,
        stageLabel: stage.label,
        requestId: latestRequestId,
        message: error instanceof Error ? error.message : "编排阶段执行失败",
      };
      return;
    }
  }
}
