import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("路由策略推荐契约：策略模板领域模型与项目持久化字段存在", async () => {
  const domain = await readRepoFile("src-tauri/src/domain/routing_strategy.rs");
  const migration = await readRepoFile("src-tauri/migrations/project/0012_project_routing_strategy.sql");
  assert.match(domain, /pub struct RoutingStrategyTemplate/);
  assert.match(domain, /pub enum ProjectStage/);
  assert.match(domain, /pub enum RiskLevel/);
  assert.match(domain, /recommended_pools: HashMap<String, String>/);
  assert.match(migration, /ALTER TABLE projects ADD COLUMN routing_strategy_id TEXT/i);
});

test("路由策略推荐契约：后端提供推荐/应用/读取命令并注册", async () => {
  const aiService = await readRepoFile("src-tauri/src/services/ai_service.rs");
  const commands = await readRepoFile("src-tauri/src/commands/settings_commands.rs");
  const lib = await readRepoFile("src-tauri/src/lib.rs");

  assert.match(aiService, /pub fn recommend_routing_strategy/);
  assert.match(aiService, /pub fn apply_routing_strategy_template/);
  assert.match(aiService, /pub fn get_project_routing_strategy_id/);
  assert.match(aiService, /built_in_routing_strategy_templates/);
  assert.match(aiService, /task_risk_level/);

  assert.match(commands, /pub async fn recommend_routing_strategy/);
  assert.match(commands, /pub async fn apply_routing_strategy_template/);
  assert.match(commands, /pub async fn get_project_routing_strategy/);
  assert.match(lib, /settings_commands::recommend_routing_strategy/);
  assert.match(lib, /settings_commands::apply_routing_strategy_template/);
  assert.match(lib, /settings_commands::get_project_routing_strategy/);
});

test("路由策略推荐契约：设置页支持推荐策略加载、应用与手动覆盖共存", async () => {
  const api = await readRepoFile("src/api/settingsApi.ts");
  const panel = await readRepoFile("src/pages/Settings/components/ModelRoutingPanel.tsx");
  const page = await readRepoFile("src/pages/Settings/SettingsPage.tsx");

  assert.match(api, /recommend_routing_strategy/);
  assert.match(api, /apply_routing_strategy_template/);
  assert.match(api, /get_project_routing_strategy/);

  assert.match(panel, /推荐策略/);
  assert.match(panel, /onRecommendRoutingStrategy/);
  assert.match(panel, /onApplyRoutingStrategy/);
  assert.match(panel, /按项目阶段与任务风险推荐池级路由模板/);

  assert.match(page, /handleRecommendRoutingStrategy/);
  assert.match(page, /handleApplyRoutingStrategy/);
  assert.match(page, /routingStrategyTemplates/);
});
