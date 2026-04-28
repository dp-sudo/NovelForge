import assert from "node:assert/strict";
import test from "node:test";

import { AppError } from "../../src/errors/app-error.js";
import { openDatabase } from "../../src/infra/db.js";
import { AiService } from "../../src/services/ai-service.js";
import { NovelForgeMvp } from "../../src/services/novelforge-mvp.js";
import { createTempWorkspace, removeTempWorkspace } from "../helpers/temp-workspace.js";

test("核心链路：章节重排支持交换顺序且索引连续", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();

  try {
    const project = await mvp.project.createProject({
      name: "reorder-core",
      genre: "悬疑",
      saveDirectory: workspace
    });

    const chapter1 = await mvp.chapter.createChapter(project.projectRoot, {
      title: "第一章",
      summary: "A"
    });
    const chapter2 = await mvp.chapter.createChapter(project.projectRoot, {
      title: "第二章",
      summary: "B"
    });
    const chapter3 = await mvp.chapter.createChapter(project.projectRoot, {
      title: "第三章",
      summary: "C"
    });

    await mvp.chapter.reorderChapters(project.projectRoot, [chapter3.id, chapter1.id, chapter2.id]);

    const chapters = await mvp.chapter.listChapters(project.projectRoot);
    assert.deepEqual(
      chapters.map((item) => item.id),
      [chapter3.id, chapter1.id, chapter2.id]
    );
    assert.deepEqual(
      chapters.map((item) => item.chapterIndex),
      [1, 2, 3]
    );
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("核心链路：Provider 配置不写入 apiKey 明文字段且可回读 hasApiKey", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();

  try {
    const project = await mvp.project.createProject({
      name: "provider-security",
      genre: "科幻",
      saveDirectory: workspace
    });

    await mvp.settings.saveProviderConfig(project.projectRoot, {
      providerName: "MockProvider",
      baseUrl: "https://example.invalid/v1",
      model: "gpt-test",
      temperature: 0.7,
      maxTokens: 1024,
      stream: true,
      apiKey: "sk-test-security"
    });

    const db = openDatabase(project.projectRoot);
    const row = db
      .prepare("SELECT value FROM settings WHERE key = ?")
      .get("ai.provider_config") as { value: string };
    db.close();

    const persisted = JSON.parse(row.value) as Record<string, unknown>;
    assert.equal("apiKey" in persisted, false);

    const config = await mvp.settings.getProviderConfig(project.projectRoot);
    assert.equal(config.hasApiKey, true);
    assert.equal(config.providerName, "MockProvider");
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("核心链路：AI 成功与失败都写入 ai_requests 状态", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();

  try {
    const project = await mvp.project.createProject({
      name: "ai-request-status",
      genre: "都市",
      saveDirectory: workspace
    });
    const chapter = await mvp.chapter.createChapter(project.projectRoot, {
      title: "第一章",
      summary: "summary"
    });

    await mvp.settings.saveProviderConfig(project.projectRoot, {
      providerName: "MockProvider",
      baseUrl: "mock://local",
      model: "mock-model",
      temperature: 0.8,
      maxTokens: 2048,
      stream: true
    });

    const success = await mvp.ai.generatePreview(project.projectRoot, {
      taskType: "generate_chapter_draft",
      chapterId: chapter.id,
      userInstruction: "写一段夜雨中的追逐"
    });
    assert.ok(success.preview.length > 0);

    await mvp.settings.saveProviderConfig(project.projectRoot, {
      providerName: "FailingProvider",
      baseUrl: "https://example.invalid/v1",
      model: "failing-model",
      temperature: 0.6,
      maxTokens: 1024,
      stream: false,
      apiKey: "sk-failing-provider"
    });

    const failingAi = new AiService({
      fetchImpl: async () => {
        throw new Error("network down");
      }
    });

    await assert.rejects(
      () =>
        failingAi.generatePreview(project.projectRoot, {
          taskType: "generate_chapter_draft",
          chapterId: chapter.id,
          userInstruction: "触发失败链路"
        }),
      (error: unknown) => error instanceof AppError
    );

    const db = openDatabase(project.projectRoot);
    const successRow = db
      .prepare("SELECT status FROM ai_requests WHERE id = ?")
      .get(success.requestId) as { status: string };
    const errorRow = db
      .prepare(
        "SELECT status, error_code, error_message FROM ai_requests WHERE status = 'error' ORDER BY created_at DESC LIMIT 1"
      )
      .get() as { status: string; error_code: string | null; error_message: string | null };
    db.close();

    assert.equal(successRow.status, "done");
    assert.equal(errorRow.status, "error");
    assert.ok((errorRow.error_code ?? "").length > 0);
    assert.ok((errorRow.error_message ?? "").length > 0);
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("核心链路：一致性扫描结果可更新状态并持久化", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();

  try {
    const project = await mvp.project.createProject({
      name: "consistency-status",
      genre: "奇幻",
      saveDirectory: workspace
    });
    const chapter = await mvp.chapter.createChapter(project.projectRoot, {
      title: "第一章",
      summary: "summary"
    });

    await mvp.glossary.create(project.projectRoot, {
      term: "神之注视",
      termType: "禁用词",
      banned: true
    });

    await mvp.chapter.saveChapterContent(
      project.projectRoot,
      chapter.id,
      "命运的齿轮开始转动，神之注视在黑夜里凝结成冰。"
    );

    const scanIssues = await mvp.consistency.scanChapter(project.projectRoot, chapter.id);
    assert.ok(scanIssues.length >= 2);

    const issues = await mvp.consistency.listIssues(project.projectRoot);
    assert.ok(issues.length >= 2);

    const firstIssue = issues[0] as { id: string };
    await mvp.consistency.updateIssueStatus(project.projectRoot, firstIssue.id, "fixed");

    const updatedIssues = await mvp.consistency.listIssues(project.projectRoot);
    const updated = updatedIssues.find(
      (item) => (item as { id: string }).id === firstIssue.id
    ) as { status: string } | undefined;
    assert.equal(updated?.status, "fixed");
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("核心链路：创建与打开项目后应写入最近项目", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();

  try {
    const created = await mvp.project.createProject({
      name: "recent-project-core",
      genre: "测试",
      saveDirectory: workspace
    });
    await mvp.project.openProject(created.projectRoot);

    const recent = await mvp.project.listRecentProjects();
    assert.ok(recent.some((item) => item.projectPath === created.projectRoot));
  } finally {
    await removeTempWorkspace(workspace);
  }
});
