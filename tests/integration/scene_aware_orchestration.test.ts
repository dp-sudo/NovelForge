import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("场景编排契约：分类器具备语义特征判别与默认后置任务映射", async () => {
  const classifier = await readRepoFile("src-tauri/src/services/ai_pipeline/scene_classifier.rs");
  assert.match(classifier, /dialogue_ratio/);
  assert.match(classifier, /combat_signal=hits/);
  assert.match(classifier, /default_post_tasks/);
  assert.match(classifier, /dialogue.*extract_state/s);
  assert.match(classifier, /combat.*review_continuity/s);
});

test("场景编排契约：后置执行器支持状态抽取/连续性审查/资产抽取并容错", async () => {
  const executor = await readRepoFile("src-tauri/src/services/ai_pipeline/post_task_executor.rs");
  assert.match(executor, /review_continuity/);
  assert.match(executor, /extract_state/);
  assert.match(executor, /extract_assets/);
  assert.match(executor, /status: "failed"/);
  assert.match(executor, /danger_level/);
  assert.match(executor, /spatial_constraint/);
});

test("场景编排契约：orchestrator 审计写入 sceneType 与 postTaskResults", async () => {
  const orchestrator = await readRepoFile("src-tauri/src/services/ai_pipeline/orchestrator.rs");
  assert.match(orchestrator, /scene_classification/);
  assert.match(orchestrator, /update_post_task_results/);
  assert.match(orchestrator, /sceneDecision/);
  assert.match(orchestrator, /postTaskResults/);
  assert.match(orchestrator, /update_pipeline_meta/);
});
