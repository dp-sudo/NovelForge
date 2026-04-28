import fs from "node:fs/promises";
import path from "node:path";

import { PROJECT_APP_MIN_VERSION, PROJECT_SCHEMA_VERSION } from "../domain/constants.js";
import type { ProjectJson } from "../domain/types.js";
import { nowIso } from "./time.js";

export function buildProjectJson(input: {
  projectId: string;
  name: string;
  author?: string;
  genre: string;
  targetWords: number;
}): ProjectJson {
  const now = nowIso();
  return {
    schemaVersion: PROJECT_SCHEMA_VERSION,
    appMinVersion: PROJECT_APP_MIN_VERSION,
    projectId: input.projectId,
    name: input.name,
    author: input.author ?? "",
    genre: input.genre,
    targetWords: input.targetWords,
    createdAt: now,
    updatedAt: now,
    database: "database/project.sqlite",
    manuscriptRoot: "manuscript/chapters",
    settings: {
      defaultNarrativePov: "third_limited",
      language: "zh-CN",
      autosaveIntervalMs: 5000
    }
  };
}

export async function writeProjectJson(projectRoot: string, projectJson: ProjectJson): Promise<void> {
  const filePath = path.join(projectRoot, "project.json");
  await fs.writeFile(filePath, JSON.stringify(projectJson, null, 2), "utf-8");
}

export async function readProjectJson(projectRoot: string): Promise<ProjectJson> {
  const filePath = path.join(projectRoot, "project.json");
  const raw = await fs.readFile(filePath, "utf-8");
  return JSON.parse(raw) as ProjectJson;
}
