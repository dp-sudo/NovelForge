import { randomUUID } from "node:crypto";

import type { GlossaryTermInput } from "../domain/types.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";
import { getProjectId, parseJsonList } from "./service-utils.js";

export class GlossaryService {
  public async create(projectRoot: string, input: GlossaryTermInput): Promise<string> {
    return withDatabase(projectRoot, (db) => {
      const id = randomUUID();
      const now = nowIso();
      db.prepare(
        `
        INSERT INTO glossary_terms(
          id, project_id, term, term_type, aliases, description, locked, banned, preferred_usage, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `
      ).run(
        id,
        getProjectId(db),
        input.term,
        input.termType,
        JSON.stringify(input.aliases ?? []),
        input.description ?? null,
        input.locked ? 1 : 0,
        input.banned ? 1 : 0,
        input.preferredUsage ?? null,
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
          FROM glossary_terms
          WHERE project_id = ?
          ORDER BY term
          `
        )
        .all(getProjectId(db)) as Array<Record<string, unknown>>;
      return rows.map((row) => ({
        ...row,
        aliases: parseJsonList(row.aliases),
        locked: Number(row.locked) === 1,
        banned: Number(row.banned) === 1
      }));
    });
  }
}
