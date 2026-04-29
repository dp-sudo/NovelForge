import path from "node:path";

export function toPosixRelative(fromDir: string, target: string): string {
  return path.relative(fromDir, target).split(path.sep).join("/");
}

export function sanitizeProjectDirectoryName(name: string): string {
  const trimmed = name.trim();
  const noIllegalChars = trimmed.replace(/[<>:"/\\|?*\x00-\x1F]/g, "_");
  const collapsed = noIllegalChars.replace(/\s+/g, " ").trim();
  return collapsed.length > 0 ? collapsed : "novelforge-project";
}

export function chapterFileName(index: number): string {
  return `ch-${index.toString().padStart(4, "0")}.md`;
}
