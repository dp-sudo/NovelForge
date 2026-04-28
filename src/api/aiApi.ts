import { invokeCommand } from "./tauriClient.js";
import type { AiPreviewRequest, AiPreviewResponse } from "../domain/types.js";
import { listen } from "@tauri-apps/api/event";
import {
  streamTaskPipeline,
  type AiPipelineEvent,
  type RunTaskPipelineInput,
  runTaskPipeline,
  cancelTaskPipeline,
} from "./pipelineApi.js";

export interface AiStreamEvent {
  requestId: string;
  type: "start" | "delta" | "progress" | "done" | "error";
  phase?: string;
  delta?: string;
  error?: string;
  errorCode?: string;
  message?: string;
  recoverable?: boolean;
  meta?: Record<string, unknown> | null;
  reasoning?: string;
}

const LEGACY_STREAM_START_TIMEOUT_MS = 15000;

interface EventStreamOptions {
  timeoutMs?: number;
  startOperation?: () => Promise<void>;
  startErrorFallback?: string;
}

async function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  onTimeout: () => unknown,
): Promise<T> {
  let timeoutHandle: ReturnType<typeof setTimeout> | null = null;
  const timeoutPromise = new Promise<T>((_, reject) => {
    timeoutHandle = setTimeout(() => reject(onTimeout()), timeoutMs);
  });
  try {
    return await Promise.race([promise, timeoutPromise]);
  } finally {
    if (timeoutHandle !== null) {
      clearTimeout(timeoutHandle);
    }
  }
}

function normalizeStartOperationError(
  error: unknown,
  fallback: string,
): Pick<AiStreamEvent, "error" | "errorCode"> {
  if (typeof error === "object" && error !== null) {
    const candidate = error as { code?: unknown; message?: unknown };
    if (typeof candidate.message === "string" && candidate.message.trim()) {
      return {
        error: candidate.message,
        errorCode: typeof candidate.code === "string" ? candidate.code : undefined,
      };
    }
  }
  if (error instanceof Error && error.message.trim()) {
    return { error: error.message };
  }
  return { error: fallback };
}

/**
 * Bridge Tauri event listener to an async generator.
 * Returns an async generator that yields AiStreamEvents as they arrive.
 */
function createEventStream<T>(
  requestId: string,
  chunkEvent: string,
  doneEvent: string,
  mapPayload: (payload: T) => AiStreamEvent | null,
  options: EventStreamOptions = {},
): AsyncGenerator<AiStreamEvent> {
  return (async function* () {
    const timeoutMs = options.timeoutMs ?? 30000;
    yield { requestId, type: "start" };

    const pending: AiStreamEvent[] = [];
    let resolve: (() => void) | null = null;
    let streamDone = false;
    let lastEventTime = Date.now();

    const unlistenChunk = await listen<T>(chunkEvent, (event) => {
      lastEventTime = Date.now();
      const mapped = mapPayload(event.payload);
      if (mapped) {
        pending.push(mapped);
        if (mapped.type === "done" || mapped.type === "error") {
          streamDone = true;
        }
      }
      if (resolve) {
        resolve();
        resolve = null;
      }
    });

    const unlistenDone = await listen(doneEvent, () => {
      if (!streamDone) {
        pending.push({ requestId, type: "done" });
        streamDone = true;
      }
      if (resolve) {
        resolve();
        resolve = null;
      }
    });

    try {
      if (options.startOperation) {
        try {
          await options.startOperation();
        } catch (error) {
          const normalized = normalizeStartOperationError(
            error,
            options.startErrorFallback ?? "AI 请求启动失败",
          );
          yield {
            requestId,
            type: "error",
            error: normalized.error,
            errorCode: normalized.errorCode,
          };
          return;
        }
      }
      while (!streamDone || pending.length > 0) {
        if (pending.length > 0) {
          const evt = pending.shift()!;
          lastEventTime = Date.now();
          yield evt;
          if (evt.type === "done" || evt.type === "error") return;
        } else {
          const waitPromise = new Promise<void>((r) => {
            resolve = r;
          });
          const elapsed = Date.now() - lastEventTime;
          const remaining = Math.max(0, timeoutMs - elapsed);
          if (remaining <= 0) {
            pending.push({ requestId, type: "error", error: "AI 响应超时，请检查网络连接" });
            streamDone = true;
            const evt = pending.shift()!;
            yield evt;
            return;
          }
          const timeoutPromise = new Promise<void>((_, reject) => {
            setTimeout(() => reject(new Error("TIMEOUT")), remaining);
          });
          try {
            await Promise.race([waitPromise, timeoutPromise]);
          } catch {
            pending.push({ requestId, type: "error", error: "AI 响应超时，请检查网络连接" });
            streamDone = true;
            const evt = pending.shift()!;
            yield evt;
            return;
          }
        }
      }
    } finally {
      await Promise.resolve(unlistenChunk()).catch(() => undefined);
      await Promise.resolve(unlistenDone()).catch(() => undefined);
    }
  })();
}

// ── Legacy non-streaming preview ──

export async function generateAiPreview(
  input: AiPreviewRequest,
  projectRoot?: string
): Promise<AiPreviewResponse> {
  return invokeCommand<AiPreviewResponse>("generate_ai_preview", {
    projectRoot,
    input: { taskType: input.taskType, userInstruction: input.userInstruction, chapterId: input.chapterId, selectedText: input.selectedText }
  });
}

// ── Chapter-aware streaming (uses Rust ContextService + PromptBuilder) ──

export interface ChapterTaskInput {
  projectRoot: string;
  chapterId: string;
  taskType: string;
  userInstruction: string;
  selectedText?: string;
}

export async function* streamAiChapterTask(
  input: ChapterTaskInput
): AsyncGenerator<AiStreamEvent> {
  const pipelineInput: RunTaskPipelineInput = {
    projectRoot: input.projectRoot,
    taskType: input.taskType,
    chapterId: input.chapterId,
    uiAction: "stream_ai_chapter_task",
    userInstruction: input.userInstruction,
    selectedText: input.selectedText,
  };

  for await (const event of streamTaskPipeline(pipelineInput)) {
    const mapped = mapPipelineEventToAiStream(event);
    if (mapped) {
      yield mapped;
    }
  }
}

// ── Legacy streaming generate (kept for backward compat) ──

export async function* streamAiGenerate(
  input: AiPreviewRequest
): AsyncGenerator<AiStreamEvent> {
  const requestId = globalThis.crypto?.randomUUID?.()
    ?? `legacy-stream-${Date.now()}-${Math.random().toString(16).slice(2)}`;

  yield* createEventStream<{ content: string; finishReason: string | null; requestId: string; error?: string; reasoning?: string }>(
    requestId,
    `ai:stream-chunk:${requestId}`,
    `ai:stream-done:${requestId}`,
    (payload) => {
      if (payload.error) {
        return { requestId, type: "error", error: payload.error };
      }
      if (payload.reasoning) {
        return { requestId, type: "delta", reasoning: payload.reasoning };
      }
      if (payload.content) {
        return { requestId, type: "delta", delta: payload.content };
      }
      if (payload.finishReason) {
        return { requestId, type: "done" };
      }
      return null;
    },
    {
      timeoutMs: 30000,
      startErrorFallback: "AI 请求启动失败，请重试",
      startOperation: async () => {
        const startTimeoutMs = LEGACY_STREAM_START_TIMEOUT_MS;
        const startedRequestId = await withTimeout(
          invokeCommand<string>("stream_ai_generate", {
            requestId,
            req: {
              providerId: "default",
              model: "default",
              messages: [{ role: "user", content: input.userInstruction }],
              stream: true,
              taskType: input.taskType,
              maxOutputTokens: 4096,
            }
          }),
          startTimeoutMs,
          () => ({
            code: "LEGACY_STREAM_START_TIMEOUT",
            message: `AI 请求启动超时（>${startTimeoutMs}ms），请重试`,
            recoverable: true,
          }),
        );
        if (startedRequestId !== requestId) {
          throw {
            code: "LEGACY_STREAM_REQUEST_ID_MISMATCH",
            message: "AI 请求标识不一致，请重试",
            recoverable: true,
          };
        }
      },
    },
  );
}

function mapPipelineEventToAiStream(event: AiPipelineEvent): AiStreamEvent | null {
  const base = {
    requestId: event.requestId,
    phase: event.phase,
    errorCode: event.errorCode,
    message: event.message,
    recoverable: event.recoverable,
    meta: event.meta,
  };

  if (event.type === "start") {
    return { ...base, type: "start" };
  }
  if (event.type === "progress") {
    return { ...base, type: "progress" };
  }
  if (event.type === "done") {
    return { ...base, type: "done" };
  }
  if (event.type === "error") {
    return {
      ...base,
      type: "error",
      error: event.message || event.errorCode || "AI 生成异常",
    };
  }
  if (event.type === "delta" && event.delta) {
    return { ...base, type: "delta", delta: event.delta };
  }
  return null;
}

export { runTaskPipeline, cancelTaskPipeline };
