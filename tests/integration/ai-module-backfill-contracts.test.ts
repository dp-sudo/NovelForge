import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

async function readSkill(name: string): Promise<string> {
  return fs.readFile(
    path.join(process.cwd(), "resources", "builtin-skills", name),
    "utf-8",
  );
}

test("问题5回填契约：角色/世界/名词/剧情/叙事技能模板强制 JSON 入库字段", async () => {
  const [character, world, glossary, plot, narrative] = await Promise.all([
    readSkill("character.create.md"),
    readSkill("world.create_rule.md"),
    readSkill("glossary.create_term.md"),
    readSkill("plot.create_node.md"),
    readSkill("narrative.create_obligation.md"),
  ]);

  for (const content of [character, world, glossary, plot, narrative]) {
    assert.match(content, /必须只输出一个 JSON 对象/);
    assert.match(content, /不要 Markdown 代码块/);
  }

  assert.match(character, /name/);
  assert.match(character, /roleType/);
  assert.match(character, /motivation/);

  assert.match(world, /title/);
  assert.match(world, /constraintLevel/);
  assert.match(world, /relatedEntities/);

  assert.match(glossary, /term/);
  assert.match(glossary, /termType/);
  assert.match(glossary, /description/);

  assert.match(plot, /title/);
  assert.match(plot, /nodeType/);
  assert.match(plot, /relatedCharacters/);

  assert.match(narrative, /obligationType/);
  assert.match(narrative, /payoffStatus/);
  assert.match(narrative, /relatedEntities/);
});

