import { AppError } from "../errors/app-error.js";

export interface OpenAiCompatibleRequest {
  baseUrl: string;
  apiKey: string;
  model: string;
  temperature: number;
  maxTokens: number;
  prompt: string;
}

export interface OpenAiCompatibleResponse {
  text: string;
  promptTokens?: number;
  completionTokens?: number;
  totalTokens?: number;
}

export type FetchLike = typeof fetch;

export async function callOpenAiCompatible(
  input: OpenAiCompatibleRequest,
  fetchImpl: FetchLike = fetch
): Promise<OpenAiCompatibleResponse> {
  const endpoint = `${input.baseUrl.replace(/\/+$/, "")}/chat/completions`;
  const response = await fetchImpl(endpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${input.apiKey}`
    },
    body: JSON.stringify({
      model: input.model,
      temperature: input.temperature,
      max_tokens: input.maxTokens,
      stream: false,
      messages: [
        {
          role: "user",
          content: input.prompt
        }
      ]
    })
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new AppError({
      code: "AI_REQUEST_FAILED",
      message: "AI 请求失败",
      detail: `${response.status} ${detail}`,
      recoverable: true
    });
  }

  const parsed = (await response.json()) as {
    choices?: Array<{ message?: { content?: string } }>;
    usage?: { prompt_tokens?: number; completion_tokens?: number; total_tokens?: number };
  };
  const text = parsed.choices?.[0]?.message?.content?.trim() ?? "";
  if (text.length === 0) {
    throw new AppError({
      code: "AI_EMPTY_RESPONSE",
      message: "AI 返回空内容",
      recoverable: true
    });
  }
  return {
    text,
    promptTokens: parsed.usage?.prompt_tokens,
    completionTokens: parsed.usage?.completion_tokens,
    totalTokens: parsed.usage?.total_tokens
  };
}
