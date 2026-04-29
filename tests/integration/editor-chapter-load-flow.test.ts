import assert from "node:assert/strict";
import test from "node:test";

import { loadEditorChapterContentWithRecovery } from "../../src/pages/Editor/chapterLoadFlow.js";

test("问题1回归：打开已有章节时应先加载历史正文，再给出草稿恢复提示", async () => {
  const result = await loadEditorChapterContentWithRecovery({
    chapterId: "chapter-1",
    projectRoot: "F:/dummy",
    readChapterContent: async () => "正式正文-A",
    recoverDraft: async () => ({
      hasNewerDraft: true,
      draftContent: "草稿正文-A"
    })
  });

  assert.equal(result.persistedContent, "正式正文-A");
  assert.equal(result.recoveryContent, "草稿正文-A");
});

test("问题1回归：切换章节时不应把已有正文清空为占位内容", async () => {
  const readByChapter: Record<string, string> = {
    "chapter-1": "第一章已有正文",
    "chapter-2": "第二章已有正文"
  };

  const noDraftRecovery = async () => ({ hasNewerDraft: false, draftContent: undefined });

  const chapter1 = await loadEditorChapterContentWithRecovery({
    chapterId: "chapter-1",
    projectRoot: "F:/dummy",
    readChapterContent: async (chapterId) => readByChapter[chapterId] ?? "",
    recoverDraft: noDraftRecovery
  });
  const chapter2 = await loadEditorChapterContentWithRecovery({
    chapterId: "chapter-2",
    projectRoot: "F:/dummy",
    readChapterContent: async (chapterId) => readByChapter[chapterId] ?? "",
    recoverDraft: noDraftRecovery
  });

  assert.equal(chapter1.persistedContent, "第一章已有正文");
  assert.equal(chapter1.recoveryContent, null);
  assert.equal(chapter2.persistedContent, "第二章已有正文");
  assert.equal(chapter2.recoveryContent, null);
});
