import assert from "node:assert/strict";
import test from "node:test";

import { AppError } from "../src/errors/app-error.js";
import { NovelForgeMvp } from "../src/services/novelforge-mvp.js";
import { createTempWorkspace, removeTempWorkspace } from "./helpers/temp-workspace.js";

test("打开无效目录时返回标准错误", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    await assert.rejects(
      () => mvp.project.openProject(workspace),
      (error: unknown) => error instanceof AppError && error.code === "PROJECT_INVALID_PATH"
    );
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("未配置模型时 AI 请求应失败", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    const project = await mvp.project.createProject({
      name: "ai-error",
      genre: "都市",
      saveDirectory: workspace
    });
    const chapter = await mvp.chapter.createChapter(project.projectRoot, {
      title: "第一章",
      summary: "summary"
    });

    await assert.rejects(
      () =>
        mvp.ai.generatePreview(project.projectRoot, {
          taskType: "generate_chapter_draft",
          chapterId: chapter.id,
          userInstruction: "继续写"
        }),
      (error: unknown) => error instanceof AppError && error.code === "AI_PROVIDER_NOT_CONFIGURED"
    );
  } finally {
    await removeTempWorkspace(workspace);
  }
});
