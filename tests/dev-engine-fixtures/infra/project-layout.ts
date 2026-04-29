import fs from "node:fs/promises";
import path from "node:path";

const REQUIRED_DIRS = [
  "database",
  "database/backups",
  "manuscript",
  "manuscript/chapters",
  "manuscript/drafts",
  "manuscript/snapshots",
  "blueprint",
  "assets",
  "assets/covers",
  "assets/attachments",
  "exports",
  "backups",
  "prompts",
  "workflows",
  "logs"
];

export async function initializeProjectDirectories(projectRoot: string): Promise<void> {
  for (const dir of REQUIRED_DIRS) {
    await fs.mkdir(path.join(projectRoot, dir), { recursive: true });
  }
}

export async function ensurePathExists(targetPath: string): Promise<boolean> {
  try {
    await fs.access(targetPath);
    return true;
  } catch {
    return false;
  }
}
