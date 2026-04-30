import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import {
  buildBookStages,
  buildPromotionStages,
  resolveChapterPlanChapterSelection,
  selectPromotionStage,
} from "../../src/api/bookPipelineApi.js";

test("问题5编排回归：全书生成边界默认只覆盖蓝图阶段", () => {
  const stages = buildBookStages({
    projectRoot: "F:/demo",
    ideaPrompt: "赛博朋克复仇史诗",
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
    ],
  );
  assert.ok(stages.every((stage) => stage.request.projectRoot === "F:/demo"));
  assert.equal(stages[0]?.request.autoPersist, true);
  assert.equal(stages.at(-1)?.request.autoPersist, true);
  assert.equal(stages[0]?.request.persistMode, "formal");
  assert.equal(stages[0]?.request.automationTier, "supervised");
});

test("问题5编排回归：晋升阶段独立编排并显式 confirm 档位", () => {
  const stages = buildPromotionStages({
    projectRoot: "F:/demo",
    ideaPrompt: "赛博朋克复仇史诗",
    chapterId: "chapter-1",
  });

  assert.deepEqual(
    stages.map((stage) => stage.key),
    [
      "character-seed",
      "world-seed",
      "plot-seed",
      "glossary-seed",
      "narrative-seed",
      "chapter-plan",
    ],
  );
  assert.ok(stages.every((stage) => stage.request.persistMode === "formal"));
  assert.ok(stages.every((stage) => stage.request.automationTier === "confirm"));
});

test("问题5编排回归：蓝图与种子阶段提示词强制 JSON 回填协议", () => {
  const blueprintStages = buildBookStages({
    projectRoot: "F:/demo",
    ideaPrompt: "赛博仙侠复仇",
  });
  const promotionStages = buildPromotionStages({
    projectRoot: "F:/demo",
    ideaPrompt: "赛博仙侠复仇",
  });

  const shouldEnforceJson = [...blueprintStages, ...promotionStages].filter((stage) =>
    stage.key.startsWith("blueprint-") || stage.key.endsWith("-seed"),
  );

  assert.equal(shouldEnforceJson.length, 13);
  assert.ok(shouldEnforceJson.every((stage) => (stage.request.userInstruction ?? "").includes("JSON")));
  assert.ok(shouldEnforceJson.every((stage) => (stage.request.userInstruction ?? "").length > 0));
});

test("问题5编排回归：无章节上下文时晋升阶段不注入 chapter-plan", () => {
  const stages = buildPromotionStages({
    projectRoot: "F:/demo",
    ideaPrompt: "群像推理",
  });

  assert.equal(stages.some((stage) => stage.key === "chapter-plan"), false);
});

test("P0-1回归：多 target 晋升可显式选择且第二目标可达", () => {
  const stages = buildPromotionStages({
    projectRoot: "F:/demo",
    ideaPrompt: "悬疑长篇",
    chapterId: "chapter-2",
  });
  const selected = selectPromotionStage(stages, "narrative-seed");
  assert.equal(selected?.key, "narrative-seed");
});

test("P1-1回归：chapter-plan 策略 user_specified 可达", () => {
  const selected = resolveChapterPlanChapterSelection({
    chapters: [
      { id: "chapter-1", chapterIndex: 1, status: "planned", targetWords: 2000, currentWords: 500 },
      { id: "chapter-2", chapterIndex: 2, status: "drafting", targetWords: 2000, currentWords: 400 },
    ],
    explicitChapterId: "chapter-2",
    activeChapterId: "chapter-1",
    windowPlanningHorizon: 5,
  });
  assert.deepEqual(selected, { chapterId: "chapter-2", strategy: "user_specified" });
});

test("P1-1回归：chapter-plan 策略 next_porous 可达", () => {
  const selected = resolveChapterPlanChapterSelection({
    chapters: [
      { id: "chapter-1", chapterIndex: 1, status: "completed", targetWords: 2000, currentWords: 2100 },
      { id: "chapter-2", chapterIndex: 2, status: "planned", targetWords: 2000, currentWords: 300 },
      { id: "chapter-3", chapterIndex: 3, status: "planned", targetWords: 2000, currentWords: 120 },
    ],
    activeChapterId: "chapter-1",
    windowPlanningHorizon: 5,
  });
  assert.deepEqual(selected, { chapterId: "chapter-2", strategy: "next_porous" });
});

test("P1-1回归：chapter-plan 策略 window_drift 可达", () => {
  const selected = resolveChapterPlanChapterSelection({
    chapters: [
      { id: "chapter-1", chapterIndex: 1, status: "completed", targetWords: 2000, currentWords: 2000 },
      { id: "chapter-2", chapterIndex: 2, status: "completed", targetWords: 2000, currentWords: 2000 },
      { id: "chapter-3", chapterIndex: 3, status: "completed", targetWords: 2000, currentWords: 3200 },
    ],
    activeChapterId: "chapter-1",
    windowPlanningHorizon: 5,
  });
  assert.deepEqual(selected, { chapterId: "chapter-3", strategy: "window_drift" });
});

test("P1-1回归：chapter-plan 策略 arc_anchor 优先", () => {
  const selected = resolveChapterPlanChapterSelection({
    chapters: [
      { id: "chapter-1", chapterIndex: 1, status: "completed", targetWords: 2000, currentWords: 2000 },
      { id: "chapter-2", chapterIndex: 2, status: "planned", targetWords: 2000, currentWords: 3200 },
      { id: "chapter-3", chapterIndex: 3, status: "revising", targetWords: 2000, currentWords: 2900 },
    ],
    activeChapterId: "chapter-3",
    windowPlanningHorizon: 2,
  });
  assert.deepEqual(selected, { chapterId: "chapter-3", strategy: "arc_anchor" });
});

test("问题5编排回归：Blueprint 页面存在一键全书生成入口", async () => {
  const page = await fs.readFile(
    path.join(process.cwd(), "src/pages/Blueprint/BlueprintPage.tsx"),
    "utf-8",
  );
  assert.match(page, /streamBookGenerationPipeline/);
  assert.match(page, /buildPromotionStages/);
  assert.match(page, /一键全书生成/);
  assert.match(page, /确认并晋升/);
});

test("问题9.5契约：蓝图晋升入口具备 chapter-plan 章节选择策略", async () => {
  const page = await fs.readFile(
    path.join(process.cwd(), "src/pages/Blueprint/BlueprintPage.tsx"),
    "utf-8",
  );
  assert.match(page, /resolvePromotionChapterSelection/);
  assert.match(page, /请选择章节以生成章节计划/);
});

test("问题5编排回归：审阅层页面显示派生审阅提示", async () => {
  const timeline = await fs.readFile(
    path.join(process.cwd(), "src/pages/Timeline/TimelinePage.tsx"),
    "utf-8",
  );
  const relationships = await fs.readFile(
    path.join(process.cwd(), "src/pages/Relationships/RelationshipsPage.tsx"),
    "utf-8",
  );

  assert.match(timeline, /本页为派生审阅层，展示已晋升的正式数据与待审信息/);
  assert.match(relationships, /本页为派生审阅层，展示已晋升的正式数据与待审信息/);
});

test("问题5回填回归：手动应用 AI 结果使用统一解析器", async () => {
  const page = await fs.readFile(
    path.join(process.cwd(), "src/pages/Blueprint/BlueprintPage.tsx"),
    "utf-8",
  );
  assert.match(page, /setFormData\(parseBlueprintContent\(cur\.key, aiResult\)\)/);
});
