import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("持久化策略契约：核心入口显式声明 persistMode 和 automationTier", async () => {
  const files = [
    "src/api/bookPipelineApi.ts",
    "src/api/blueprintApi.ts",
    "src/api/characterApi.ts",
    "src/api/worldApi.ts",
    "src/api/glossaryApi.ts",
    "src/api/plotApi.ts",
    "src/api/narrativeApi.ts",
    "src/pages/Timeline/TimelinePage.tsx",
    "src/pages/Relationships/RelationshipsPage.tsx",
  ];

  for (const rel of files) {
    const raw = await readRepoFile(rel);
    assert.match(raw, /persistMode:/, `${rel} 缺少 persistMode 声明`);
    assert.match(raw, /automationTier:/, `${rel} 缺少 automationTier 声明`);
  }
});

test("持久化策略契约：pipeline 输入类型声明显式策略并保留 autoPersist 兼容桥", async () => {
  const pipelineApi = await readRepoFile("src/api/pipelineApi.ts");
  const moduleApi = await readRepoFile("src/api/moduleAiApi.ts");

  assert.match(pipelineApi, /export type PersistMode = "none" \| "formal" \| "derived_review";/);
  assert.match(pipelineApi, /export type AutomationTier = "auto" \| "supervised" \| "confirm";/);
  assert.match(pipelineApi, /autoPersist\?: boolean;/);
  assert.match(pipelineApi, /persistMode\?: PersistMode;/);
  assert.match(pipelineApi, /automationTier\?: AutomationTier;/);

  assert.match(moduleApi, /export interface RunModuleAiTaskInput/);
  assert.match(moduleApi, /persistMode\?: PersistMode;/);
  assert.match(moduleApi, /automationTier\?: AutomationTier;/);
});

test("持久化策略契约：后端 run_ai_task_pipeline 入参接受新字段", async () => {
  const service = await readRepoFile("src-tauri/src/services/ai_pipeline_service.rs");
  const orchestrator = await readRepoFile("src-tauri/src/services/ai_pipeline/orchestrator.rs");
  const handlers = await readRepoFile("src-tauri/src/services/ai_pipeline/task_handlers.rs");

  assert.match(service, /pub auto_persist: bool,/);
  assert.match(service, /pub persist_mode: Option<String>,/);
  assert.match(service, /pub automation_tier: Option<String>,/);
  assert.match(orchestrator, /resolve_persist_mode\(/);
  assert.match(orchestrator, /should_persist_task_output\(/);
  assert.match(orchestrator, /persistMode/);
  assert.match(handlers, /persist_mode:/);
  assert.match(handlers, /should_persist_runtime_state_writes/);
});
