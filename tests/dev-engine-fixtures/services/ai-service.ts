import { randomUUID } from "node:crypto";

import { callOpenAiCompatible, type FetchLike } from "../../../src/adapters/openai-compatible-adapter.js";
import { AppError } from "../../../src/errors/app-error.js";
import type { AiPreviewRequest, AiPreviewResponse } from "../../../src/domain/types.js";
import { appendProjectLog } from "../infra/logger.js";
import { loadProjectApiKey } from "../infra/secret-store.js";
import { nowIso } from "../infra/time.js";
import { withDatabase } from "./service-context.js";
import { ContextService } from "./context-service.js";
import { getProjectId } from "./service-utils.js";

interface RuntimeProviderConfig {
  providerName: string;
  baseUrl: string;
  model: string;
  temperature: number;
  maxTokens: number;
  stream: boolean;
}

function buildPrompt(input: {
  request: AiPreviewRequest;
  context: Awaited<ReturnType<ContextService["collectForChapter"]>>;
}): string {
  return [
    "# 角色",
    "你是文火 NovelForge 的小说创作助手。",
    "",
    "# 任务",
    `${input.request.taskType}`,
    "",
    "# 项目上下文",
    JSON.stringify(input.context.globalContext, null, 2),
    "",
    "# 当前相关上下文",
    JSON.stringify(input.context.relatedContext, null, 2),
    "",
    "# 用户输入",
    input.request.userInstruction,
    "",
    "# 约束",
    "1. 不得输出解释性前缀（如：以下是生成内容）。",
    "2. 不得改动用户未授权部分。",
    "3. 输出只返回正文。",
    ""
  ].join("\n");
}

function mockResponse(taskType: AiPreviewRequest["taskType"], instruction: string): string {
  if (taskType === "scan_consistency") {
    return JSON.stringify(
      {
        issues: [
          {
            issueType: "prose_style",
            severity: "low",
            sourceText: "命运的齿轮开始转动",
            explanation: "存在典型套话表达",
            suggestedFix: "改为具体动作或感官描写",
            relatedAsset: "chapter"
          }
        ]
      },
      null,
      2
    );
  }
  return `【AI预览草稿】\n${instruction}\n\n他推开门，雨声骤然压进屋里。`;
}

export class AiService {
  private readonly contextService: ContextService;
  private readonly fetchImpl: FetchLike;

  public constructor(input?: { contextService?: ContextService; fetchImpl?: FetchLike }) {
    this.contextService = input?.contextService ?? new ContextService();
    this.fetchImpl = input?.fetchImpl ?? fetch;
  }

  public async generatePreview(
    projectRoot: string,
    request: AiPreviewRequest
  ): Promise<AiPreviewResponse> {
    if (!request.chapterId) {
      throw new AppError({
        code: "AI_CHAPTER_REQUIRED",
        message: "AI 请求缺少 chapterId",
        recoverable: true
      });
    }

    const requestId = randomUUID();
    const startedAt = nowIso();
    const context = await this.contextService.collectForChapter(
      projectRoot,
      request.chapterId,
      request.userInstruction
    );

    return withDatabase(projectRoot, async (db) => {
      const projectId = getProjectId(db);
      const provider = db.prepare("SELECT value FROM settings WHERE key = ?").get(
        "ai.provider_config"
      ) as { value: string } | undefined;
      if (!provider) {
        throw new AppError({
          code: "AI_PROVIDER_NOT_CONFIGURED",
          message: "未配置模型",
          recoverable: true
        });
      }
      const config = JSON.parse(provider.value) as RuntimeProviderConfig;
      const apiKey = await loadProjectApiKey(projectId);
      if (!apiKey && !config.baseUrl.startsWith("mock://")) {
        throw new AppError({
          code: "AI_PROVIDER_NOT_CONFIGURED",
          message: "未配置 API Key",
          recoverable: true
        });
      }

      const prompt = buildPrompt({ request, context });
      const previewSummary = prompt.slice(0, 240);
      db.prepare(
        `
        INSERT INTO ai_requests(id, project_id, task_type, provider, model, prompt_preview, status, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        `
      ).run(requestId, projectId, request.taskType, config.providerName, config.model, previewSummary, "running", startedAt);

      try {
        let preview: string;
        if (config.baseUrl.startsWith("mock://")) {
          preview = mockResponse(request.taskType, request.userInstruction);
        } else {
          const response = await callOpenAiCompatible(
            {
              baseUrl: config.baseUrl,
              apiKey: apiKey ?? "",
              model: config.model,
              temperature: config.temperature,
              maxTokens: config.maxTokens,
              prompt
            },
            this.fetchImpl
          );
          preview = response.text;
        }

        db.prepare(
          `
          UPDATE ai_requests
          SET status = 'done', completed_at = ?
          WHERE id = ?
          `
        ).run(nowIso(), requestId);

        await appendProjectLog(
          projectRoot,
          `AI_REQUEST_DONE requestId=${requestId} provider=${config.providerName} model=${config.model}`
        );

        return {
          requestId,
          preview,
          usedContext: context.usedContext,
          risks: []
        };
      } catch (error) {
        const appError = error instanceof AppError ? error : new AppError({
          code: "AI_REQUEST_FAILED",
          message: "AI 请求失败",
          detail: error instanceof Error ? error.message : String(error),
          recoverable: true
        });
        db.prepare(
          `
          UPDATE ai_requests
          SET status = 'error', error_code = ?, error_message = ?, completed_at = ?
          WHERE id = ?
          `
        ).run(appError.code, appError.message, nowIso(), requestId);
        await appendProjectLog(
          projectRoot,
          `AI_REQUEST_ERROR requestId=${requestId} code=${appError.code} message=${appError.message}`
        );
        throw appError;
      }
    });
  }
}
