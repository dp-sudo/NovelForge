import { invokeCommand } from "./tauriClient.js";
import type { AiPreviewRequest, AiPreviewResponse } from "../domain/types.js";
import { listen } from "@tauri-apps/api/event";

export interface AiStreamEvent {
  requestId: string;
  type: "start" | "delta" | "done" | "error";
  delta?: string;
  error?: string;
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
): AsyncGenerator<AiStreamEvent> {
  return (async function* () {
    yield { requestId, type: "start" };

    const pending: AiStreamEvent[] = [];
    let resolve: (() => void) | null = null;
    let streamDone = false;

    const unlistenChunk = await listen<T>(chunkEvent, (event) => {
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
          yield evt;
          if (evt.type === "done" || evt.type === "error") return;
        } else {
          await new Promise<void>((r) => {
            resolve = r;
          });
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
  const requestId = await invokeCommand<string>("stream_ai_chapter_task", { input });

  yield* createEventStream<{ content: string; finishReason: string | null; requestId: string }>(
    requestId,
    `ai:stream-chunk:${requestId}`,
    `ai:stream-done:${requestId}`,
    (payload) => {
      if (payload.content) {
        return { requestId, type: "delta", delta: payload.content };
      }
      return null;
    }
  );
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

  yield* createEventStream<{ content: string; finishReason: string | null; requestId: string }>(
    requestId,
    `ai:stream-chunk:${requestId}`,
    `ai:stream-done:${requestId}`,
    (payload) => {
      if (payload.content) {
        return { requestId, type: "delta", delta: payload.content };
      }
      return null;
    }
  );
}
