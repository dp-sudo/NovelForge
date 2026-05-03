import { invokeCommand } from "./tauriClient.js";

// --- Types ---

export interface ConstitutionRule {
  id: string;
  projectId: string;
  ruleType: string;
  category: string;
  content: string;
  severity: string;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ConstitutionViolation {
  id: string;
  ruleId: string;
  chapterId: string | null;
  violationText: string;
  severity: string;
  detectedAt: string;
}

export interface CreateConstitutionRuleInput {
  ruleType: string;
  category: string;
  content: string;
  severity?: string;
}

export interface UpdateConstitutionRuleInput {
  content?: string;
  category?: string;
  severity?: string;
}

export interface ConstitutionValidationResult {
  totalRulesChecked: number;
  violationsFound: number;
  violations: Array<{
    ruleId: string;
    ruleContent: string;
    severity: string;
    violationText: string;
  }>;
}

// --- API calls ---

export async function listConstitutionRules(
  projectRoot: string
): Promise<ConstitutionRule[]> {
  return invokeCommand<ConstitutionRule[]>("list_constitution_rules", {
    projectRoot,
  });
}

export async function createConstitutionRule(
  projectRoot: string,
  input: CreateConstitutionRuleInput
): Promise<string> {
  return invokeCommand<string>("create_constitution_rule", {
    projectRoot,
    input,
  });
}

export async function updateConstitutionRule(
  projectRoot: string,
  ruleId: string,
  input: UpdateConstitutionRuleInput
): Promise<void> {
  return invokeCommand<void>("update_constitution_rule", {
    projectRoot,
    ruleId,
    input,
  });
}

export async function deleteConstitutionRule(
  projectRoot: string,
  ruleId: string
): Promise<void> {
  return invokeCommand<void>("delete_constitution_rule", {
    projectRoot,
    ruleId,
  });
}

export async function toggleConstitutionRule(
  projectRoot: string,
  ruleId: string,
  isActive: boolean
): Promise<void> {
  return invokeCommand<void>("toggle_constitution_rule", {
    projectRoot,
    ruleId,
    isActive,
  });
}

export async function validateTextAgainstConstitution(
  projectRoot: string,
  text: string,
  chapterId?: string
): Promise<ConstitutionValidationResult> {
  return invokeCommand<ConstitutionValidationResult>(
    "validate_text_against_constitution",
    {
      projectRoot,
      text,
      chapterId: chapterId ?? null,
      ruleType: null,
    }
  );
}

export async function listConstitutionViolations(
  projectRoot: string
): Promise<ConstitutionViolation[]> {
  return invokeCommand<ConstitutionViolation[]>(
    "list_constitution_violations",
    { projectRoot }
  );
}

export async function getConstitutionPromptText(
  projectRoot: string
): Promise<string> {
  return invokeCommand<string>("get_constitution_prompt_text", {
    projectRoot,
  });
}
