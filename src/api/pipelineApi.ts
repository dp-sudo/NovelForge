import { listen } from "@tauri-apps/api/event";
import { invokeCommand } from "./tauriClient.js";

const PIPELINE_EVENT_NAME = "ai:pipeline:event";
const DEFAULT_EVENT_TIMEOUT_MS = 120000;
const MIN_EVENT_TIMEOUT_MS = 1000;

export type AiPipelinePhase =
  | "validate"
  | "context"
  | "route"
  | "prompt"
  | "generate"
  | "postprocess"
  | "persist"
  | "done"
  | string;

export type AiPipelineEventType = "start" | "delta" | "progress" | "done" | "error";

export interface RunTaskPipelineInput {
  projectRoot: string;
  taskType: string;
  chapterId?: string;
  uiAction?: string;
  userInstruction?: string;
  selectedText?: string;
  chapterContent?: string;
  blueprintStepKey?: string;
  blueprintStepTitle?: string;
}

export interface AiPipelineEvent {
  requestId: string;
  phase: AiPipelinePhase;
  type: AiPipelineEventType;
  delta?: string;
  errorCode?: string;
  message?: string;
  recoverable?: boolean;
  meta?: Record<string, unknown> | null;
}

export interface TaskPipelineStreamOptions {
  timeoutMs?: number;
  cancelOnExit?: boolean;
}

export async function runTaskPipeline(input: RunTaskPipelineInput): Promise<string> {
  return invokeCommand<string>("run_ai_task_pipeline", {
    input: {
      projectRoot: input.projectRoot,
      taskType: input.taskType,
      chapterId: input.chapterId,
      uiAction: input.uiAction,
      userInstruction: input.userInstruction ?? "",
      selectedText: input.selectedText,
      chapterContent: input.chapterContent,
      blueprintStepKey: input.blueprintStepKey,
      blueprintStepTitle: input.blueprintStepTitle,
    },
  });
}

export async function cancelTaskPipeline(requestId: string): Promise<void> {
  await invokeCommand<void>("cancel_ai_task_pipeline", { requestId });
}

export async function* streamTaskPipeline(
  input: RunTaskPipelineInput,
  options: TaskPipelineStreamOptions = {},
): AsyncGenerator<AiPipelineEvent> {
  const requestId = await runTaskPipeline(input);
  yield* streamTaskPipelineByRequestId(requestId, options);
}

export async function* streamTaskPipelineByRequestId(
  requestId: string,
  options: TaskPipelineStreamOptions = {},
): AsyncGenerator<AiPipelineEvent> {
  const pending: AiPipelineEvent[] = [];
  const timeoutMs = Math.max(
    options.timeoutMs ?? DEFAULT_EVENT_TIMEOUT_MS,
    MIN_EVENT_TIMEOUT_MS,
  );
  let done = false;
  let resolveWaiter: (() => void) | null = null;
  let lastEventTime = Date.now();

  const unlisten = await listen<unknown>(PIPELINE_EVENT_NAME, (event) => {
    const parsed = parsePipelineEvent(event.payload);
    if (!parsed || parsed.requestId !== requestId) {
      return;
    }
    pending.push(parsed);
    lastEventTime = Date.now();
    if (parsed.type === "done" || parsed.type === "error") {
      done = true;
    }
    if (resolveWaiter) {
      resolveWaiter();
      resolveWaiter = null;
    }
  });

  try {
    while (!done || pending.length > 0) {
      if (pending.length > 0) {
        yield pending.shift()!;
        continue;
      }

      const elapsed = Date.now() - lastEventTime;
      const remaining = timeoutMs - elapsed;
      if (remaining <= 0) {
        done = true;
        yield {
          requestId,
          phase: "done",
          type: "error",
          errorCode: "PIPELINE_EVENT_TIMEOUT",
          message: "AI 响应超时，请检查网络连接",
          recoverable: true,
          meta: null,
        };
        break;
      }

      try {
        const waitPromise = new Promise<void>((resolve) => {
          resolveWaiter = resolve;
          if (pending.length > 0) {
            resolveWaiter();
            resolveWaiter = null;
          }
        });
        const timeoutPromise = new Promise<void>((_, reject) => {
          setTimeout(() => reject(new Error("TIMEOUT")), remaining);
        });
        await Promise.race([waitPromise, timeoutPromise]);
      } catch {
        resolveWaiter = null;
        done = true;
        yield {
          requestId,
          phase: "done",
          type: "error",
          errorCode: "PIPELINE_EVENT_TIMEOUT",
          message: "AI 响应超时，请检查网络连接",
          recoverable: true,
          meta: null,
        };
        break;
      }
    }
  } finally {
    unlisten();
    if (!done && options.cancelOnExit) {
      await cancelTaskPipeline(requestId).catch(() => undefined);
    }
  }
}

function parsePipelineEvent(payload: unknown): AiPipelineEvent | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }
  const candidate = payload as Record<string, unknown>;
  const requestId = asString(candidate.requestId);
  const phase = asString(candidate.phase);
  const eventType = asString(candidate.type);
  if (!requestId || !phase || !eventType) {
    return null;
  }

  return {
    requestId,
    phase,
    type: eventType as AiPipelineEventType,
    delta: asOptionalString(candidate.delta),
    errorCode: asOptionalString(candidate.errorCode),
    message: asOptionalString(candidate.message),
    recoverable: typeof candidate.recoverable === "boolean" ? candidate.recoverable : undefined,
    meta: toMeta(candidate.meta),
  };
}

function asString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function asOptionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function toMeta(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  return value as Record<string, unknown>;
}
