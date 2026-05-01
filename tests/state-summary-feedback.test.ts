import assert from "node:assert/strict";
import test from "node:test";

import { summarizeStateDeltaForFeedback, type ChapterContext } from "../src/api/contextApi.js";

function makeContext(stateSummary: ChapterContext["stateSummary"]): Pick<ChapterContext, "stateSummary"> {
  return { stateSummary };
}

test("状态摘要反馈：窗口进度仍保持专用文案", () => {
  const lines = summarizeStateDeltaForFeedback(
    makeContext([
      {
        subjectType: "window",
        subjectId: "current_window",
        stateKind: "progress",
        payload: {
          chapterId: "chapter-1",
          wordCount: 1680,
        },
      },
    ]),
  );

  assert.deepEqual(lines, ["窗口进度更新：chapter-1（1680 字）"]);
});

test("状态摘要反馈：结构化草案写入的状态应输出可读文案", () => {
  const lines = summarizeStateDeltaForFeedback(
    makeContext([
      {
        subjectType: "relationship",
        subjectId: "rel-1",
        stateKind: "relationship",
        payload: {
          sourceLabel: "林夜",
          targetLabel: "李伯",
          relationshipType: "同盟",
        },
      },
      {
        subjectType: "character",
        subjectId: "char-1",
        stateKind: "involvement",
        payload: {
          characterLabel: "林夜",
          involvementType: "高参与",
        },
      },
      {
        subjectType: "scene",
        subjectId: "scene-1",
        stateKind: "scene",
        payload: {
          sceneLabel: "青石镇",
          sceneType: "地点场景",
        },
      },
    ]),
  );

  assert.deepEqual(lines, [
    "关系状态更新：林夜 ↔ 李伯（同盟）",
    "角色戏份更新：林夜（高参与）",
    "场景状态更新：青石镇（地点场景）",
  ]);
});
