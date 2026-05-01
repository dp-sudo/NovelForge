import { invoke } from "@tauri-apps/api/core";
import type { AppErrorDto } from "../types/error.js";

type InvokeInput = Record<string, unknown> | undefined;

interface InflightInvokeRecord {
  callId: number;
  command: string;
  requestId?: string;
  startedAt: number;
}

const inflightInvokes = new Map<number, InflightInvokeRecord>();
const unloadCleanupHandlers = new Set<(reason: string) => void>();
let invokeSequence = 0;

declare global {
  interface Window {
    __NOVELFORGE_INFLIGHT_DIAGNOSTIC_BOUND__?: boolean;
  }
}

function parseTauriError(error: unknown): AppErrorDto {
  if (typeof error === "string") {
    try {
      return JSON.parse(error) as AppErrorDto;
    } catch {
      return {
        code: "UNKNOWN_ERROR",
        message: error,
        recoverable: false
      };
    }
  }

  if (typeof error === "object" && error !== null) {
    const maybeDto = error as Partial<AppErrorDto>;
    if (typeof maybeDto.code === "string" && typeof maybeDto.message === "string") {
      return {
        code: maybeDto.code,
        message: maybeDto.message,
        detail: maybeDto.detail,
        recoverable: Boolean(maybeDto.recoverable),
        suggestedAction: maybeDto.suggestedAction
      };
    }
  }

  return {
    code: "UNKNOWN_ERROR",
    message: "Unknown tauri error",
    recoverable: false
  };
}

function nowISO(): string {
  return new Date().toISOString();
}

function extractRequestId(input: InvokeInput): string | undefined {
  if (!input) {
    return undefined;
  }
  if (typeof input.requestId === "string" && input.requestId.trim()) {
    return input.requestId.trim();
  }
  const nested = input.input;
  if (nested && typeof nested === "object" && !Array.isArray(nested)) {
    const candidate = (nested as Record<string, unknown>).requestId;
    if (typeof candidate === "string" && candidate.trim()) {
      return candidate.trim();
    }
  }
  return undefined;
}

function formatRequestLabel(requestId: string | undefined): string {
  return requestId ? ` requestId=${requestId}` : "";
}

const SENSITIVE_LOG_KEY_RE = /(api[_-]?key|token|secret|password|authorization|license[_-]?key)/i;
const LARGE_TEXT_LOG_KEY_RE = /(content|chapter[_-]?content|selected[_-]?text|user[_-]?instruction|prompt)/i;

function sanitizeLogValue(value: unknown, key?: string, seen: WeakSet<object> = new WeakSet<object>()): unknown {
  if (typeof value === "string") {
    if (key && SENSITIVE_LOG_KEY_RE.test(key)) {
      return "[REDACTED]";
    }
    if (key && LARGE_TEXT_LOG_KEY_RE.test(key)) {
      return `[${value.length} chars]`;
    }
    return value;
  }

  if (!value || typeof value !== "object") {
    return value;
  }

  if (seen.has(value)) {
    return "[Circular]";
  }
  seen.add(value);

  if (Array.isArray(value)) {
    return value.map((item) => sanitizeLogValue(item, key, seen));
  }

  const safe: Record<string, unknown> = {};
  for (const [childKey, childValue] of Object.entries(value as Record<string, unknown>)) {
    safe[childKey] = sanitizeLogValue(childValue, childKey, seen);
  }
  return safe;
}

function logApiCall(callId: number, command: string, requestId: string | undefined, input: InvokeInput): void {
  const safe = input ? sanitizeLogValue(input) : {};
  const payload = safe && typeof safe === "object" && !Array.isArray(safe) ? (safe as Record<string, unknown>) : {};
  console.log(`[${nowISO()}] [API] >> #${callId} ${command}${formatRequestLabel(requestId)}`, Object.keys(payload).length ? payload : "");
}

function logApiResult(callId: number, command: string, requestId: string | undefined, elapsedMs: number): void {
  console.log(`[${nowISO()}] [API] << #${callId} ${command}${formatRequestLabel(requestId)} (${elapsedMs}ms)`);
}

function logApiError(callId: number, command: string, requestId: string | undefined, elapsedMs: number, error: AppErrorDto): void {
  console.warn(`[${nowISO()}] [API] !! #${callId} ${command}${formatRequestLabel(requestId)} (${elapsedMs}ms) FAILED: [${error.code}] ${error.message}`);
}

function logInflightSnapshot(reason: string): void {
  if (inflightInvokes.size === 0) {
    return;
  }
  const details = Array.from(inflightInvokes.values())
    .map((row) => {
      const ageMs = Math.max(0, Math.round(performance.now() - row.startedAt));
      return `#${row.callId} ${row.command}${formatRequestLabel(row.requestId)} age=${ageMs}ms`;
    })
    .join("; ");
  console.warn(`[${nowISO()}] [API] !! in-flight invoke snapshot | reason=${reason} | count=${inflightInvokes.size} | ${details}`);
}

function runUnloadCleanups(reason: string): void {
  for (const handler of unloadCleanupHandlers) {
    try {
      handler(reason);
    } catch (error) {
      console.warn(`[${nowISO()}] [API] !! unload cleanup failed (${reason})`, error);
    }
  }
}

export function registerUnloadCleanup(handler: (reason: string) => void): () => void {
  unloadCleanupHandlers.add(handler);
  return () => {
    unloadCleanupHandlers.delete(handler);
  };
}

function bindUnloadDiagnosticsOnce(): void {
  if (typeof window === "undefined" || window.__NOVELFORGE_INFLIGHT_DIAGNOSTIC_BOUND__) {
    return;
  }
  window.__NOVELFORGE_INFLIGHT_DIAGNOSTIC_BOUND__ = true;
  window.addEventListener("beforeunload", () => {
    runUnloadCleanups("beforeunload");
    logInflightSnapshot("beforeunload");
  });
}

bindUnloadDiagnosticsOnce();

const hotContext = (import.meta as ImportMeta & {
  hot?: {
    on: (event: string, cb: () => void) => void;
  };
}).hot;

if (hotContext) {
  hotContext.on("vite:beforeFullReload", () => {
    runUnloadCleanups("vite:beforeFullReload");
    logInflightSnapshot("vite:beforeFullReload");
  });
}

export async function invokeCommand<TOutput>(
  command: string,
  input?: Record<string, unknown>
): Promise<TOutput> {
  const callId = ++invokeSequence;
  const requestId = extractRequestId(input);
  const start = performance.now();
  inflightInvokes.set(callId, { callId, command, requestId, startedAt: start });
  logApiCall(callId, command, requestId, input);
  try {
    const result = await invoke<TOutput>(command, input);
    logApiResult(callId, command, requestId, Math.round(performance.now() - start));
    return result;
  } catch (error) {
    const parsed = parseTauriError(error);
    logApiError(callId, command, requestId, Math.round(performance.now() - start), parsed);
    throw parsed;
  } finally {
    inflightInvokes.delete(callId);
  }
}

export function logUI(action: string, detail?: string): void {
  console.log(`[${nowISO()}] [UI] ${action}${detail ? ` | ${detail}` : ""}`);
}
