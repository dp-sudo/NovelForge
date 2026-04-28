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

export async function invokeCommand<TOutput>(
  command: string,
  input?: Record<string, unknown>
): Promise<TOutput> {
  try {
    return await invoke<TOutput>(command, input);
  } catch (error) {
    throw parseTauriError(error);
  }
}
