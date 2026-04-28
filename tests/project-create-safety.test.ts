import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import { AppError } from "../src/errors/app-error.js";
import { sanitizeProjectDirectoryName } from "../src/infra/path-utils.js";
import { NovelForgeMvp } from "../src/services/novelforge-mvp.js";
import { createTempWorkspace, removeTempWorkspace } from "./helpers/temp-workspace.js";

test("创建项目失败时不得删除已存在目录", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  const projectName = "同名目录保留";
  const existingProjectRoot = path.join(workspace, sanitizeProjectDirectoryName(projectName));
  const markerFile = path.join(existingProjectRoot, "keep.txt");

  try {
    await fs.mkdir(existingProjectRoot, { recursive: true });
    await fs.writeFile(markerFile, "must-keep", "utf-8");

    await assert.rejects(
      () =>
        mvp.project.createProject({
          name: projectName,
          genre: "测试",
          saveDirectory: workspace
        }),
      (error: unknown) => error instanceof AppError && error.code === "PROJECT_CREATE_FAILED"
    );

    const markerContent = await fs.readFile(markerFile, "utf-8");
    assert.equal(markerContent, "must-keep");
  } finally {
    await removeTempWorkspace(workspace);
  }
});
