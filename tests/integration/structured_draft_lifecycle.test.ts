import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("structured draft 生命周期契约：reject 命令已注册并暴露前端调用", async () => {
  const contextCommands = await readRepoFile("src-tauri/src/commands/context_commands.rs");
  const libRs = await readRepoFile("src-tauri/src/lib.rs");
  const contextApi = await readRepoFile("src/api/contextApi.ts");

  assert.match(contextCommands, /pub async fn reject_structured_draft/);
  assert.match(libRs, /commands::context_commands::reject_structured_draft/);
  assert.match(contextApi, /invokeCommand<RejectStructuredDraftResult>\("reject_structured_draft"/);
});

test("structured draft 生命周期契约：编辑器提供忽略草案入口", async () => {
  const editorPage = await readRepoFile("src/pages/Editor/EditorPage.tsx");
  const contextPanel = await readRepoFile("src/pages/Editor/components/EditorContextPanel.tsx");

  assert.match(editorPage, /rejectStructuredDraft\(projectRoot,\s*chapterId,\s*draftItemId\)/);
  assert.match(editorPage, /onRejectStructuredDraft=\{handleRejectStructuredDraft\}/);
  assert.match(contextPanel, /onRejectStructuredDraft/);
  assert.match(contextPanel, /忽略/);
});

test("structured draft 生命周期契约：API 与运行文档同步 reject/completed 语义", async () => {
  const apiDoc = await readRepoFile("docs/api/api-integration-spec.md");
  const runtimeDoc = await readRepoFile("docs/runtime/runtime-process-spec.md");

  assert.match(apiDoc, /rejectStructuredDraft\s*->\s*reject_structured_draft/);
  assert.match(apiDoc, /pending -> applied \| rejected/);
  assert.match(apiDoc, /structured_draft_batches\.status = completed/);
  assert.match(runtimeDoc, /reject_structured_draft/);
  assert.match(runtimeDoc, /pending -> applied \| rejected/);
  assert.match(runtimeDoc, /structured_draft_batches\.status = completed/);
});
