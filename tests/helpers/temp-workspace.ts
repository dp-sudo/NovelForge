import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

export async function createTempWorkspace(prefix = "novelforge-test-"): Promise<string> {
  return fs.mkdtemp(path.join(os.tmpdir(), prefix));
}

export async function removeTempWorkspace(workspace: string): Promise<void> {
  await fs.rm(workspace, { recursive: true, force: true });
}
