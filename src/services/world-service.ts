import { randomUUID } from "node:crypto";

import type { WorldRuleInput } from "../domain/types.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";
import { getProjectId, parseJsonList } from "./service-utils.js";

export class WorldService {
  public async create(projectRoot: string, input: WorldRuleInput): Promise<string> {
    return withDatabase(projectRoot, (db) => {
      const id = randomUUID();
      const now = nowIso();
      db.prepare(
        `
        INSERT INTO world_rules(
          id, project_id, title, category, description, constraint_level, related_entities, examples, contradiction_policy, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `
      ).run(
        id,
        getProjectId(db),
        input.title,
        input.category,
        input.description,
        input.constraintLevel,
        JSON.stringify(input.relatedEntities ?? []),
        input.examples ?? null,
        input.contradictionPolicy ?? null,
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
          FROM world_rules
          WHERE project_id = ? AND is_deleted = 0
          ORDER BY updated_at DESC
          `
        )
        .all(getProjectId(db)) as Array<Record<string, unknown>>;
      return rows.map((row) => ({
        ...row,
        related_entities: parseJsonList(row.related_entities)
      }));
    });
  }

  public async softDelete(projectRoot: string, id: string): Promise<void> {
    await withDatabase(projectRoot, (db) => {
      db.prepare("UPDATE world_rules SET is_deleted = 1, updated_at = ? WHERE id = ?").run(nowIso(), id);
    });
  }
}
