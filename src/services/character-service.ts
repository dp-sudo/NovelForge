import { randomUUID } from "node:crypto";

import { AppError } from "../errors/app-error.js";
import type { CharacterInput } from "../domain/types.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";
import { getProjectId, parseJsonList } from "./service-utils.js";

export class CharacterService {
  public async create(projectRoot: string, input: CharacterInput): Promise<string> {
    return withDatabase(projectRoot, (db) => {
      const id = randomUUID();
      const now = nowIso();
      db.prepare(
        `
        INSERT INTO characters(
          id, project_id, name, aliases, role_type, age, gender, identity_text, appearance,
          motivation, desire, fear, flaw, arc_stage, locked_fields, notes, created_at, updated_at
        )
        VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `
      ).run(
        id,
        getProjectId(db),
        input.name,
        JSON.stringify(input.aliases ?? []),
        input.roleType,
        input.age ?? null,
        input.gender ?? null,
        input.identityText ?? null,
        input.appearance ?? null,
        input.motivation ?? null,
        input.desire ?? null,
        input.fear ?? null,
        input.flaw ?? null,
        input.arcStage ?? null,
        JSON.stringify(input.lockedFields ?? []),
        input.notes ?? null,
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
          FROM characters
          WHERE project_id = ? AND is_deleted = 0
          ORDER BY updated_at DESC
          `
        )
        .all(getProjectId(db)) as Array<Record<string, unknown>>;
      return rows.map((row) => ({
        ...row,
        aliases: parseJsonList(row.aliases),
        locked_fields: parseJsonList(row.locked_fields)
      }));
    });
  }

  public async update(projectRoot: string, id: string, input: Partial<CharacterInput>): Promise<void> {
    await withDatabase(projectRoot, (db) => {
      const current = db.prepare("SELECT * FROM characters WHERE id = ?").get(id) as Record<
        string,
        unknown
      >;
      if (!current) {
        throw new AppError({
          code: "CHARACTER_NOT_FOUND",
          message: "角色不存在",
          recoverable: true
        });
      }
      db.prepare(
        `
        UPDATE characters
        SET
          name = ?,
          aliases = ?,
          role_type = ?,
          age = ?,
          gender = ?,
          identity_text = ?,
          appearance = ?,
          motivation = ?,
          desire = ?,
          fear = ?,
          flaw = ?,
          arc_stage = ?,
          locked_fields = ?,
          notes = ?,
          updated_at = ?
        WHERE id = ?
        `
      ).run(
        input.name ?? String(current.name ?? ""),
        JSON.stringify(input.aliases ?? parseJsonList(current.aliases)),
        input.roleType ?? String(current.role_type ?? "配角"),
        input.age ?? (current.age as string | null),
        input.gender ?? (current.gender as string | null),
        input.identityText ?? (current.identity_text as string | null),
        input.appearance ?? (current.appearance as string | null),
        input.motivation ?? (current.motivation as string | null),
        input.desire ?? (current.desire as string | null),
        input.fear ?? (current.fear as string | null),
        input.flaw ?? (current.flaw as string | null),
        input.arcStage ?? (current.arc_stage as string | null),
        JSON.stringify(input.lockedFields ?? parseJsonList(current.locked_fields)),
        input.notes ?? (current.notes as string | null),
        nowIso(),
        id
      );
    });
  }

  public async softDelete(projectRoot: string, id: string): Promise<void> {
    await withDatabase(projectRoot, (db) => {
      const references = db
        .prepare(
          `
          SELECT chapter_id
          FROM chapter_links
          WHERE project_id = ? AND target_type = 'character' AND target_id = ?
          `
        )
        .all(getProjectId(db), id) as Array<{ chapter_id: string }>;

      if (references.length > 0) {
        throw new AppError({
          code: "CHARACTER_REFERENCED",
          message: "角色已被章节引用，删除前需要确认风险",
          detail: `chapterCount=${references.length}`,
          recoverable: true,
          suggestedAction: "请先解除章节关联，或在 UI 中二次确认后删除"
        });
      }

      db.prepare("UPDATE characters SET is_deleted = 1, updated_at = ? WHERE id = ?").run(nowIso(), id);
    });
  }
}
