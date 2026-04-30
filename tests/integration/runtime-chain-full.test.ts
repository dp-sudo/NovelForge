import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

function extractInvokedCommands(apiContent: string): Set<string> {
  const commands = new Set<string>();
  const pattern = /invokeCommand(?:<[^>]+>)?\("([a-z0-9_]+)"/g;
  for (const match of apiContent.matchAll(pattern)) {
    commands.add(match[1]);
  }
  return commands;
}

test("问题1回归：integration 目录不再依赖 dev-engine fixtures", async () => {
  const integrationDir = path.join(REPO_ROOT, "tests", "integration");
  const files = (await fs.readdir(integrationDir)).filter((name) => name.endsWith(".test.ts"));
  for (const file of files) {
    const content = await fs.readFile(path.join(integrationDir, file), "utf-8");
    const hasFixtureImport = /from\s+["']\.\.\/dev-engine-fixtures\//.test(content);
    assert.equal(
      hasFixtureImport,
      false,
      `integration 测试仍依赖 fixtures: ${file}`,
    );
  }
});

test("问题1回归：AI 模块 API 仅走 run_ai_task_pipeline / cancel_ai_task_pipeline", async () => {
  const apiFiles = [
    "src/api/blueprintApi.ts",
    "src/api/characterApi.ts",
    "src/api/worldApi.ts",
    "src/api/plotApi.ts",
    "src/api/glossaryApi.ts",
    "src/api/narrativeApi.ts",
    "src/api/consistencyApi.ts",
  ];

  for (const file of apiFiles) {
    const content = await readRepoFile(file);
    assert.equal(
      /invokeCommand<string>\("ai_generate_|invokeCommand<string>\("ai_scan_consistency"|invokeCommand<string>\("generate_blueprint_suggestion"/.test(content),
      false,
      `仍存在重复 Rust AI 命令入口调用: ${file}`,
    );
    assert.match(content, /runModuleAiTask\(/, `未接入统一 AI pipeline 封装: ${file}`);
  }
});

test("问题1回归：闭环关键 API 调用命令在 Tauri 侧已注册", async () => {
  const apiFiles = [
    "src/api/projectApi.ts",
    "src/api/chapterApi.ts",
    "src/api/blueprintApi.ts",
    "src/api/characterApi.ts",
    "src/api/worldApi.ts",
    "src/api/plotApi.ts",
    "src/api/glossaryApi.ts",
    "src/api/narrativeApi.ts",
    "src/api/consistencyApi.ts",
    "src/api/exportApi.ts",
    "src/api/pipelineApi.ts",
  ];
  const invoked = new Set<string>();
  for (const file of apiFiles) {
    const content = await readRepoFile(file);
    for (const command of extractInvokedCommands(content)) {
      invoked.add(command);
    }
  }

  const libRs = await readRepoFile("src-tauri/src/lib.rs");
  const missing = [...invoked].filter((command) => !libRs.includes(`::${command}`)).sort();
  assert.deepEqual(missing, [], `关键 API 存在未注册命令: ${missing.join(", ")}`);
});

test("问题5回归：晋升来源轨迹链路已接入迁移、持久化与上下文查询", async () => {
  const migration = await readRepoFile("src-tauri/migrations/project/0005_entity_provenance.sql");
  const migrator = await readRepoFile("src-tauri/src/infra/migrator.rs");
  const handlers = await readRepoFile("src-tauri/src/services/ai_pipeline/task_handlers.rs");
  const contextService = await readRepoFile("src-tauri/src/services/context_service.rs");

  assert.match(migration, /CREATE TABLE IF NOT EXISTS entity_provenance/);
  assert.match(migration, /source_kind TEXT NOT NULL/);
  assert.match(migration, /source_ref TEXT/);
  assert.match(migrator, /0005_entity_provenance/);
  assert.match(handlers, /INSERT INTO entity_provenance/);
  assert.match(handlers, /manual_promotion/);
  assert.match(handlers, /auto_promotion/);
  assert.match(contextService, /entity_provenance/);
  assert.match(contextService, /source_kind/);
});

test("问题9契约：蓝图与上下文 API 暴露窗口规划与摘要回报接口", async () => {
  const blueprintApi = await readRepoFile("src/api/blueprintApi.ts");
  const contextApi = await readRepoFile("src/api/contextApi.ts");

  assert.match(blueprintApi, /getWindowPlanningData/);
  assert.match(blueprintApi, /windowPlanningHorizon/);
  assert.match(contextApi, /getSummaryFeedback/);
  assert.match(contextApi, /driftWarnings/);
  assert.match(contextApi, /assetPromotionCount/);
  assert.match(contextApi, /stateUpdateCount/);
});

test("问题9契约：审阅层页面明确标注非一级事实源", async () => {
  const timeline = await readRepoFile("src/pages/Timeline/TimelinePage.tsx");
  const relationships = await readRepoFile("src/pages/Relationships/RelationshipsPage.tsx");

  assert.match(timeline, /不是一级事实源/);
  assert.match(relationships, /不是一级事实源/);
});
