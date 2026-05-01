import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

function extractRegisteredCommands(libRsContent: string): Set<string> {
  const commands = new Set<string>();
  const pattern = /commands::[a-z_]+::([a-z_][a-z0-9_]*)/g;
  for (const match of libRsContent.matchAll(pattern)) {
    commands.add(match[1]);
  }
  return commands;
}

function extractInvokedCommands(apiContent: string): Set<string> {
  const commands = new Set<string>();
  const pattern = /invokeCommand(?:<[^>]+>)?\("([a-z0-9_]+)"/g;
  for (const match of apiContent.matchAll(pattern)) {
    commands.add(match[1]);
  }
  return commands;
}

test("问题2契约验证：src/api 调用的 command 必须在 Tauri invoke_handler 注册", async () => {
  const apiDir = path.join(REPO_ROOT, "src", "api");
  const files = (await fs.readdir(apiDir))
    .filter((name) => name.endsWith(".ts") && name !== "dev-engine.ts");
  const invokeCommands = new Set<string>();

  for (const file of files) {
    const content = await fs.readFile(path.join(apiDir, file), "utf-8");
    for (const command of extractInvokedCommands(content)) {
      invokeCommands.add(command);
    }
  }

  const libRs = await readRepoFile("src-tauri/src/lib.rs");
  const registered = extractRegisteredCommands(libRs);
  const missing = [...invokeCommands].filter((cmd) => !registered.has(cmd)).sort();

  assert.deepEqual(
    missing,
    [],
    `src/api 发现未注册命令: ${missing.join(", ")}`
  );
});

test("问题2契约验证：run_ai_task_pipeline payload 形状保持前后端一致", async () => {
  const pipelineApi = await readRepoFile("src/api/pipelineApi.ts");

  assert.match(
    pipelineApi,
    /invokeCommand<string>\("run_ai_task_pipeline",\s*\{\s*input:\s*\{/s
  );
  assert.match(pipelineApi, /projectRoot:\s*input\.projectRoot/);
  assert.match(pipelineApi, /taskType:\s*input\.taskType/);
  assert.match(pipelineApi, /userInstruction:\s*input\.userInstruction\s*\?\?\s*""/);
  assert.match(pipelineApi, /const policy = resolvePersistPolicy\(input\);/);
  assert.match(pipelineApi, /autoPersist:\s*policy\.autoPersist/);
  assert.match(pipelineApi, /persistMode:\s*policy\.persistMode/);
  assert.match(pipelineApi, /automationTier:\s*policy\.automationTier/);
  assert.match(
    pipelineApi,
    /invokeCommand<void>\("cancel_ai_task_pipeline",\s*\{\s*requestId\s*\}\s*\)/
  );
});

test("问题2契约验证：pipeline 事件协议名与事件类型前后端一致", async () => {
  const pipelineApi = await readRepoFile("src/api/pipelineApi.ts");
  const pipelineService = await readRepoFile("src-tauri/src/services/ai_pipeline_service.rs");

  const frontendEventName = /const PIPELINE_EVENT_NAME = "([^"]+)"/.exec(pipelineApi);
  const backendEventName = /const PIPELINE_EVENT_NAME: &str = "([^"]+)"/.exec(pipelineService);

  assert.ok(frontendEventName, "前端 PIPELINE_EVENT_NAME 缺失");
  assert.ok(backendEventName, "后端 PIPELINE_EVENT_NAME 缺失");
  assert.equal(frontendEventName?.[1], backendEventName?.[1]);
  assert.equal(frontendEventName?.[1], "ai:pipeline:event");

  assert.match(
    pipelineApi,
    /type AiPipelineEventType = "start" \| "delta" \| "progress" \| "done" \| "error"/
  );
  assert.match(pipelineService, /event_type:\s*"done"\.to_string\(\)/);
  assert.match(pipelineService, /event_type:\s*"error"\.to_string\(\)/);
});

test("问题2最小UI烟雾验证：章节页到编辑器路由入口仍然可达", async () => {
  const router = await readRepoFile("src/app/router.tsx");
  const chaptersPage = await readRepoFile("src/pages/Chapters/ChaptersPage.tsx");

  assert.match(router, /chapters:\s*ChaptersPage/);
  assert.match(router, /editor:\s*EditorPage/);
  assert.match(chaptersPage, /setActiveChapter\(chapter\.id,\s*chapter\.title\)/);
  assert.match(chaptersPage, /setActiveRoute\("editor"\)/);
});

test("问题6契约验证：compatibility-only 命令从强制注册约束解绑并标注移除里程碑", async () => {
  const libRs = await readRepoFile("src-tauri/src/lib.rs");
  const settingsCommands = await readRepoFile("src-tauri/src/commands/settings_commands.rs");
  const aiCommands = await readRepoFile("src-tauri/src/commands/ai_commands.rs");
  const loggerInfra = await readRepoFile("src-tauri/src/infra/logger.rs");
  const apiDoc = await readRepoFile("docs/api/api-integration-spec.md");

  const deprecatedCommands = [
    "load_provider_config",
    "save_provider_config",
    "register_ai_provider",
    "test_ai_connection",
  ];

  for (const command of deprecatedCommands) {
    const stillRegistered = new RegExp(`commands::[a-z_]+::${command}`).test(libRs);
    if (stillRegistered) {
      assert.match(
        apiDoc,
        new RegExp(`${command}[\\s\\S]*compatibility-only[\\s\\S]*2026-07-31`, "i"),
      );
    }
  }

  assert.match(settingsCommands, /\[DEPRECATED_COMMAND\] load_provider_config is compatibility-only/);
  assert.match(settingsCommands, /\[DEPRECATED_COMMAND\] save_provider_config is compatibility-only/);
  assert.match(aiCommands, /\[DEPRECATED_COMMAND\] register_ai_provider is compatibility-only/);
  assert.match(aiCommands, /\[DEPRECATED_COMMAND\] test_ai_connection is compatibility-only/);
  assert.match(loggerInfra, /\[DEPRECATED_COMMAND_USAGE\] command=\{\} source=\{\} count=\{\}/);
  assert.match(settingsCommands, /record_deprecated_command_usage\(command,\s*src\)/);
  assert.match(settingsCommands, /log_deprecated_command\(DEPRECATED_LOAD_PROVIDER_LOG,\s*"load_provider"/);
  assert.match(settingsCommands, /DEPRECATED_LOAD_PROVIDER_CONFIG_LOG,\s*"load_provider_config"/);
  assert.match(settingsCommands, /DEPRECATED_SAVE_PROVIDER_CONFIG_LOG,\s*"save_provider_config"/);
  assert.match(aiCommands, /record_deprecated_command_usage\(command,\s*src\)/);
  assert.match(aiCommands, /log_deprecated_command\(\s*DEPRECATED_REGISTER_AI_PROVIDER_LOG,\s*"register_ai_provider"/);
  assert.match(aiCommands, /log_deprecated_command\(\s*DEPRECATED_TEST_AI_CONNECTION_LOG,\s*"test_ai_connection"/);
});

test("问题3文档同步：AI 主链路统一为 pipeline + 事件流", async () => {
  const runtimeDoc = await readRepoFile("docs/runtime/runtime-process-spec.md");
  const apiDoc = await readRepoFile("docs/api/api-integration-spec.md");

  assert.match(runtimeDoc, /run_ai_task_pipeline \+ ai:pipeline:event/);
  assert.match(apiDoc, /run_ai_task_pipeline/);
  assert.match(apiDoc, /streamTaskPipeline/);
});
