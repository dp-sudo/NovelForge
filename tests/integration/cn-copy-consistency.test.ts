import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("设置与技能文案：关键展示词应统一为中文", async () => {
  const targets = [
    "src/pages/Settings/SettingsPage.tsx",
    "src/components/skills/SkillsManager.tsx",
    "src/components/skills/SkillCard.tsx",
    "src/components/skills/SkillDetail.tsx",
    "src/components/forms/ApiKeyInput.tsx",
    "src/pages/Editor/EditorPage.tsx",
    "src-tauri/src/services/ai_service.rs",
  ] as const;

  const content = (await Promise.all(targets.map((p) => readRepoFile(p)))).join("\n");
  const bannedVisibleEnglish = [
    /Provider 名称/,
    /请选择 Provider/,
    /Fallback Provider/,
    /Fallback 模型 ID/,
    /按任务类型指定 Provider \/ Model/,
    /API Key 已配置/,
    /未配置 API Key/,
    /"Workflow"/,
    /"Capability"/,
    /"Extractor"/,
    /"Review"/,
    /"Policy"/,
    /"Unclassified"/,
    /"Always On"/,
    /Base URL 不能为空/,
    /Base URL 必须/,
    /Base URL 格式不合法/,
    /No route configured for task type/,
    /No provider specified and no task type for route resolution/,
    /skill registry lock failed/,
    /All providers failed/,
    /Provider '\{\}'/,
  ];

  for (const pattern of bannedVisibleEnglish) {
    assert.equal(pattern.test(content), false, `仍存在未中文化展示词: ${pattern}`);
  }
});
