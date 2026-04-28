import fs from "node:fs/promises";
import path from "node:path";

const REDACTION_PATTERNS = [
  /Bearer\s+[A-Za-z0-9\-._~+/]+=*/gi,
  /api[_-]?key["']?\s*[:=]\s*["'][^"']+["']/gi,
  /"apiKey"\s*:\s*"[^"]+"/gi
];

export function redactLog(message: string): string {
  let next = message;
  for (const pattern of REDACTION_PATTERNS) {
    next = next.replace(pattern, "[REDACTED]");
  }
  return next;
}

export async function appendProjectLog(projectRoot: string, line: string): Promise<void> {
  const logDir = path.join(projectRoot, "logs");
  await fs.mkdir(logDir, { recursive: true });
  const logPath = path.join(logDir, "app.log");
  const safe = redactLog(line);
  await fs.appendFile(logPath, `${new Date().toISOString()} ${safe}\n`, "utf-8");
}
