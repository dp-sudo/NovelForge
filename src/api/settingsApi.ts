import { invokeCommand } from "./tauriClient.js";
import type { LlmProviderConfig, RefreshResult, ModelRecord, RefreshLog, TaskRoute } from "../types/ai.js";

export type { LlmProviderConfig, RefreshResult, ModelRecord, RefreshLog, TaskRoute } from "../types/ai.js";

export interface EditorSettingsData {
  fontSize: number;
  lineHeight: number;
  autosaveInterval: number;
  narrativePov: string;
}

export interface GitRepositoryStatus {
  initialized: boolean;
  branch: string;
  hasChanges: boolean;
}

export interface GitCommitRecord {
  commitId: string;
  summary: string;
  committedAt: string;
}

export interface GitSnapshotResult {
  noChanges: boolean;
  commit?: GitCommitRecord;
}

export interface LicenseStatus {
  activated: boolean;
  tier: string;
  licenseKeyMasked?: string;
  activatedAt?: string;
  expiresAt?: string;
  offlineAvailable: boolean;
}

export interface AppUpdateInfo {
  available: boolean;
  currentVersion: string;
  targetVersion?: string;
  body?: string;
  date?: string;
}

// ── Provider config ──

export async function listProviders(): Promise<LlmProviderConfig[]> {
  return invokeCommand<LlmProviderConfig[]>("list_providers");
}

export async function saveProvider(
  config: LlmProviderConfig,
  apiKey?: string
): Promise<LlmProviderConfig> {
  return invokeCommand<LlmProviderConfig>("save_provider", {
    config,
    apiKey: apiKey || null,
  });
}

export async function deleteProvider(providerId: string): Promise<void> {
  await invokeCommand<void>("delete_provider", { providerId });
}

export async function testProviderConnection(providerId: string): Promise<string> {
  return invokeCommand<string>("test_provider_connection", { providerId });
}

// ── Model registry ──

export async function refreshProviderModels(providerId: string): Promise<RefreshResult> {
  return invokeCommand<RefreshResult>("refresh_provider_models", { providerId });
}

export async function getProviderModels(providerId: string): Promise<ModelRecord[]> {
  return invokeCommand<ModelRecord[]>("get_provider_models", { providerId });
}

export async function getRefreshLogs(providerId: string): Promise<RefreshLog[]> {
  return invokeCommand<RefreshLog[]>("get_refresh_logs", { providerId });
}

// ── Task routes ──

export async function listTaskRoutes(): Promise<TaskRoute[]> {
  return invokeCommand<TaskRoute[]>("list_task_routes");
}

export async function saveTaskRoute(route: TaskRoute): Promise<TaskRoute> {
  return invokeCommand<TaskRoute>("save_task_route", { route });
}

export async function deleteTaskRoute(routeId: string): Promise<void> {
  await invokeCommand<void>("delete_task_route", { routeId });
}

// ── Remote registry ──

export async function checkRemoteRegistry(url: string): Promise<{ currentVersion: string; remoteVersion: string; hasUpdate: boolean; checkedAt: string }> {
  return invokeCommand("check_remote_registry", { url });
}

export async function applyRegistryUpdate(url: string): Promise<{ added: number; updated: number; version: string; appliedAt: string }> {
  return invokeCommand("apply_registry_update", { url });
}

// ── Editor settings ──

export async function loadEditorSettings(): Promise<EditorSettingsData> {
  return invokeCommand<EditorSettingsData>("load_editor_settings");
}

export async function saveEditorSettings(input: EditorSettingsData): Promise<void> {
  await invokeCommand<void>("save_editor_settings", { settings: input });
}

// —— Git integration ——

export async function initProjectRepository(projectRoot: string): Promise<GitRepositoryStatus> {
  return invokeCommand<GitRepositoryStatus>("init_project_repository", { projectRoot });
}

export async function getProjectRepositoryStatus(projectRoot: string): Promise<GitRepositoryStatus> {
  return invokeCommand<GitRepositoryStatus>("get_project_repository_status", { projectRoot });
}

export async function commitProjectSnapshot(projectRoot: string, message?: string): Promise<GitSnapshotResult> {
  return invokeCommand<GitSnapshotResult>("commit_project_snapshot", {
    input: { projectRoot, message: message || null },
  });
}

export async function listProjectHistory(projectRoot: string, limit?: number): Promise<GitCommitRecord[]> {
  return invokeCommand<GitCommitRecord[]>("list_project_history", { projectRoot, limit });
}

// —— License ——

export async function getLicenseStatus(): Promise<LicenseStatus> {
  return invokeCommand<LicenseStatus>("get_license_status");
}

export async function activateLicense(licenseKey: string): Promise<LicenseStatus> {
  return invokeCommand<LicenseStatus>("activate_license", { licenseKey });
}

// —— Updater ——

export async function checkAppUpdate(): Promise<AppUpdateInfo> {
  return invokeCommand<AppUpdateInfo>("check_app_update");
}

export async function installAppUpdate(): Promise<AppUpdateInfo> {
  return invokeCommand<AppUpdateInfo>("install_app_update");
}
