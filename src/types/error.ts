export interface AppErrorDto {
  code: string;
  message: string;
  detail?: string;
  recoverable: boolean;
  suggestedAction?: string;
}
