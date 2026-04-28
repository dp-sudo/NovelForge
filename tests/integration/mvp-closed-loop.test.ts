import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { setTimeout as sleep } from "node:timers/promises";

import { NovelForgeMvp } from "../../src/services/novelforge-mvp.js";
import { AppError } from "../../src/errors/app-error.js";
import { openDatabase } from "../../src/infra/db.js";
import { createTempWorkspace, removeTempWorkspace } from "../helpers/temp-workspace.js";

test("MVP 主闭环可执行：项目→蓝图→资产→章节→AI→检查→导出→重开恢复", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    const created = await mvp.project.createProject({
      name: "长夜:行舟?",
      author: "测试作者",
      genre: "玄幻",
      targetWords: 120000,
      saveDirectory: workspace
    });
    assert.ok(created.projectRoot.includes("长夜_行舟_"));

    const opened = await mvp.project.openProject(created.projectRoot);
    assert.equal(opened.project.name, "长夜:行舟?");

    await mvp.blueprint.saveStep(created.projectRoot, "step-01-anchor", "核心灵感：秩序与代价。");
    await mvp.blueprint.markCompleted(created.projectRoot, "step-01-anchor");
    const steps = await mvp.blueprint.listSteps(created.projectRoot);
    const step1 = steps.find((item) => item.stepKey === "step-01-anchor");
    assert.equal(step1?.status, "completed");

    const characterId = await mvp.character.create(created.projectRoot, {
      name: "沈烬",
      roleType: "主角",
      motivation: "查清失踪真相",
      flaw: "过度自责"
    });
    await mvp.world.create(created.projectRoot, {
      title: "夜潮规则",
      category: "世界规则",
      description: "夜潮期间不得离城",
      constraintLevel: "strong"
    });
    await mvp.glossary.create(created.projectRoot, {
      term: "玄灯司",
      termType: "组织名",
      locked: true,
      description: "执法机构"
    });
    await mvp.glossary.create(created.projectRoot, {
      term: "神之注视",
      termType: "禁用词",
      banned: true
    });
    const plotId = await mvp.plot.create(created.projectRoot, {
      title: "发现异常",
      nodeType: "开端",
      sortOrder: 1
    });

    const chapter = await mvp.chapter.createChapter(created.projectRoot, {
      title: "第一章 风起",
      summary: "主角第一次发现异常。",
      targetWords: 3000
    });
    await mvp.chapter.saveChapterContent(
      created.projectRoot,
      chapter.id,
      "沈烬走进雨夜。神之注视像阴影一样覆盖街巷。命运的齿轮开始转动。"
    );

    // 写入章节关联，用于引用检查和上下文收集。
    const db = openDatabase(created.projectRoot);
    db.prepare(
      `
      INSERT INTO chapter_links(id, project_id, chapter_id, target_type, target_id, relation_type, created_at)
      VALUES (?, ?, ?, 'character', ?, 'appears', datetime('now'))
      `
    ).run("link-char-1", opened.project.projectId, chapter.id, characterId);
    db.prepare(
      `
      INSERT INTO chapter_links(id, project_id, chapter_id, target_type, target_id, relation_type, created_at)
      VALUES (?, ?, ?, 'plot_node', ?, 'relates', datetime('now'))
      `
    ).run("link-plot-1", opened.project.projectId, chapter.id, plotId);
    db.close();

    await mvp.chapter.autosaveDraft(
      created.projectRoot,
      chapter.id,
      "这是一份更晚的草稿，尚未正式保存。"
    );
    await sleep(20);
    const recover = await mvp.chapter.recoverDraft(created.projectRoot, chapter.id);
    assert.equal(recover.hasNewerDraft, true);
    assert.match(recover.draftContent ?? "", /更晚的草稿/);

    await mvp.settings.saveProviderConfig(created.projectRoot, {
      providerName: "MockProvider",
      baseUrl: "mock://local",
      model: "mock-model",
      temperature: 0.8,
      maxTokens: 2048,
      stream: true
    });
    const aiPreview = await mvp.ai.generatePreview(created.projectRoot, {
      taskType: "generate_chapter_draft",
      userInstruction: "写出主角在雨夜追查线索的冲突场景",
      chapterId: chapter.id
    });
    assert.ok(aiPreview.preview.length > 0);

    const issues = await mvp.consistency.scanChapter(created.projectRoot, chapter.id);
    assert.ok(issues.length >= 2);

    const exportDir = path.join(created.projectRoot, "exports");
    const chapterOut = path.join(exportDir, "chapter-1.md");
    const bookOut = path.join(exportDir, "book.txt");
    const bookDocxOut = path.join(exportDir, "book.docx");
    const bookPdfOut = path.join(exportDir, "book.pdf");
    const bookEpubOut = path.join(exportDir, "book.epub");
    await mvp.export.exportChapter(created.projectRoot, chapter.id, "md", chapterOut, {
      includeChapterSummary: true
    });
    await mvp.export.exportBook(created.projectRoot, "txt", bookOut, {
      includeChapterTitle: true
    });
    await mvp.export.exportBook(created.projectRoot, "docx", bookDocxOut, {
      includeChapterTitle: true
    });
    await mvp.export.exportBook(created.projectRoot, "pdf", bookPdfOut, {
      includeChapterTitle: true
    });
    await mvp.export.exportBook(created.projectRoot, "epub", bookEpubOut, {
      includeChapterTitle: true
    });
    assert.ok((await fs.readFile(chapterOut, "utf-8")).includes("第一章 风起"));
    assert.ok((await fs.readFile(bookOut, "utf-8")).length > 10);
    assert.ok((await fs.readFile(bookDocxOut, "utf-8")).includes("DOCX EXPORT"));
    assert.ok((await fs.readFile(bookPdfOut, "utf-8")).includes("PDF EXPORT"));
    assert.ok((await fs.readFile(bookEpubOut, "utf-8")).includes("EPUB EXPORT"));

    const reopened = await mvp.project.openProject(created.projectRoot);
    assert.equal(reopened.project.projectId, opened.project.projectId);
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("角色被章节引用时删除应被拦截", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    const created = await mvp.project.createProject({
      name: "引用删除拦截",
      genre: "悬疑",
      saveDirectory: workspace
    });
    const characterId = await mvp.character.create(created.projectRoot, {
      name: "祁曜",
      roleType: "主角"
    });
    const chapter = await mvp.chapter.createChapter(created.projectRoot, {
      title: "第一章",
      summary: "章节摘要"
    });

    const db = openDatabase(created.projectRoot);
    const projectId = (
      db.prepare("SELECT id FROM projects LIMIT 1").get() as {
        id: string;
      }
    ).id;
    db.prepare(
      `
      INSERT INTO chapter_links(id, project_id, chapter_id, target_type, target_id, relation_type, created_at)
      VALUES (?, ?, ?, 'character', ?, 'appears', datetime('now'))
      `
    ).run("link-char-test", projectId, chapter.id, characterId);
    db.close();

    await assert.rejects(
      () => mvp.character.softDelete(created.projectRoot, characterId),
      (error: unknown) => error instanceof AppError && error.code === "CHARACTER_REFERENCED"
    );
  } finally {
    await removeTempWorkspace(workspace);
  }
});
