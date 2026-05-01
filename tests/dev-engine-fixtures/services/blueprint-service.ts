import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";

import { BLUEPRINT_STEP_KEYS, type BlueprintStepKey } from "../../../src/domain/constants.js";
import type { BlueprintStep } from "../../../src/domain/types.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";

function parseCertaintyZones(raw: unknown): BlueprintStep["certaintyZones"] {
  if (typeof raw !== "string" || raw.trim().length === 0) {
    return undefined;
  }
  try {
    const parsed = JSON.parse(raw) as {
      frozen?: unknown;
      promised?: unknown;
      exploratory?: unknown;
    };
    const toList = (value: unknown): string[] =>
      Array.isArray(value)
        ? value
            .filter((item): item is string => typeof item === "string")
            .map((item) => item.trim())
            .filter((item) => item.length > 0)
        : [];
    const certaintyZones = {
      frozen: toList(parsed.frozen),
      promised: toList(parsed.promised),
      exploratory: toList(parsed.exploratory),
    };
    const hasAny =
      certaintyZones.frozen.length > 0 ||
      certaintyZones.promised.length > 0 ||
      certaintyZones.exploratory.length > 0;
    return hasAny ? certaintyZones : undefined;
  } catch {
    return undefined;
  }
}

function stepTitle(stepKey: BlueprintStepKey): string {
  switch (stepKey) {
    case "step-01-anchor":
      return "灵感定锚";
    case "step-02-genre":
      return "类型策略";
    case "step-03-premise":
      return "故事母题";
    case "step-04-characters":
      return "角色工坊";
    case "step-05-world":
      return "世界规则";
    case "step-06-glossary":
      return "名词锁定";
    case "step-07-plot":
      return "剧情骨架";
    case "step-08-chapters":
      return "章节路线";
  }
}

export class BlueprintService {
  public async listSteps(projectRoot: string): Promise<BlueprintStep[]> {
    return withDatabase(projectRoot, (db) => {
      const projectRow = db.prepare("SELECT id FROM projects LIMIT 1").get() as { id: string };
      const rows = db
        .prepare(
          `
          SELECT id, project_id, step_key, title, content, content_path, status, ai_generated, completed_at, created_at, updated_at
               , certainty_zones_json
          FROM blueprint_steps
          WHERE project_id = ?
          ORDER BY step_key
          `
        )
        .all(projectRow.id) as Array<Record<string, unknown>>;

      return BLUEPRINT_STEP_KEYS.map((stepKey) => {
        const row = rows.find((item) => item.step_key === stepKey);
        if (!row) {
          return {
            id: "",
            projectId: projectRow.id,
            stepKey,
            title: stepTitle(stepKey),
            content: "",
            contentPath: `blueprint/${stepKey}.md`,
            status: "not_started",
            aiGenerated: false,
            createdAt: "",
            updatedAt: ""
          } satisfies BlueprintStep;
        }
        return {
          id: row.id as string,
          projectId: row.project_id as string,
          stepKey: row.step_key as BlueprintStepKey,
          title: row.title as string,
          content: (row.content as string) ?? "",
          contentPath: row.content_path as string,
          status: row.status as BlueprintStep["status"],
          aiGenerated: Number(row.ai_generated) === 1,
          certaintyZones: parseCertaintyZones(row.certainty_zones_json),
          completedAt: (row.completed_at as string | null) ?? undefined,
          createdAt: row.created_at as string,
          updatedAt: row.updated_at as string
        };
      });
    });
  }

  public async saveStep(
    projectRoot: string,
    stepKey: BlueprintStepKey,
    content: string,
    aiGenerated = false
  ): Promise<void> {
    const now = nowIso();
    const contentPath = path.join(projectRoot, "blueprint", `${stepKey}.md`);
    await fs.writeFile(contentPath, content, "utf-8");

    await withDatabase(projectRoot, (db) => {
      const projectRow = db.prepare("SELECT id FROM projects LIMIT 1").get() as { id: string };
      db.prepare(
        `
        INSERT INTO blueprint_steps (
          id, project_id, step_key, title, content, content_path, status, ai_generated, completed_at, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(project_id, step_key) DO UPDATE SET
          title = excluded.title,
          content = excluded.content,
          content_path = excluded.content_path,
          status = excluded.status,
          ai_generated = excluded.ai_generated,
          updated_at = excluded.updated_at
        `
      ).run(
        randomUUID(),
        projectRow.id,
        stepKey,
        stepTitle(stepKey),
        content,
        `blueprint/${stepKey}.md`,
        content.trim().length > 0 ? "in_progress" : "not_started",
        aiGenerated ? 1 : 0,
        null,
        now,
        now
      );
    });
  }

  public async markCompleted(projectRoot: string, stepKey: BlueprintStepKey): Promise<void> {
    await withDatabase(projectRoot, (db) => {
      const now = nowIso();
      const projectRow = db.prepare("SELECT id FROM projects LIMIT 1").get() as { id: string };
      db.prepare(
        `
        UPDATE blueprint_steps
        SET status = 'completed', completed_at = ?, updated_at = ?
        WHERE project_id = ? AND step_key = ?
        `
      ).run(now, now, projectRow.id, stepKey);
    });
  }

  public async resetStep(projectRoot: string, stepKey: BlueprintStepKey): Promise<void> {
    const contentPath = path.join(projectRoot, "blueprint", `${stepKey}.md`);
    await fs.writeFile(contentPath, "", "utf-8");

    await withDatabase(projectRoot, (db) => {
      const now = nowIso();
      const projectRow = db.prepare("SELECT id FROM projects LIMIT 1").get() as { id: string };
      db.prepare(
        `
        UPDATE blueprint_steps
        SET content = '', status = 'not_started', completed_at = NULL, ai_generated = 0, updated_at = ?
        WHERE project_id = ? AND step_key = ?
        `
      ).run(now, projectRow.id, stepKey);
    });
  }
}
