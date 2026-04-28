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

/**
 * Bridge Tauri event listener to an async generator.
 * Returns an async generator that yields AiStreamEvents as they arrive.
 */
function createEventStream<T>(
  requestId: string,
  chunkEvent: string,
  doneEvent: string,
  mapPayload: (payload: T) => AiStreamEvent | null,
  timeoutMs: number = 30000,
): AsyncGenerator<AiStreamEvent> {
  return (async function* () {
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
      unlistenChunk();
      unlistenDone();
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
  const requestId = await invokeCommand<string>("stream_ai_generate", {
    req: {
      providerId: "default",
      model: "default",
      messages: [{ role: "user", content: input.userInstruction }],
      stream: true,
      taskType: input.taskType,
      maxOutputTokens: 4096,
    }
  });

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
    }
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
