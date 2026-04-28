import type { DatabaseSync } from "node:sqlite";

export function getProjectId(db: DatabaseSync): string {
  const row = db.prepare("SELECT id FROM projects LIMIT 1").get() as { id: string };
  return row.id;
}

export function parseJsonList(value: unknown): string[] {
  if (typeof value !== "string" || value.trim().length === 0) {
    return [];
  }
  try {
    const parsed = JSON.parse(value) as unknown;
    return Array.isArray(parsed) ? parsed.map((item) => String(item)) : [];
  } catch {
    return [];
  }
}
