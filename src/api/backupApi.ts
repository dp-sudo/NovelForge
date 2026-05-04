import { invokeCommand } from "./tauriClient.js";

export interface BackupResult {
  filePath: string;
  fileSize: number;
  createdAt: string;
}

export interface RestoreResult {
  projectRoot: string;
  filesRestored: number;
}

export async function createBackup(projectRoot: string): Promise<BackupResult> {
  return invokeCommand<BackupResult>("create_backup", { projectRoot });
}

export async function listBackups(projectRoot: string): Promise<BackupResult[]> {
  return invokeCommand<BackupResult[]>("list_backups", { projectRoot });
}

export async function restoreBackup(projectRoot: string, backupPath: string): Promise<RestoreResult> {
  return invokeCommand<RestoreResult>("restore_backup", { projectRoot, backupPath });
}
