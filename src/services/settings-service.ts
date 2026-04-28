import { AppError } from "../errors/app-error.js";
import type { ProviderConfigInput } from "../domain/types.js";
import {
  deleteProjectApiKey,
  loadProjectApiKey,
  saveProjectApiKey,
} from "../infra/secret-store.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";
import { getProjectId } from "./service-utils.js";

const CONFIG_KEY = "ai.provider_config";

export class SettingsService {
  public async saveProviderConfig(projectRoot: string, config: ProviderConfigInput): Promise<void> {
    await withDatabase(projectRoot, async (db) => {
      const projectId = getProjectId(db);
      const now = nowIso();
      const payload = {
        providerName: config.providerName,
        baseUrl: config.baseUrl,
        model: config.model,
        temperature: config.temperature,
        maxTokens: config.maxTokens,
        stream: config.stream
      };
      db.prepare(
        `
        INSERT INTO settings(key, value, updated_at) VALUES(?, ?, ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
        `
      ).run(CONFIG_KEY, JSON.stringify(payload), now);

      if (typeof config.apiKey === "string") {
        const trimmed = config.apiKey.trim();
        if (trimmed.length > 0) {
          await saveProjectApiKey(projectId, trimmed);
        } else {
          await deleteProjectApiKey(projectId);
        }
      }
    });
  }

  public async getProviderConfig(projectRoot: string): Promise<Omit<ProviderConfigInput, "apiKey"> & { hasApiKey: boolean }> {
    return withDatabase(projectRoot, async (db) => {
      const projectId = getProjectId(db);
      const row = db.prepare("SELECT value FROM settings WHERE key = ?").get(CONFIG_KEY) as
        | { value: string }
        | undefined;
      if (!row) {
        throw new AppError({
          code: "AI_PROVIDER_NOT_CONFIGURED",
          message: "未配置模型",
          recoverable: true,
          suggestedAction: "请先在设置页填写 Provider、Base URL、Model 和 API Key"
        });
      }
      const parsed = JSON.parse(row.value) as Omit<ProviderConfigInput, "apiKey">;
      const apiKey = await loadProjectApiKey(projectId);
      return {
        ...parsed,
        hasApiKey: Boolean(apiKey && apiKey.length > 0)
      };
    });
  }
}
