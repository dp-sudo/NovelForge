import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";

import { AppError } from "../../../src/errors/app-error.js";
import { initializeDatabase, openDatabase } from "../infra/db.js";
import { appendProjectLog } from "../infra/logger.js";
import { sanitizeProjectDirectoryName } from "../infra/path-utils.js";
import { initializeProjectDirectories, ensurePathExists } from "../infra/project-layout.js";
import { buildProjectJson, readProjectJson, writeProjectJson } from "../infra/project-json.js";
import { listRecentProjects, markRecentProject } from "../infra/recent-projects.js";
import { nowIso } from "../infra/time.js";
import type { CreateProjectInput, ProjectJson } from "../../../src/domain/types.js";

export interface ProjectOpenResult {
  projectRoot: string;
  project: ProjectJson;
}

export class ProjectService {
  public async createProject(input: CreateProjectInput): Promise<ProjectOpenResult> {
    if (input.name.trim().length < 1 || input.name.trim().length > 80) {
      throw new AppError({
        code: "PROJECT_NAME_INVALID",
        message: "作品名称必须在 1-80 字之间",
        recoverable: true
      });
    }

    const sanitizedDirectoryName = sanitizeProjectDirectoryName(input.name);
    const projectRoot = path.join(input.saveDirectory, sanitizedDirectoryName);
    const targetWords = input.targetWords ?? 300000;
    let createdProjectRoot = false;

    try {
      await fs.mkdir(projectRoot, { recursive: false });
      createdProjectRoot = true;
      await initializeProjectDirectories(projectRoot);
      await initializeDatabase(projectRoot);

      const projectJson = buildProjectJson({
        projectId: randomUUID(),
        name: input.name.trim(),
        author: input.author,
        genre: input.genre,
        targetWords
      });
      await writeProjectJson(projectRoot, projectJson);

      const db = openDatabase(projectRoot);
      try {
        const now = nowIso();
        db.prepare(
          `
          INSERT INTO projects(id, name, author, genre, target_words, current_words, project_path, schema_version, created_at, updated_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          `
        ).run(
          projectJson.projectId,
          projectJson.name,
          projectJson.author,
          projectJson.genre,
          projectJson.targetWords,
          0,
          projectRoot,
          projectJson.schemaVersion,
          now,
          now
        );
      } finally {
        db.close();
      }

      await markRecentProject(projectRoot);
      await appendProjectLog(projectRoot, "PROJECT_CREATED");
      return { projectRoot, project: projectJson };
    } catch (error) {
      if (createdProjectRoot) {
        await fs.rm(projectRoot, { recursive: true, force: true });
      }
      throw new AppError({
        code: "PROJECT_CREATE_FAILED",
        message: "创建项目失败",
        detail: error instanceof Error ? error.message : String(error),
        recoverable: true,
        suggestedAction: "请检查目标目录权限或更换保存路径"
      });
    }
  }

  public async openProject(projectRoot: string): Promise<ProjectOpenResult> {
    const projectJsonPath = path.join(projectRoot, "project.json");
    const dbPath = path.join(projectRoot, "database", "project.sqlite");

    if (!(await ensurePathExists(projectJsonPath)) || !(await ensurePathExists(dbPath))) {
      throw new AppError({
        code: "PROJECT_INVALID_PATH",
        message: "不是有效项目目录",
        recoverable: true,
        suggestedAction: "请选择包含 project.json 和 database/project.sqlite 的目录"
      });
    }

    const project = await readProjectJson(projectRoot);
    if (project.schemaVersion !== "1.0.0") {
      throw new AppError({
        code: "PROJECT_VERSION_UNSUPPORTED",
        message: "项目版本不兼容",
        detail: `schemaVersion=${project.schemaVersion}`,
        recoverable: false,
        suggestedAction: "请先执行项目迁移"
      });
    }

    const db = openDatabase(projectRoot);
    db.close();

    await markRecentProject(projectRoot);
    await appendProjectLog(projectRoot, "PROJECT_OPENED");
    return { projectRoot, project };
  }

  public async listRecentProjects(): Promise<Awaited<ReturnType<typeof listRecentProjects>>> {
    return listRecentProjects();
  }
}
