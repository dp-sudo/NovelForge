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
