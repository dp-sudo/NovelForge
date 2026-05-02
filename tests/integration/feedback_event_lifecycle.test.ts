import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("反馈生命周期契约：feedback_events 扩展状态流转字段", async () => {
  const migration = await readRepoFile("src-tauri/migrations/project/0011_feedback_event_lifecycle.sql");
  assert.match(migration, /ALTER TABLE feedback_events ADD COLUMN resolved_at TEXT/i);
  assert.match(migration, /ALTER TABLE feedback_events ADD COLUMN resolved_by TEXT/i);
  assert.match(migration, /ALTER TABLE feedback_events ADD COLUMN resolution_note TEXT/i);
});

test("反馈生命周期契约：反馈状态枚举与服务状态机命令落地", async () => {
  const domain = await readRepoFile("src-tauri/src/domain/feedback.rs");
  const service = await readRepoFile("src-tauri/src/services/feedback_service.rs");
  assert.match(domain, /pub enum FeedbackEventStatus/);
  assert.match(domain, /Open/);
  assert.match(domain, /Acknowledged/);
  assert.match(domain, /Resolved/);
  assert.match(domain, /Ignored/);

  assert.match(service, /pub fn acknowledge_feedback_event/);
  assert.match(service, /pub fn resolve_feedback_event/);
  assert.match(service, /pub fn ignore_feedback_event/);
  assert.match(service, /FEEDBACK_EVENT_INVALID_STATUS_TRANSITION/);
  assert.match(service, /build_closed_loop_note/);
});

test("反馈生命周期契约：Dashboard command 与前端面板闭环可用", async () => {
  const commands = await readRepoFile("src-tauri/src/commands/dashboard_commands.rs");
  const lib = await readRepoFile("src-tauri/src/lib.rs");
  const api = await readRepoFile("src/api/statsApi.ts");
  const dashboard = await readRepoFile("src/pages/Dashboard/DashboardPage.tsx");

  assert.match(commands, /pub async fn acknowledge_feedback_event/);
  assert.match(commands, /pub async fn resolve_feedback_event/);
  assert.match(commands, /pub async fn ignore_feedback_event/);
  assert.match(lib, /dashboard_commands::acknowledge_feedback_event/);
  assert.match(lib, /dashboard_commands::resolve_feedback_event/);
  assert.match(lib, /dashboard_commands::ignore_feedback_event/);

  assert.match(api, /acknowledge_feedback_event/);
  assert.match(api, /resolve_feedback_event/);
  assert.match(api, /ignore_feedback_event/);

  assert.match(dashboard, /待处理（open）/);
  assert.match(dashboard, /已确认（acknowledged）/);
  assert.match(dashboard, /已解决（resolved）/);
  assert.match(dashboard, /已忽略（ignored）/);
  assert.match(dashboard, /setFeedbackActionTarget/);
});
