import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

export interface RecentProjectItem {
  projectPath: string;
  openedAt: string;
}

const MAX_RECENT_ITEMS = 20;

function recentProjectsFilePath(): string {
  return path.join(os.homedir(), ".novelforge", "recent-projects.json");
}

async function ensureRecentProjectsFile(): Promise<string> {
  const filePath = recentProjectsFilePath();
  await fs.mkdir(path.dirname(filePath), { recursive: true });

  try {
    await fs.access(filePath);
  } catch {
    await fs.writeFile(filePath, "[]", "utf-8");
  }

  return filePath;
}

export async function listRecentProjects(): Promise<RecentProjectItem[]> {
  const filePath = await ensureRecentProjectsFile();
  const raw = await fs.readFile(filePath, "utf-8");
  try {
    const parsed = JSON.parse(raw) as RecentProjectItem[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

async function writeRecentProjects(items: RecentProjectItem[]): Promise<void> {
  const filePath = await ensureRecentProjectsFile();
  const tempFile = `${filePath}.${process.pid}.${Date.now()}.${Math.random().toString(16).slice(2)}.tmp`;
  await fs.writeFile(tempFile, JSON.stringify(items, null, 2), "utf-8");
  try {
    await fs.rename(tempFile, filePath);
  } catch {
    await fs.rm(filePath, { force: true });
    await fs.rename(tempFile, filePath);
  }
}

export async function markRecentProject(projectPath: string): Promise<void> {
  const list = await listRecentProjects();
  const next = [
    { projectPath, openedAt: new Date().toISOString() },
    ...list.filter((item) => item.projectPath !== projectPath)
  ].slice(0, MAX_RECENT_ITEMS);
  await writeRecentProjects(next);
}

export async function removeRecentProject(projectPath: string): Promise<void> {
  const list = await listRecentProjects();
  const next = list.filter((item) => item.projectPath !== projectPath);
  await writeRecentProjects(next);
}
