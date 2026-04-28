import { listen } from "@tauri-apps/api/event";
import { invokeCommand, logUI, registerUnloadCleanup } from "./tauriClient.js";

const PIPELINE_EVENT_NAME = "ai:pipeline:event";
const DEFAULT_EVENT_TIMEOUT_MS = 120000;
const MIN_EVENT_TIMEOUT_MS = 1000;
const DEFAULT_START_TIMEOUT_MS = 15000;
const DEFAULT_FIRST_EVENT_TIMEOUT_MS = 15000;

const activeRequestIds = new Set<string>();
let unloadCleanupBound = false;

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
  startTimeoutMs?: number;
}

interface RunTaskPipelineOptions {
  timeoutMs?: number;
}

interface PipelineStartTimeoutError {
  code: string;
  message: string;
  recoverable: boolean;
}

function createPipelineStartTimeoutError(timeoutMs: number): PipelineStartTimeoutError {
  return {
    code: "PIPELINE_START_TIMEOUT",
    message: `AI 请求启动超时（>${timeoutMs}ms），请重试`,
    recoverable: true,
  };
}

function trackRequestId(requestId: string): void {
  if (!requestId) return;
  activeRequestIds.add(requestId);
  bindUnloadPipelineCleanupOnce();
}

function untrackRequestId(requestId: string | null): void {
  if (!requestId) return;
  activeRequestIds.delete(requestId);
}

function bindUnloadPipelineCleanupOnce(): void {
  if (unloadCleanupBound) {
    return;
  }
  unloadCleanupBound = true;
  registerUnloadCleanup((reason) => {
    if (activeRequestIds.size === 0) {
      return;
    }
    const requestIds = Array.from(activeRequestIds);
    activeRequestIds.clear();
    logUI("PIPELINE.CANCEL_ON_UNLOAD", `reason=${reason} count=${requestIds.length}`);
    void Promise.allSettled(
      requestIds.map((requestId) =>
        invokeCommand<void>("cancel_ai_task_pipeline", { requestId }).catch(() => undefined),
      ),
    );
  });
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

export async function runTaskPipeline(
  input: RunTaskPipelineInput,
  options: RunTaskPipelineOptions = {},
): Promise<string> {
  const timeoutMs = Math.max(options.timeoutMs ?? DEFAULT_START_TIMEOUT_MS, MIN_EVENT_TIMEOUT_MS);
  const requestPromise = invokeCommand<string>("run_ai_task_pipeline", {
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
  const requestId = await withTimeout(
    requestPromise,
    timeoutMs,
    () => createPipelineStartTimeoutError(timeoutMs),
  );
  trackRequestId(requestId);
  logUI("PIPELINE.START", `requestId=${requestId} taskType=${input.taskType}`);
  return requestId;
}

export async function cancelTaskPipeline(requestId: string, reason: string = "manual"): Promise<void> {
  logUI("PIPELINE.CANCEL", `requestId=${requestId} reason=${reason}`);
  try {
    await invokeCommand<void>("cancel_ai_task_pipeline", { requestId });
    untrackRequestId(requestId);
    logUI("PIPELINE.CANCELLED", `requestId=${requestId} reason=${reason}`);
  } catch (error) {
    logUI("PIPELINE.CANCEL_ERROR", `requestId=${requestId} reason=${reason}`);
    throw error;
  }
}

export async function* streamTaskPipeline(
  input: RunTaskPipelineInput,
  options: TaskPipelineStreamOptions = {},
): AsyncGenerator<AiPipelineEvent> {
  const timeoutMs = Math.max(
    options.timeoutMs ?? DEFAULT_EVENT_TIMEOUT_MS,
    MIN_EVENT_TIMEOUT_MS,
  );
  const startTimeoutMs = Math.max(
    options.startTimeoutMs ?? DEFAULT_START_TIMEOUT_MS,
    MIN_EVENT_TIMEOUT_MS,
  );
  const cancelOnExit = options.cancelOnExit ?? true;
  const pending: AiPipelineEvent[] = [];
  const pendingBeforeRequest = new Map<string, AiPipelineEvent[]>();

  let requestId: string | null = null;
  let done = false;
  let terminalLogged = false;
  let resolveWaiter: (() => void) | null = null;
  let lastEventTime = Date.now();

  const acceptEvent = (event: AiPipelineEvent) => {
    pending.push(event);
    lastEventTime = Date.now();
    if (event.type === "done" || event.type === "error") {
      done = true;
      untrackRequestId(event.requestId);
      if (!terminalLogged && requestId) {
        const action = event.type === "done" ? "PIPELINE.DONE" : "PIPELINE.ERROR";
        const detailParts = [
          `requestId=${requestId}`,
          `phase=${event.phase}`,
          event.errorCode ? `errorCode=${event.errorCode}` : undefined,
        ].filter(Boolean);
        logUI(action, detailParts.join(" "));
        terminalLogged = true;
      }
    }
    if (resolveWaiter) {
      resolveWaiter();
      resolveWaiter = null;
    }
  };

  const unlisten = await listen<unknown>(PIPELINE_EVENT_NAME, (event) => {
    const parsed = parsePipelineEvent(event.payload);
    if (!parsed) {
      return;
    }
    if (!requestId) {
      const queue = pendingBeforeRequest.get(parsed.requestId);
      if (queue) {
        queue.push(parsed);
      } else {
        pendingBeforeRequest.set(parsed.requestId, [parsed]);
      }
      return;
    }
    if (parsed.requestId !== requestId) {
      return;
    }
    acceptEvent(parsed);
  });

  try {
    requestId = await runTaskPipeline(input, { timeoutMs: startTimeoutMs });
    acceptEvent({
      requestId,
      phase: "run",
      type: "start",
      message: "pipeline accepted",
      meta: null,
    });
    const earlyEvents = pendingBeforeRequest.get(requestId);
    if (earlyEvents) {
      for (const event of earlyEvents) {
        acceptEvent(event);
      }
      pendingBeforeRequest.delete(requestId);
    }

    while (!done || pending.length > 0) {
      if (pending.length > 0) {
        yield pending.shift()!;
        continue;
      }

      const elapsed = Date.now() - lastEventTime;
      const remaining = timeoutMs - elapsed;
      if (remaining <= 0) {
        done = true;
        if (!terminalLogged) {
          logUI("PIPELINE.ERROR", `requestId=${requestId} phase=done errorCode=PIPELINE_EVENT_TIMEOUT`);
          terminalLogged = true;
        }
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
        let timeoutHandle: ReturnType<typeof setTimeout> | null = null;
        const waitPromise = new Promise<void>((resolve) => {
          resolveWaiter = resolve;
          if (pending.length > 0) {
            resolveWaiter();
            resolveWaiter = null;
          }
        });
        const timeoutPromise = new Promise<void>((_, reject) => {
          timeoutHandle = setTimeout(() => reject(new Error("TIMEOUT")), remaining);
        });
        await Promise.race([waitPromise, timeoutPromise]);
        if (timeoutHandle !== null) {
          clearTimeout(timeoutHandle);
        }
      } catch {
        resolveWaiter = null;
        done = true;
        if (!terminalLogged) {
          logUI("PIPELINE.ERROR", `requestId=${requestId} phase=done errorCode=PIPELINE_EVENT_TIMEOUT`);
          terminalLogged = true;
        }
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
    if (requestId && !done && cancelOnExit) {
      await cancelTaskPipeline(requestId, "stream_exit").catch(() => undefined);
    }
    if (requestId && done) {
      untrackRequestId(requestId);
    }
    await Promise.resolve(unlisten()).catch(() => undefined);
    logUI("PIPELINE.STREAM_CLOSED", `requestId=${requestId ?? "pending"} done=${done}`);
  }
}

export async function* streamTaskPipelineByRequestId(
  requestId: string,
  options: TaskPipelineStreamOptions = {},
): AsyncGenerator<AiPipelineEvent> {
  trackRequestId(requestId);
  const pending: AiPipelineEvent[] = [];
  const timeoutMs = Math.max(
    options.timeoutMs ?? DEFAULT_EVENT_TIMEOUT_MS,
    MIN_EVENT_TIMEOUT_MS,
  );
  const cancelOnExit = options.cancelOnExit ?? true;
  let done = false;
  let terminalLogged = false;
  let firstEventSeen = false;
  let resolveWaiter: (() => void) | null = null;
  let lastEventTime = Date.now();
  const streamStartedAt = Date.now();

  const unlisten = await listen<unknown>(PIPELINE_EVENT_NAME, (event) => {
    const parsed = parsePipelineEvent(event.payload);
    if (!parsed || parsed.requestId !== requestId) {
      return;
    }
    firstEventSeen = true;
    pending.push(parsed);
    lastEventTime = Date.now();
    if (parsed.type === "done" || parsed.type === "error") {
      done = true;
      untrackRequestId(requestId);
      if (!terminalLogged) {
        const action = parsed.type === "done" ? "PIPELINE.DONE" : "PIPELINE.ERROR";
        const detailParts = [
          `requestId=${requestId}`,
          `phase=${parsed.phase}`,
          parsed.errorCode ? `errorCode=${parsed.errorCode}` : undefined,
        ].filter(Boolean);
        logUI(action, detailParts.join(" "));
        terminalLogged = true;
      }
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

      const activeTimeout = firstEventSeen
        ? timeoutMs
        : Math.min(timeoutMs, DEFAULT_FIRST_EVENT_TIMEOUT_MS);
      const waitBaseline = firstEventSeen ? lastEventTime : streamStartedAt;
      const remaining = activeTimeout - (Date.now() - waitBaseline);
      if (remaining <= 0) {
        done = true;
        if (!terminalLogged) {
          const code = firstEventSeen ? "PIPELINE_EVENT_TIMEOUT" : "PIPELINE_FIRST_EVENT_TIMEOUT";
          logUI("PIPELINE.ERROR", `requestId=${requestId} phase=done errorCode=${code}`);
          terminalLogged = true;
        }
        yield {
          requestId,
          phase: "done",
          type: "error",
          errorCode: firstEventSeen ? "PIPELINE_EVENT_TIMEOUT" : "PIPELINE_FIRST_EVENT_TIMEOUT",
          message: firstEventSeen
            ? "AI 响应超时，请检查网络连接"
            : "AI 启动后未收到事件，请重试",
          recoverable: true,
          meta: null,
        };
        break;
      }

      try {
        let timeoutHandle: ReturnType<typeof setTimeout> | null = null;
        const waitPromise = new Promise<void>((resolve) => {
          resolveWaiter = resolve;
          if (pending.length > 0) {
            resolveWaiter();
            resolveWaiter = null;
          }
        });
        const timeoutPromise = new Promise<void>((_, reject) => {
          timeoutHandle = setTimeout(() => reject(new Error("TIMEOUT")), remaining);
        });
        await Promise.race([waitPromise, timeoutPromise]);
        if (timeoutHandle !== null) {
          clearTimeout(timeoutHandle);
        }
      } catch {
        resolveWaiter = null;
        done = true;
        if (!terminalLogged) {
          const code = firstEventSeen ? "PIPELINE_EVENT_TIMEOUT" : "PIPELINE_FIRST_EVENT_TIMEOUT";
          logUI("PIPELINE.ERROR", `requestId=${requestId} phase=done errorCode=${code}`);
          terminalLogged = true;
        }
        yield {
          requestId,
          phase: "done",
          type: "error",
          errorCode: firstEventSeen ? "PIPELINE_EVENT_TIMEOUT" : "PIPELINE_FIRST_EVENT_TIMEOUT",
          message: firstEventSeen
            ? "AI 响应超时，请检查网络连接"
            : "AI 启动后未收到事件，请重试",
          recoverable: true,
          meta: null,
        };
        break;
      }
    }
  } finally {
    if (!done && cancelOnExit) {
      await cancelTaskPipeline(requestId, "stream_exit").catch(() => undefined);
    }
    if (done) {
      untrackRequestId(requestId);
    }
    await Promise.resolve(unlisten()).catch(() => undefined);
    logUI("PIPELINE.STREAM_CLOSED", `requestId=${requestId} done=${done}`);
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
