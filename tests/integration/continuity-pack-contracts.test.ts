import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("Continuity Pack 契约：PromptResolver 显式注入 7 类上下文", async () => {
  const resolver = await readRepoFile("src-tauri/src/services/ai_pipeline/prompt_resolver.rs");
  assert.match(resolver, /Constitution Context/);
  assert.match(resolver, /Canon Context/);
  assert.match(resolver, /Lexicon Policy Context/);
  assert.match(resolver, /State Context/);
  assert.match(resolver, /Promise Context/);
  assert.match(resolver, /Window Plan Context/);
  assert.match(resolver, /Recent Continuity Context/);
});

test("Continuity Pack 契约：编译器结构与深度策略存在", async () => {
  const compiler = await readRepoFile("src-tauri/src/services/ai_pipeline/continuity_pack.rs");
  assert.match(compiler, /pub struct ContinuityPack/);
  assert.match(compiler, /pub constitution_context: Vec<String>/);
  assert.match(compiler, /pub canon_context: Vec<String>/);
  assert.match(compiler, /pub lexicon_policy_context: Vec<String>/);
  assert.match(compiler, /pub state_context: Vec<String>/);
  assert.match(compiler, /pub promise_context: Vec<String>/);
  assert.match(compiler, /pub window_plan_context: Vec<String>/);
  assert.match(compiler, /pub recent_continuity_context: Vec<String>/);
  assert.match(compiler, /enum ContinuityPackDepth/);
  assert.match(compiler, /\"minimal\"/);
  assert.match(compiler, /\"standard\"/);
  assert.match(compiler, /\"deep\"/);
});

test("Continuity Pack 契约：Orchestrator 在 Prompt 前编译并传入 Resolver", async () => {
  const orchestrator = await readRepoFile("src-tauri/src/services/ai_pipeline/orchestrator.rs");
  assert.match(orchestrator, /ContinuityPackCompiler/);
  assert.match(orchestrator, /resolve_continuity_pack_depth/);
  assert.match(orchestrator, /continuity_pack = ContinuityPackCompiler(?:::default\(\))?\.compile/);
  assert.match(orchestrator, /phase: PHASE_PROMPT/);
  assert.match(orchestrator, /&continuity_pack/);
});

test("Continuity Pack 契约：ContextService 暴露编译所需查询接口", async () => {
  const contextService = await readRepoFile("src-tauri/src/services/context_service.rs");
  assert.match(contextService, /pub fn get_constitution_context/);
  assert.match(contextService, /pub fn get_canon_context/);
  assert.match(contextService, /pub fn get_state_summary/);
  assert.match(contextService, /pub fn get_promise_context/);
  assert.match(contextService, /pub fn get_window_plan/);
  assert.match(contextService, /pub fn get_recent_continuity/);
});
