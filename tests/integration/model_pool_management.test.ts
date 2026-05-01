import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("模型池管理契约：前端存在独立 API 与设置面板组件", async () => {
  const api = await readRepoFile("src/api/modelPoolApi.ts");
  const panel = await readRepoFile("src/pages/Settings/components/ModelPoolPanel.tsx");
  const settingsPage = await readRepoFile("src/pages/Settings/SettingsPage.tsx");

  assert.match(api, /list_model_pools/);
  assert.match(api, /create_model_pool/);
  assert.match(api, /update_model_pool/);
  assert.match(api, /delete_model_pool/);
  assert.match(panel, /模型池管理/);
  assert.match(settingsPage, /ModelPoolPanel/);
  assert.match(settingsPage, /key: "modelPool"/);
});

test("模型池管理契约：后端暴露模型池 CRUD 命令并注册到 Tauri", async () => {
  const commands = await readRepoFile("src-tauri/src/commands/settings_commands.rs");
  const lib = await readRepoFile("src-tauri/src/lib.rs");

  assert.match(commands, /pub async fn list_model_pools/);
  assert.match(commands, /pub async fn create_model_pool/);
  assert.match(commands, /pub async fn update_model_pool/);
  assert.match(commands, /pub async fn delete_model_pool/);
  assert.match(lib, /settings_commands::list_model_pools/);
  assert.match(lib, /settings_commands::create_model_pool/);
  assert.match(lib, /settings_commands::update_model_pool/);
  assert.match(lib, /settings_commands::delete_model_pool/);
});

test("任务路由契约：路由面板支持模型池模式与直接兼容模式", async () => {
  const panel = await readRepoFile("src/pages/Settings/components/ModelRoutingPanel.tsx");
  const settingsPage = await readRepoFile("src/pages/Settings/SettingsPage.tsx");

  assert.match(panel, /路由模式/);
  assert.match(panel, /模型池路由/);
  assert.match(panel, /modelPoolId/);
  assert.match(panel, /fallbackModelPoolId/);
  assert.match(settingsPage, /selectedPool/);
  assert.match(settingsPage, /hasConfiguredModelPools/);
});
