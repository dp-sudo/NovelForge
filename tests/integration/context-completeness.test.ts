import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("上下文完整性契约：章节关键任务强制 deep 最小深度", async () => {
  const compiler = await readRepoFile("src-tauri/src/services/ai_pipeline/continuity_pack.rs");
  assert.match(compiler, /fn required_min_depth/);
  assert.match(compiler, /"chapter\.draft"/);
  assert.match(compiler, /"chapter\.continue"/);
  assert.match(compiler, /"chapter\.rewrite"/);
  assert.match(compiler, /Some\(ContinuityPackDepth::Deep\)/);
});

test("上下文完整性契约：编排器发出 context_incomplete warning 事件", async () => {
  const orchestrator = await readRepoFile("src-tauri/src/services/ai_pipeline/orchestrator.rs");
  assert.match(orchestrator, /assess_continuity_pack_completeness/);
  assert.match(orchestrator, /event_type: "warning"/);
  assert.match(orchestrator, /PIPELINE_CONTEXT_INCOMPLETE/);
  assert.match(orchestrator, /"context_incomplete"/);
  assert.match(orchestrator, /"missingLayers"/);
});

test("上下文完整性契约：Prompt 阶段显式记录请求深度与生效深度", async () => {
  const orchestrator = await readRepoFile("src-tauri/src/services/ai_pipeline/orchestrator.rs");
  assert.match(orchestrator, /requestedContinuityDepth/);
  assert.match(orchestrator, /continuityDepth/);
  assert.match(orchestrator, /contextComplete/);
});

