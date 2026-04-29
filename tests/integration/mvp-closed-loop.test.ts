import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("问题1回归：并行 Node 运行时已迁出 src 主目录", async () => {
  const srcServicesPath = path.join(REPO_ROOT, "src", "services");
  const srcInfraDbPath = path.join(REPO_ROOT, "src", "infra", "db.ts");

  const servicesExists = await fs
    .stat(srcServicesPath)
    .then(() => true)
    .catch(() => false);
  const dbExists = await fs
    .stat(srcInfraDbPath)
    .then(() => true)
    .catch(() => false);

  assert.equal(servicesExists, false, "src/services 仍存在并行 Node 业务运行时目录");
  assert.equal(dbExists, false, "src/infra/db.ts 仍存在并行 Node 数据访问层");
});

test("问题2回归：AI 主链路命令面已收敛到 pipeline", async () => {
  const libRs = await readRepoFile("src-tauri/src/lib.rs");
  const aiCommands = await readRepoFile("src-tauri/src/commands/ai_commands.rs");

  const removedCommands = [
    "ai_generate_character",
    "ai_generate_world_rule",
    "ai_generate_plot_node",
    "ai_generate_glossary_term",
    "ai_generate_narrative_obligation",
    "ai_scan_consistency",
    "generate_blueprint_suggestion",
  ];

  for (const command of removedCommands) {
    assert.equal(
      libRs.includes(`::${command}`),
      false,
      `重复 AI 命令仍在 invoke_handler 注册: ${command}`,
    );
    assert.equal(
      aiCommands.includes(`fn ${command}`),
      false,
      `重复 AI 命令实现仍存在: ${command}`,
    );
  }

  assert.equal(libRs.includes("::run_ai_task_pipeline"), true);
  assert.equal(libRs.includes("::cancel_ai_task_pipeline"), true);
});

test("问题5回归：一键全书生成已接入 Blueprint 页面可达入口", async () => {
  const blueprintPage = await readRepoFile("src/pages/Blueprint/BlueprintPage.tsx");
  assert.match(blueprintPage, /streamBookGenerationPipeline/);
  assert.match(blueprintPage, /一键全书生成/);
  assert.match(blueprintPage, /handleRunBookPipeline/);
});
