import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("技能契约：前后端暴露 manifest 元数据更新输入", async () => {
  const api = await readRepoFile("src/api/skillsApi.ts");
  const cmd = await readRepoFile("src-tauri/src/commands/skill_commands.rs");
  assert.match(api, /export interface UpdateSkillInput/);
  assert.match(api, /updateSkill\(input: UpdateSkillInput\)/);
  assert.match(cmd, /pub struct UpdateSkillInput/);
  assert.match(cmd, /pub manifest: Option<SkillManifestPatch>/);
});

test("技能契约：SkillManifest 扩展字段在后端结构体中存在", async () => {
  const registry = await readRepoFile("src-tauri/src/services/skill_registry.rs");
  assert.match(registry, /pub skill_class: Option<String>/);
  assert.match(registry, /pub bundle_ids: Vec<String>/);
  assert.match(registry, /pub always_on: bool/);
  assert.match(registry, /pub trigger_conditions: Vec<String>/);
  assert.match(registry, /pub required_contexts: Vec<String>/);
  assert.match(registry, /pub state_writes: Vec<String>/);
  assert.match(registry, /pub automation_tier: Option<String>/);
  assert.match(registry, /pub scene_tags: Vec<String>/);
  assert.match(registry, /pub affects_layers: Vec<String>/);
});

test("技能契约：设置 UI 可编辑分类、绑定与触发条件", async () => {
  const detail = await readRepoFile("src/components/skills/SkillDetail.tsx");
  assert.match(detail, /技能分类/);
  assert.match(detail, /bundleIds/);
  assert.match(detail, /triggerConditions/);
  assert.match(detail, /Always On/);
  assert.match(detail, /sceneTags/);
});

test("技能契约：技能管理支持按 skillClass 展示和筛选", async () => {
  const manager = await readRepoFile("src/components/skills/SkillsManager.tsx");
  const list = await readRepoFile("src/components/skills/SkillList.tsx");
  const card = await readRepoFile("src/components/skills/SkillCard.tsx");
  assert.match(manager, /workflow/);
  assert.match(manager, /capability/);
  assert.match(manager, /extractor/);
  assert.match(manager, /policy/);
  assert.match(manager, /s\.skillClass/);
  assert.match(list, /skillClass/);
  assert.match(card, /SKILL_CLASS_LABELS/);
});
