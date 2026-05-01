import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("AI 策略契约：settingsApi 暴露项目级策略读写函数", async () => {
  const api = await readRepoFile("src/api/settingsApi.ts");
  assert.match(api, /export async function saveAiStrategyProfile/);
  assert.match(api, /export async function getAiStrategyProfile/);
  assert.match(api, /export async function saveProjectAiStrategy/);
  assert.match(api, /export async function getProjectAiStrategy/);
  assert.match(api, /"save_ai_strategy_profile"/);
  assert.match(api, /"get_ai_strategy_profile"/);
});

test("AI 策略契约：project commands 定义输入结构并暴露命令", async () => {
  const commands = await readRepoFile("src-tauri/src/commands/project_commands.rs");
  assert.match(commands, /pub struct SaveAiStrategyProfileInput/);
  assert.match(commands, /pub struct GetAiStrategyProfileInput/);
  assert.match(commands, /pub async fn save_ai_strategy_profile/);
  assert.match(commands, /pub async fn get_ai_strategy_profile/);
});

test("AI 策略契约：lib.rs 注册新命令", async () => {
  const libRs = await readRepoFile("src-tauri/src/lib.rs");
  assert.match(libRs, /commands::project_commands::save_ai_strategy_profile/);
  assert.match(libRs, /commands::project_commands::get_ai_strategy_profile/);
});

test("AI 策略契约：项目迁移包含 ai_strategy_profile 字段", async () => {
  const migration = await readRepoFile(
    "src-tauri/migrations/project/0004_ai_strategy_profile.sql",
  );
  assert.match(migration, /ALTER TABLE projects ADD COLUMN ai_strategy_profile TEXT;/);
});

test("AI 策略设置契约：SettingsPage 暴露 AI 策略页签与面板入口", async () => {
  const page = await readRepoFile("src/pages/Settings/SettingsPage.tsx");
  assert.match(page, /"aiStrategy"/);
  assert.match(page, /AiStrategyPanel/);
});

test("AI 策略设置契约：AiStrategyPanel 包含关键配置区与保存动作", async () => {
  const panel = await readRepoFile("src/components/settings/AiStrategyPanel.tsx");
  assert.match(panel, /AI 策略配置/);
  assert.match(panel, /默认工作流栈/);
  assert.match(panel, /审查严格度/);
  assert.match(panel, /const level = index \+ 1;/);
  assert.match(panel, /onClick=\{\(\) => onChange\(level\)\}/);
  assert.match(panel, /默认能力包/);
  assert.match(panel, /自动持久化策略/);
  assert.match(panel, /保存 AI 策略/);
});

test("AI 策略运行时契约：orchestrator 消费默认工作流栈、能力包与场景编排", async () => {
  const orchestrator = await readRepoFile("src-tauri/src/services/ai_pipeline/orchestrator.rs");
  assert.match(orchestrator, /default_workflow_stack/);
  assert.match(orchestrator, /default_capability_bundles/);
  assert.match(orchestrator, /infer_scene_bundle_ids/);
  assert.match(orchestrator, /CertaintyZones/);
});
