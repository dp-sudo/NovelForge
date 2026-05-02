import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

type SceneType = "dialogue" | "action" | "exposition" | "introspection" | "combat";

interface BenchmarkSample {
  id: string;
  content: string;
  expected_scene_type: SceneType;
  features?: Record<string, unknown>;
}

interface BaselineMetrics {
  accuracy: number;
}

const REPO_ROOT = process.cwd();
const BENCHMARK_PATH = path.join(REPO_ROOT, "tests/fixtures/scene_classification_benchmark.json");
const BASELINE_PATH = path.join(REPO_ROOT, "tests/fixtures/scene_classification_baseline.json");
const SCENE_TYPES: SceneType[] = ["dialogue", "action", "exposition", "introspection", "combat"];

function countHits(haystack: string, keywords: string[]): number {
  return keywords.filter((keyword) => keyword && haystack.includes(keyword)).length;
}

function estimateDialogueRatio(text: string): number {
  let dialogueChars = 0;
  let totalChars = 0;
  let insideCnQuote = false;
  let insideEnQuote = false;
  for (const ch of text) {
    if (ch === "“") {
      insideCnQuote = true;
      continue;
    }
    if (ch === "”") {
      insideCnQuote = false;
      continue;
    }
    if (ch === "\"") {
      insideEnQuote = !insideEnQuote;
      continue;
    }
    if (/\s/.test(ch)) {
      continue;
    }
    totalChars += 1;
    if (insideCnQuote || insideEnQuote) {
      dialogueChars += 1;
    }
  }
  return totalChars === 0 ? 0 : dialogueChars / totalChars;
}

function classifyScene(content: string): SceneType {
  const lowered = content.toLowerCase();
  const length = Math.max(lowered.length, 1);
  const dialogueHits = countHits(lowered, [
    "dialogue", "conversation", "对话", "说道", "问道", "回答", "争辩", "告白", "“", "”", "\"",
  ]);
  const actionHits = countHits(lowered, [
    "action", "chase", "run", "rush", "move", "行动", "追逐", "逃离", "潜入", "移动", "突袭", "翻身", "闪避", "挥刀", "跃起",
  ]);
  const combatHits = countHits(lowered, [
    "combat", "battle", "fight", "skirmish", "战斗", "厮杀", "交锋", "搏斗", "决战", "反击", "受伤", "流血", "重伤", "骨折",
  ]);
  const expositionHits = countHits(lowered, [
    "exposition", "worldbuilding", "background", "lore", "设定", "背景", "解释", "历史", "说明", "传说", "规则", "起源",
  ]);
  const introspectionHits = countHits(lowered, [
    "introspection", "inner monologue", "内心", "独白", "心理", "回忆", "自省", "思考", "犹豫", "恐惧", "后悔",
  ]);
  const dialogueRatio = estimateDialogueRatio(content);
  const actionDensity = actionHits / length;

  const scores = new Map<SceneType, number>([
    ["dialogue", dialogueHits + dialogueRatio * 10],
    ["action", actionHits + actionDensity * 120],
    ["exposition", expositionHits],
    ["introspection", introspectionHits],
    ["combat", combatHits + actionDensity * 40],
  ]);

  if (dialogueRatio >= 0.6) {
    scores.set("dialogue", (scores.get("dialogue") || 0) + 8);
  }
  if (combatHits >= 2 && actionDensity >= 0.01) {
    scores.set("combat", (scores.get("combat") || 0) + 10);
  }
  if (expositionHits >= 2 && dialogueRatio <= 0.35 && actionDensity <= 0.01) {
    scores.set("exposition", (scores.get("exposition") || 0) + 6);
  }
  if (introspectionHits >= 2 && actionHits <= 2) {
    scores.set("introspection", (scores.get("introspection") || 0) + 6);
  }

  let bestType: SceneType = "action";
  let bestScore = -1;
  for (const [sceneType, score] of scores.entries()) {
    if (score > bestScore) {
      bestScore = score;
      bestType = sceneType;
    }
  }
  return bestType;
}

test("scene classification accuracy regression", async () => {
  const benchmark = JSON.parse(await fs.readFile(BENCHMARK_PATH, "utf-8")) as BenchmarkSample[];
  const baseline = JSON.parse(await fs.readFile(BASELINE_PATH, "utf-8")) as BaselineMetrics;

  let correct = 0;
  const confusion: Record<SceneType, Record<SceneType, number>> = {
    dialogue: { dialogue: 0, action: 0, exposition: 0, introspection: 0, combat: 0 },
    action: { dialogue: 0, action: 0, exposition: 0, introspection: 0, combat: 0 },
    exposition: { dialogue: 0, action: 0, exposition: 0, introspection: 0, combat: 0 },
    introspection: { dialogue: 0, action: 0, exposition: 0, introspection: 0, combat: 0 },
    combat: { dialogue: 0, action: 0, exposition: 0, introspection: 0, combat: 0 },
  };
  const actualTotals: Record<SceneType, number> = {
    dialogue: 0,
    action: 0,
    exposition: 0,
    introspection: 0,
    combat: 0,
  };

  for (const sample of benchmark) {
    const predicted = classifyScene(sample.content);
    confusion[sample.expected_scene_type][predicted] += 1;
    actualTotals[sample.expected_scene_type] += 1;
    if (predicted === sample.expected_scene_type) {
      correct += 1;
    }
  }

  const accuracy = correct / benchmark.length;
  const recall: Record<SceneType, number> = {
    dialogue: confusion.dialogue.dialogue / Math.max(actualTotals.dialogue, 1),
    action: confusion.action.action / Math.max(actualTotals.action, 1),
    exposition: confusion.exposition.exposition / Math.max(actualTotals.exposition, 1),
    introspection: confusion.introspection.introspection / Math.max(actualTotals.introspection, 1),
    combat: confusion.combat.combat / Math.max(actualTotals.combat, 1),
  };

  // eslint-disable-next-line no-console
  console.log("Scene classification accuracy:", accuracy.toFixed(4));
  // eslint-disable-next-line no-console
  console.log("Confusion matrix:", JSON.stringify(confusion));
  // eslint-disable-next-line no-console
  console.log("Recall by scene:", JSON.stringify(recall));

  assert.ok(accuracy >= 0.85, `accuracy=${accuracy.toFixed(4)} should be >= 0.85`);
  assert.ok(
    baseline.accuracy - accuracy <= 0.05,
    `accuracy drop is too high: baseline=${baseline.accuracy.toFixed(4)} current=${accuracy.toFixed(4)}`,
  );
  for (const sceneType of SCENE_TYPES) {
    assert.ok(
      recall[sceneType] >= 0.75,
      `${sceneType} recall=${recall[sceneType].toFixed(4)} should be >= 0.75`,
    );
  }
});
