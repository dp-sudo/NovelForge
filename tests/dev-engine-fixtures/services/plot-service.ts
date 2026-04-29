import { randomUUID } from "node:crypto";

import type { PlotNodeInput } from "../../../src/domain/types.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";
import { getProjectId, parseJsonList } from "./service-utils.js";

const STATUS_MAP: Record<NonNullable<PlotNodeInput["status"]>, string> = {
  未使用: "unused",
  规划中: "planned",
  已写入: "written",
  需调整: "adjust_required"
};

export class PlotService {
  public async create(projectRoot: string, input: PlotNodeInput): Promise<string> {
    return withDatabase(projectRoot, (db) => {
      const id = randomUUID();
      const now = nowIso();
      db.prepare(
        `
        INSERT INTO plot_nodes(
          id, project_id, title, node_type, sort_order, goal, conflict, emotional_curve, status, related_characters, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `
      ).run(
        id,
        getProjectId(db),
        input.title,
        input.nodeType,
        input.sortOrder,
        input.goal ?? null,
        input.conflict ?? null,
        input.emotionalCurve ?? null,
        input.status ? STATUS_MAP[input.status] : "planned",
        JSON.stringify(input.relatedCharacters ?? []),
        now,
        now
      );
      return id;
    });
  }

  public async list(projectRoot: string): Promise<Array<Record<string, unknown>>> {
    return withDatabase(projectRoot, (db) => {
      const rows = db
        .prepare(
          `
          SELECT *
          FROM plot_nodes
          WHERE project_id = ?
          ORDER BY sort_order, created_at
          `
        )
        .all(getProjectId(db)) as Array<Record<string, unknown>>;
      return rows.map((row) => ({
        ...row,
        related_characters: parseJsonList(row.related_characters)
      }));
    });
  }

  public async reorder(projectRoot: string, orderedIds: string[]): Promise<void> {
    await withDatabase(projectRoot, (db) => {
      db.exec("BEGIN");
      try {
        for (let index = 0; index < orderedIds.length; index += 1) {
          db.prepare("UPDATE plot_nodes SET sort_order = ?, updated_at = ? WHERE id = ?").run(
            index + 1,
            nowIso(),
            orderedIds[index]
          );
        }
        db.exec("COMMIT");
      } catch (error) {
        db.exec("ROLLBACK");
        throw error;
      }
    });
  }
}
