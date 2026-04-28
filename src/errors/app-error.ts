export interface AppErrorDto {
  code: string;
  message: string;
  detail?: string;
  recoverable: boolean;
  suggestedAction?: string;
}

export class AppError extends Error {
  public readonly code: string;
  public readonly detail?: string;
  public readonly recoverable: boolean;
  public readonly suggestedAction?: string;

  public constructor(input: AppErrorDto) {
    super(input.message);
    this.name = "AppError";
    this.code = input.code;
    this.detail = input.detail;
    this.recoverable = input.recoverable;
    this.suggestedAction = input.suggestedAction;
  }

  public toDto(): AppErrorDto {
    return {
      code: this.code,
      message: this.message,
      detail: this.detail,
      recoverable: this.recoverable,
      suggestedAction: this.suggestedAction
    };
  }
}

export function ensureAppError(error: unknown, fallbackCode = "UNKNOWN_ERROR"): AppError {
  if (error instanceof AppError) {
    return error;
  }

  const message = error instanceof Error ? error.message : String(error);
  return new AppError({
    code: fallbackCode,
    message,
    recoverable: false
  });
}
