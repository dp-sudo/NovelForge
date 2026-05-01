import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("状态 taxonomy 契约：后端分类覆盖动作/外观/信息边界与场景高阶状态", async () => {
  const storyState = await readRepoFile("src-tauri/src/services/story_state_service.rs");
  const runtimeWriter = await readRepoFile("src-tauri/src/services/ai_pipeline/runtime_state_writer.rs");

  assert.match(storyState, /CharacterAction/);
  assert.match(storyState, /CharacterAppearance/);
  assert.match(storyState, /CharacterKnowledge/);
  assert.match(storyState, /SceneDangerLevel/);
  assert.match(storyState, /SceneSpatialConstraint/);
  assert.match(runtimeWriter, /StoryStateTaxonomy::CharacterAction/);
  assert.match(runtimeWriter, /StoryStateTaxonomy::CharacterAppearance/);
  assert.match(runtimeWriter, /StoryStateTaxonomy::CharacterKnowledge/);
  assert.match(runtimeWriter, /StoryStateTaxonomy::SceneDangerLevel/);
  assert.match(runtimeWriter, /StoryStateTaxonomy::SceneSpatialConstraint/);
});

test("状态 taxonomy 契约：内置抽取器声明新增 stateWrites", async () => {
  const action = await readRepoFile("resources/builtin-skills/extractor-character-action.md");
  const appearance = await readRepoFile("resources/builtin-skills/extractor-character-appearance.md");
  const knowledge = await readRepoFile("resources/builtin-skills/extractor-character-knowledge.md");
  const chapterDraft = await readRepoFile("resources/builtin-skills/chapter.draft.md");
  const chapterContinue = await readRepoFile("resources/builtin-skills/chapter.continue.md");

  assert.match(action, /stateWrites:\s*\[character\.action\]/);
  assert.match(appearance, /stateWrites:\s*\[character\.appearance\]/);
  assert.match(knowledge, /stateWrites:\s*\[character\.knowledge\]/);
  assert.match(chapterDraft, /scene\.danger_level/);
  assert.match(chapterDraft, /scene\.spatial_constraint/);
  assert.match(chapterContinue, /character\.action/);
});

test("状态 taxonomy 契约：编辑器上下文面板可见状态 JSON 预览", async () => {
  const panel = await readRepoFile("src/pages/Editor/components/EditorContextPanel.tsx");
  assert.match(panel, /JSON\.stringify\(item\.payload\)/);
  assert.match(panel, /最新状态:/);
});
