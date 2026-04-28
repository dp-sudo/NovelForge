import assert from "node:assert/strict";
import test from "node:test";

import { NovelForgeMvp } from "../src/services/novelforge-mvp.js";
import { openDatabase } from "../src/infra/db.js";
import { createTempWorkspace, removeTempWorkspace } from "./helpers/temp-workspace.js";

test("自动保存草稿不应提升正式版本号", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    const project = await mvp.project.createProject({
      name: "autosave-version",
      genre: "科幻",
      saveDirectory: workspace
    });
    const chapter = await mvp.chapter.createChapter(project.projectRoot, {
      title: "第一章",
      summary: "test"
    });
    await mvp.chapter.autosaveDraft(project.projectRoot, chapter.id, "草稿内容");

    const db = openDatabase(project.projectRoot);
    const version = (
      db.prepare("SELECT version FROM chapters WHERE id = ?").get(chapter.id) as {
        version: number;
      }
    ).version;
    db.close();
    assert.equal(version, 1);
  } finally {
    await removeTempWorkspace(workspace);
  }
});
