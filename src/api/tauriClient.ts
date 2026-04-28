import { invoke } from "@tauri-apps/api/core";
import type { AppErrorDto } from "../types/error.js";

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

function logApiCall(command: string, input: Record<string, unknown> | undefined): void {
  const safe = input
    ? { ...input, apiKey: input.apiKey ? "[REDACTED]" : undefined, content: input.content ? `[${String(input.content).length} chars]` : undefined }
    : {};
  console.log(`[${nowISO()}] [API] >> ${command}`, Object.keys(safe).length ? safe : "");
}

function logApiResult(command: string, elapsedMs: number): void {
  console.log(`[${nowISO()}] [API] << ${command} (${elapsedMs}ms)`);
}

function logApiError(command: string, elapsedMs: number, error: AppErrorDto): void {
  console.warn(`[${nowISO()}] [API] !! ${command} (${elapsedMs}ms) FAILED: [${error.code}] ${error.message}`);
}

export async function invokeCommand<TOutput>(
  command: string,
  input?: Record<string, unknown>
): Promise<TOutput> {
  const start = performance.now();
  logApiCall(command, input);
  try {
    const result = await invoke<TOutput>(command, input);
    logApiResult(command, Math.round(performance.now() - start));
    return result;
  } catch (error) {
    const parsed = parseTauriError(error);
    logApiError(command, Math.round(performance.now() - start), parsed);
    throw parsed;
  }
}

export function logUI(action: string, detail?: string): void {
  console.log(`[${nowISO()}] [UI] ${action}${detail ? ` | ${detail}` : ""}`);
}
