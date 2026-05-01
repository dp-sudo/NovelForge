import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("结构化状态契约：StoryStateService 定义 taxonomy 并在读取链路补齐", async () => {
  const service = await readRepoFile("src-tauri/src/services/story_state_service.rs");
  assert.match(service, /enum StoryStateTaxonomy/);
  assert.match(service, /Emotion/);
  assert.match(service, /SceneEnvironment/);
  assert.match(service, /RelationshipTemperature/);
  assert.match(service, /enrich_payload_with_taxonomy/);
});

test("结构化状态契约：runtime writer 写入 category\/value 结构", async () => {
  const writer = await readRepoFile("src-tauri/src/services/ai_pipeline/runtime_state_writer.rs");
  assert.match(writer, /\"schemaVersion\": 1/);
  assert.match(writer, /\"category\": taxonomy\.as_str\(\)/);
  assert.match(writer, /\"value\": structured_value/);
  assert.match(writer, /character\.emotion/);
  assert.match(writer, /relationship\.temperature/);
});

test("结构化状态契约：章节与抽取技能声明情绪\/场景\/关系温度写回", async () => {
  const chapterDraft = await readRepoFile("resources/builtin-skills/chapter.draft.md");
  const chapterContinue = await readRepoFile("resources/builtin-skills/chapter.continue.md");
  const contextCollect = await readRepoFile("resources/builtin-skills/context.collect.md");
  assert.match(chapterDraft, /character\.emotion/);
  assert.match(chapterDraft, /scene\.environment/);
  assert.match(chapterDraft, /relationship\.temperature/);
  assert.match(chapterContinue, /character\.emotion/);
  assert.match(contextCollect, /relationship\.temperature/);
});

