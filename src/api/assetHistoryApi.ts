import { invokeCommand } from "./tauriClient.js";

// ── Types ──

export interface AssetChangeRecord {
  id: string;
  projectId: string;
  assetType: string;
  assetId: string;
  assetName: string;
  changeType: "create" | "update" | "delete";
  changedBy: "user" | "ai" | "system";
  aiTaskType: string | null;
  aiRequestId: string | null;
  fieldName: string | null;
  oldValue: string | null;
  newValue: string | null;
  changeReason: string | null;
  createdAt: string;
}

export interface AssetHistoryFilter {
  assetType?: string;
  assetId?: string;
  changedBy?: "user" | "ai" | "system";
  startDate?: string;
  endDate?: string;
  limit?: number;
}

export interface AssetHistorySummary {
  totalChanges: number;
  changesByType: Record<string, number>;
  changesBySource: Record<string, number>;
  recentChanges: AssetChangeRecord[];
}

// ── API Calls ──

/**
 * 获取资产变更历史记录
 */
export async function listAssetChangeHistory(
  projectRoot: string,
  filter?: AssetHistoryFilter
): Promise<AssetChangeRecord[]> {
  return invokeCommand<AssetChangeRecord[]>("list_asset_change_history", {
    projectRoot,
    filter: filter ?? null,
  });
}

/**
 * 获取特定资产的变更历史
 */
export async function getAssetHistory(
  projectRoot: string,
  assetType: string,
  assetId: string
): Promise<AssetChangeRecord[]> {
  return invokeCommand<AssetChangeRecord[]>("get_asset_history", {
    projectRoot,
    assetType,
    assetId,
  });
}

/**
 * 获取资产变更历史摘要统计
 */
export async function getAssetHistorySummary(
  projectRoot: string
): Promise<AssetHistorySummary> {
  return invokeCommand<AssetHistorySummary>("get_asset_history_summary", {
    projectRoot,
  });
}

/**
 * 记录资产变更（通常由后端自动调用，但也可手动记录）
 */
export async function recordAssetChange(
  projectRoot: string,
  change: {
    assetType: string;
    assetId: string;
    assetName: string;
    changeType: "create" | "update" | "delete";
    changedBy: "user" | "ai" | "system";
    fieldName?: string;
    oldValue?: string;
    newValue?: string;
    changeReason?: string;
    aiTaskType?: string;
    aiRequestId?: string;
  }
): Promise<string> {
  return invokeCommand<string>("record_asset_change", {
    projectRoot,
    change,
  });
}

/**
 * 比较资产的两个版本
 */
export async function compareAssetVersions(
  projectRoot: string,
  assetType: string,
  assetId: string,
  fromTimestamp: string,
  toTimestamp: string
): Promise<{
  changes: Array<{
    fieldName: string;
    oldValue: string | null;
    newValue: string | null;
  }>;
}> {
  return invokeCommand("compare_asset_versions", {
    projectRoot,
    assetType,
    assetId,
    fromTimestamp,
    toTimestamp,
  });
}
