import { DatabaseSync } from "node:sqlite";

import { openDatabase } from "../infra/db.js";

export async function withDatabase<T>(
  projectRoot: string,
  run: (db: DatabaseSync) => Promise<T> | T
): Promise<T> {
  const db = openDatabase(projectRoot);
  try {
    return await run(db);
  } finally {
    db.close();
  }
}
