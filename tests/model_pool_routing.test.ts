import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("模型池路由契约：应用级迁移创建模型池表并扩展任务路由字段", async () => {
  const migration = await readRepoFile("src-tauri/migrations/app/0004_model_pools.sql");
  assert.match(migration, /CREATE TABLE IF NOT EXISTS llm_model_pools/);
  assert.match(migration, /ALTER TABLE llm_task_routes ADD COLUMN model_pool_id/);
  assert.match(migration, /ALTER TABLE llm_task_routes ADD COLUMN fallback_model_pool_id/);
});

test("模型池路由契约：后端路由支持 task -> pool -> provider\/model 兼容链路", async () => {
  const aiService = await readRepoFile("src-tauri/src/services/ai_service.rs");
  assert.match(aiService, /default_model_pool_id_for_task/);
  assert.match(aiService, /resolve_with_model_pool_if_configured/);
  assert.match(aiService, /model_pool_id/);
  assert.match(aiService, /fallback_model_pool_id/);
});

test("模型池路由契约：前端 TaskRoute DTO 兼容模型池字段", async () => {
  const aiTypes = await readRepoFile("src/types/ai.ts");
  assert.match(aiTypes, /modelPoolId\?: string;/);
  assert.match(aiTypes, /fallbackModelPoolId\?: string;/);
  const poolTypes = await readRepoFile("src/types/modelPool.ts");
  assert.match(poolTypes, /export interface ModelPool/);
  assert.match(poolTypes, /entries: ModelPoolEntry\[];/);
});

