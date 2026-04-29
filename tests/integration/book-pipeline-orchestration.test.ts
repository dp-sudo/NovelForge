import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import { buildBookStages } from "../../src/api/bookPipelineApi.js";

test("问题5编排回归：全书生成阶段顺序稳定且可追踪", () => {
  const stages = buildBookStages({
    projectRoot: "F:/demo",
    ideaPrompt: "赛博朋克复仇史诗",
    chapterId: "chapter-1",
  });

  assert.deepEqual(
    stages.map((stage) => stage.key),
    [
      "blueprint-anchor",
      "blueprint-genre",
      "blueprint-premise",
      "blueprint-characters",
      "blueprint-world",
      "blueprint-glossary",
      "blueprint-plot",
      "blueprint-chapters",
      "character-seed",
      "world-seed",
      "plot-seed",
      "glossary-seed",
      "narrative-seed",
      "chapter-plan",
    ]
  );
  assert.ok(stages.every((stage) => stage.request.projectRoot === "F:/demo"));
  assert.equal(stages[0]?.request.autoPersist, true);
  assert.equal(stages.at(-1)?.request.autoPersist, false);
});

test("问题5编排回归：蓝图与种子阶段提示词强制 JSON 回填协议", () => {
  const stages = buildBookStages({
    projectRoot: "F:/demo",
    ideaPrompt: "赛博仙侠复仇",
    chapterId: "chapter-1",
  });

  const shouldEnforceJson = stages.filter((stage) =>
    stage.key.startsWith("blueprint-") ||
    stage.key === "character-seed" ||
    stage.key === "world-seed" ||
    stage.key === "plot-seed" ||
    stage.key === "glossary-seed" ||
    stage.key === "narrative-seed"
  );

  assert.ok(shouldEnforceJson.length >= 13);
  assert.ok(shouldEnforceJson.every((stage) => stage.request.userInstruction.includes("必须只输出")));
  assert.ok(shouldEnforceJson.every((stage) => stage.request.userInstruction.includes("JSON")));
});

test("问题5编排回归：无章节上下文时不注入 chapter-plan 阶段", () => {
  const stages = buildBookStages({
    projectRoot: "F:/demo",
    ideaPrompt: "群像推理",
  });

  assert.equal(stages.some((stage) => stage.key === "chapter-plan"), false);
});

test("问题5编排回归：Blueprint 页面存在一键全书生成入口", async () => {
  const page = await fs.readFile(
    path.join(process.cwd(), "src/pages/Blueprint/BlueprintPage.tsx"),
    "utf-8",
  );
  assert.match(page, /streamBookGenerationPipeline/);
  assert.match(page, /一键全书生成/);
});

test("问题5回填回归：手动应用 AI 结果使用统一解析器", async () => {
  const page = await fs.readFile(
    path.join(process.cwd(), "src/pages/Blueprint/BlueprintPage.tsx"),
    "utf-8",
  );
  assert.match(page, /setFormData\(parseBlueprintContent\(cur\.key, aiResult\)\)/);
});
