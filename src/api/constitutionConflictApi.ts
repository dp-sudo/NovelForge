import { invokeCommand } from "./tauriClient.js";
import type { ConstitutionRule } from "./constitutionApi.js";

// ── Types ──

export interface ConstitutionConflict {
  id: string;
  projectId: string;
  ruleIdA: string;
  ruleIdB: string;
  conflictType: "direct_contradiction" | "logical_inconsistency" | "temporal_conflict";
  severity: "low" | "medium" | "high";
  explanation: string;
  aiDetected: boolean;
  resolutionStatus: "open" | "acknowledged" | "resolved" | "false_positive";
  resolutionNote: string | null;
  detectedAt: string;
  resolvedAt: string | null;
}

export interface ConstitutionConflictWithRules extends ConstitutionConflict {
  ruleA: ConstitutionRule;
  ruleB: ConstitutionRule;
}

export interface ConflictDetectionResult {
  totalRulesChecked: number;
  conflictsFound: number;
  conflicts: ConstitutionConflictWithRules[];
}

export interface RuleTag {
  id: string;
  ruleId: string;
  tagType: "entity" | "temporal" | "constraint" | "theme";
  tagValue: string;
  createdAt: string;
}

// ── API Calls ──

/**
 * 列出所有宪法规则冲突
 */
export async function listConstitutionConflicts(
  projectRoot: string,
  status?: "open" | "acknowledged" | "resolved" | "false_positive"
): Promise<ConstitutionConflictWithRules[]> {
  return invokeCommand<ConstitutionConflictWithRules[]>(
    "list_constitution_conflicts",
    {
      projectRoot,
      status: status ?? null,
    }
  );
}

/**
 * 运行宪法冲突检测（AI 驱动）
 */
export async function detectConstitutionConflicts(
  projectRoot: string
): Promise<ConflictDetectionResult> {
  return invokeCommand<ConflictDetectionResult>(
    "detect_constitution_conflicts",
    {
      projectRoot,
    }
  );
}

/**
 * 更新冲突解决状态
 */
export async function updateConflictResolution(
  projectRoot: string,
  conflictId: string,
  status: "acknowledged" | "resolved" | "false_positive",
  note?: string
): Promise<void> {
  return invokeCommand<void>("update_conflict_resolution", {
    projectRoot,
    conflictId,
    status,
    note: note ?? null,
  });
}

/**
 * 删除冲突记录
 */
export async function deleteConflict(
  projectRoot: string,
  conflictId: string
): Promise<void> {
  return invokeCommand<void>("delete_constitution_conflict", {
    projectRoot,
    conflictId,
  });
}

/**
 * 获取规则的标签
 */
export async function getRuleTags(
  projectRoot: string,
  ruleId: string
): Promise<RuleTag[]> {
  return invokeCommand<RuleTag[]>("get_rule_tags", {
    projectRoot,
    ruleId,
  });
}

/**
 * 为规则添加标签
 */
export async function addRuleTag(
  projectRoot: string,
  ruleId: string,
  tagType: "entity" | "temporal" | "constraint" | "theme",
  tagValue: string
): Promise<string> {
  return invokeCommand<string>("add_rule_tag", {
    projectRoot,
    ruleId,
    tagType,
    tagValue,
  });
}

/**
 * 删除规则标签
 */
export async function deleteRuleTag(
  projectRoot: string,
  tagId: string
): Promise<void> {
  return invokeCommand<void>("delete_rule_tag", {
    projectRoot,
    tagId,
  });
}

/**
 * 获取冲突统计摘要
 */
export async function getConflictSummary(
  projectRoot: string
): Promise<{
  totalConflicts: number;
  openConflicts: number;
  conflictsBySeverity: Record<string, number>;
  conflictsByType: Record<string, number>;
}> {
  return invokeCommand("get_conflict_summary", {
    projectRoot,
  });
}
