import assert from "node:assert/strict";
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
      "character-seed",
      "world-seed",
      "plot-seed",
      "chapter-plan",
    ]
  );
  assert.ok(stages.every((stage) => stage.request.projectRoot === "F:/demo"));
  assert.equal(stages[0]?.request.autoPersist, true);
  assert.equal(stages.at(-1)?.request.autoPersist, false);
});

test("问题5编排回归：无章节上下文时不注入 chapter-plan 阶段", () => {
  const stages = buildBookStages({
    projectRoot: "F:/demo",
    ideaPrompt: "群像推理",
  });

  assert.equal(stages.some((stage) => stage.key === "chapter-plan"), false);
});
