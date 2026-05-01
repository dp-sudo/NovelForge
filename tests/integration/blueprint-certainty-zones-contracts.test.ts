import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("确定性分区契约：blueprintApi 暴露 certaintyZones DTO 参数", async () => {
  const api = await readRepoFile("src/api/blueprintApi.ts");
  assert.match(api, /certaintyZones/);
  assert.match(api, /save_blueprint_step/);
});

test("确定性分区契约：BlueprintPage 提供冻结区\/承诺区\/探索区编辑入口", async () => {
  const page = await readRepoFile("src/pages/Blueprint/BlueprintPage.tsx");
  assert.match(page, /冻结区/);
  assert.match(page, /承诺区/);
  assert.match(page, /探索区/);
});

test("确定性分区契约：orchestrator 具备 DTO 优先并兼容文本回退", async () => {
  const orchestrator = await readRepoFile("src-tauri/src/services/ai_pipeline/orchestrator.rs");
  assert.match(orchestrator, /certainty_zones/);
  assert.match(orchestrator, /extract_certainty_zones_from_content/);
});
