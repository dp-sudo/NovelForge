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
    "src-tauri/src/infra/app_database.rs",
    "src-tauri/src/services/chapter_service.rs",
    "src-tauri/src/infra/credential_manager.rs",
    "src-tauri/src/services/git_service.rs",
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
    /Cannot open app database/,
    /Cannot read provider/,
    /Cannot list providers/,
    /Error reading providers/,
    /Cannot save provider/,
    /Cannot delete provider/,
    /Cannot load models/,
    /Error reading models/,
    /Cannot update model/,
    /Cannot insert model/,
    /Cannot insert refresh log/,
    /Cannot load task routes/,
    /Error reading task routes/,
    /Cannot save task route/,
    /Cannot delete task route/,
    /Cannot read app setting/,
    /Cannot save app setting/,
    /Cannot load refresh logs/,
    /Error reading refresh logs/,
    /Cannot read app database schema/,
    /Cannot migrate app database schema/,
    /Cannot create app data directory for database migration/,
    /Cannot migrate legacy app database/,
    /Cannot read chapter links/,
    /Cannot read chapter link row/,
    /Cannot access credential store/,
    /Cannot delete API key/,
    /Cannot create local secret directory/,
    /Cannot save API key/,
    /Cannot load API key/,
    /Provider id cannot be empty/,
    /Project path does not exist/,
    /Cannot parse git commit log/,
    /Cannot update \.gitignore/,
    /Git executable is not available on this machine/,
    /Failed to execute git command/,
    /Git command returned non-zero exit status/,
  ];

  for (const pattern of bannedVisibleEnglish) {
    assert.equal(pattern.test(content), false, `仍存在未中文化展示词: ${pattern}`);
  }
});
