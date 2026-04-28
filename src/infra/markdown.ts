import fs from "node:fs/promises";
import path from "node:path";

function formatValue(value: string | number | string[]): string {
  if (Array.isArray(value)) {
    if (value.length === 0) {
      return "[]";
    }
    return `\n${value.map((item) => `  - ${item}`).join("\n")}`;
  }
  return String(value);
}

export function buildChapterMarkdown(input: {
  id: string;
  index: number;
  title: string;
  status: string;
  summary: string;
  wordCount: number;
  createdAt: string;
  updatedAt: string;
  linkedPlotNodes?: string[];
  appearingCharacters?: string[];
  linkedWorldRules?: string[];
  content: string;
}): string {
  const frontmatter = [
    "---",
    `id: ${input.id}`,
    `index: ${input.index}`,
    `title: ${input.title}`,
    `status: ${input.status}`,
    `summary: ${input.summary}`,
    `wordCount: ${input.wordCount}`,
    `createdAt: ${input.createdAt}`,
    `updatedAt: ${input.updatedAt}`,
    `linkedPlotNodes: ${formatValue(input.linkedPlotNodes ?? [])}`,
    `appearingCharacters: ${formatValue(input.appearingCharacters ?? [])}`,
    `linkedWorldRules: ${formatValue(input.linkedWorldRules ?? [])}`,
    "---",
    "",
    `# ${input.title}`,
    "",
    input.content.trim().length > 0 ? input.content.trim() : "正文从这里开始。",
    ""
  ];

  return frontmatter.join("\n");
}

export async function writeFileAtomic(targetPath: string, content: string): Promise<void> {
  const tempPath = `${targetPath}.tmp`;
  await fs.writeFile(tempPath, content, "utf-8");
  try {
    await fs.rename(tempPath, targetPath);
  } catch {
    // Windows rename may fail when target exists.
    await fs.rm(targetPath, { force: true });
    await fs.rename(tempPath, targetPath);
  }
}

export async function readTextIfExists(filePath: string): Promise<string | undefined> {
  try {
    return await fs.readFile(filePath, "utf-8");
  } catch {
    return undefined;
  }
}

export function chapterPath(projectRoot: string, relativePath: string): string {
  return path.join(projectRoot, relativePath);
}
