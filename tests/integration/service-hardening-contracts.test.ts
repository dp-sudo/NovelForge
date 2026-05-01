import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("provider 配置写入应通过一致性 helper 收口，避免 secret/app-db 顺序分叉", async () => {
  const settingsService = await readRepoFile("src-tauri/src/services/settings_service.rs");

  assert.match(settingsService, /persist_provider_with_secret/);
  assert.match(settingsService, /delete_provider_with_secret/);
});

test("备份写入应使用流式复制而不是整文件 read_to_end", async () => {
  const backupService = await readRepoFile("src-tauri/src/services/backup_service.rs");
  const createBackupSection = backupService
    .split("pub fn create_backup")[1]
    ?.split("pub fn try_auto_backup")[0];

  assert.ok(createBackupSection, "create_backup section missing");
  assert.match(createBackupSection, /std::io::copy/);
  assert.doesNotMatch(createBackupSection, /read_to_end\(&mut content\)/);
});
