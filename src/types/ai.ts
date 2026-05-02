// ── LlmProviderConfig (matches Rust adapters::llm_types::ProviderConfig) ──

export interface LlmProviderConfig {
  id: string;
  displayName: string;
  vendor: string;
  protocol: string;
  baseUrl: string;
  endpointPath?: string;
  apiKey?: string;
  authMode: string;
  authHeaderName?: string;
  anthropicVersion?: string;
  betaHeaders?: Record<string, string>;
  customHeaders?: Record<string, string>;
  defaultModel?: string;
  timeoutMs: number;
  connectTimeoutMs: number;
  maxRetries: number;
  modelRefreshMode?: string;
  modelsPath?: string;
  lastModelRefreshAt?: string;
}

// ── LlmModelConfig (spec §3.2) ──

export interface LlmModelConfig {
  id: string;
  providerId: string;
  modelName: string;
  displayName: string;
  contextWindowTokens: number;
  maxOutputTokens: number | null;
  supportsStreaming: boolean;
  supportsTools: boolean;
  supportsJsonObject: boolean;
  supportsJsonSchema: boolean;
  supportsThinking: boolean;
  supportsReasoningEffort: boolean;
  supportsPromptCache: boolean;
  status: string;
  recommendedFor: string[];
}

// ── UnifiedGenerateRequest (spec §3.3) ──

export interface UnifiedGenerateRequest {
  providerId: string;
  model: string;
  taskType?: string;
  systemPrompt?: string;
  messages: UnifiedMessage[];
  responseFormat?: "text" | "json_object" | "json_schema";
  stream?: boolean;
  temperature?: number;
  topP?: number;
  maxOutputTokens?: number;
}

export interface UnifiedMessage {
  role: string;
  content: string;
}

// ── UnifiedGenerateResponse (spec §3.4) ──

export interface UnifiedGenerateResponse {
  requestId: string;
  providerId: string;
  model: string;
  text?: string;
  finishReason?: string;
  usage?: {
    inputTokens?: number;
    outputTokens?: number;
    totalTokens?: number;
  };
}

// ── LlmError (spec §20) ──

export type LlmErrorCode =
  | "missing_api_key"
  | "invalid_api_key"
  | "insufficient_quota"
  | "rate_limited"
  | "model_not_found"
  | "context_length_exceeded"
  | "max_output_exceeded"
  | "content_policy_violation"
  | "network_timeout"
  | "stream_interrupted"
  | "invalid_json_response"
  | "unsupported_feature"
  | "unknown";

// ── TaskRoute ──

export interface TaskRoute {
  id: string;
  taskType: string;
  providerId: string;
  modelId: string;
  fallbackProviderId?: string;
  fallbackModelId?: string;
  postTasks?: string[];
  maxRetries: number;
  createdAt?: string;
  updatedAt?: string;
}

export interface PromotionPolicy {
  id: string;
  targetType: string;
  sourceKind: string;
  policyMode: string;
  requireReason: boolean;
  enabled: boolean;
  notes?: string;
  createdAt?: string;
  updatedAt?: string;
}

export interface WritingStyle {
  languageStyle: "plain" | "balanced" | "ornate" | "colloquial";
  descriptionDensity: number;
  dialogueRatio: number;
  sentenceRhythm: "short" | "long" | "mixed";
  atmosphere: "warm" | "cold" | "humorous" | "serious" | "suspenseful" | "neutral";
  psychologicalDepth: number;
}

export function defaultWritingStyle(): WritingStyle {
  return {
    languageStyle: "balanced",
    descriptionDensity: 4,
    dialogueRatio: 4,
    sentenceRhythm: "mixed",
    atmosphere: "neutral",
    psychologicalDepth: 4,
  };
}

export interface AiStrategyProfile {
  automationDefault: "auto" | "supervised" | "confirm";
  reviewStrictness: number;
  defaultWorkflowStack: string[];
  alwaysOnPolicySkills: string[];
  defaultCapabilityBundles: string[];
  stateWritePolicy: "chapter_confirmed" | "manual_only";
  continuityPackDepth: "minimal" | "standard" | "deep";
  enforceContextCompleteness: boolean;
  chapterGenerationMode: "draft_only" | "plan_draft" | "plan_scene_draft";
  windowPlanningHorizon: number;
}

export function defaultAiStrategyProfile(): AiStrategyProfile {
  return {
    automationDefault: "supervised",
    reviewStrictness: 4,
    defaultWorkflowStack: ["chapter.plan", "chapter.draft"],
    alwaysOnPolicySkills: ["consistency.scan"],
    defaultCapabilityBundles: [
      "bundle.character-expression",
      "bundle.emotion-progression",
      "bundle.scene-environment",
      "bundle.rule-fulfillment",
    ],
    stateWritePolicy: "chapter_confirmed",
    continuityPackDepth: "standard",
    enforceContextCompleteness: true,
    chapterGenerationMode: "plan_scene_draft",
    windowPlanningHorizon: 10,
  };
}

// ── CapabilityReport ──

export interface CapabilityReport {
  providerId: string;
  textResponse: boolean;
  streaming: boolean;
  jsonObject: boolean;
  jsonSchema: boolean;
  tools: boolean;
  thinking: boolean;
  error: string | null;
}

// ── ModelRecord ──

export interface ModelRecord {
  id: string;
  providerId: string;
  modelName: string;
  displayName: string | null;
  contextWindowTokens: number | null;
  maxOutputTokens: number | null;
  supportsStreaming: boolean;
  supportsTools: boolean;
  supportsJsonObject: boolean;
  supportsJsonSchema: boolean;
  supportsThinking: boolean;
  supportsReasoningEffort: boolean;
  status: string;
  source?: string | null;
  userOverridden?: boolean;
  lastSeenAt?: string | null;
  registryVersion?: string | null;
}

// ── RefreshLog ──

export interface RefreshLog {
  id: string;
  providerId: string;
  refreshType: string;
  status: string;
  modelsAdded: number;
  modelsUpdated: number;
  modelsRemoved: number;
  errorMessage: string | null;
  createdAt: string;
}

// ── RefreshResult ──

export interface RefreshResult {
  added: number;
  updated: number;
  removed: number;
  capabilities: CapabilityReport;
}

// ── Built-in vendor info (display metadata) ──

export interface VendorInfo {
  id: string;
  displayName: string;
  vendor: string;
  defaultBaseUrl: string;
  defaultProtocol: string;
  defaultModel: string;
  apiKeyPlaceholder: string;
  supports: ("streaming" | "tools" | "thinking" | "json" | "vision")[];
}

export const VENDOR_PRESETS: VendorInfo[] = [
  {
    id: "deepseek", displayName: "DeepSeek", vendor: "deepseek",
    defaultBaseUrl: "https://api.deepseek.com", defaultProtocol: "openai_chat_completions",
    defaultModel: "deepseek-v4-flash", apiKeyPlaceholder: "sk-...",
    supports: ["streaming", "tools", "thinking", "json"]
  },
  {
    id: "kimi", displayName: "Kimi (Moonshot)", vendor: "kimi",
    defaultBaseUrl: "https://api.moonshot.ai/v1", defaultProtocol: "openai_chat_completions",
    defaultModel: "kimi-k2.6", apiKeyPlaceholder: "sk-moonshot-...",
    supports: ["streaming", "tools", "thinking"]
  },
  {
    id: "zhipu", displayName: "智谱 GLM", vendor: "zhipu",
    defaultBaseUrl: "https://open.bigmodel.cn/api/paas/v4", defaultProtocol: "openai_chat_completions",
    defaultModel: "glm-5.1", apiKeyPlaceholder: "zhipu-...",
    supports: ["streaming", "tools", "thinking", "json"]
  },
  {
    id: "minimax", displayName: "MiniMax", vendor: "minimax",
    defaultBaseUrl: "https://api.minimax.io/anthropic", defaultProtocol: "anthropic_messages",
    defaultModel: "MiniMax-M2.7", apiKeyPlaceholder: "minimax-...",
    supports: ["streaming", "tools", "thinking"]
  },
  {
    id: "openai", displayName: "OpenAI", vendor: "openai",
    defaultBaseUrl: "https://api.openai.com/v1", defaultProtocol: "openai_chat_completions",
    defaultModel: "gpt-5.5", apiKeyPlaceholder: "sk-proj-...",
    supports: ["streaming", "tools", "thinking", "json", "vision"]
  },
  {
    id: "anthropic", displayName: "Anthropic Claude", vendor: "anthropic",
    defaultBaseUrl: "https://api.anthropic.com/v1", defaultProtocol: "anthropic_messages",
    defaultModel: "claude-sonnet-4-6", apiKeyPlaceholder: "sk-ant-...",
    supports: ["streaming", "tools", "thinking"]
  },
  {
    id: "gemini", displayName: "Google Gemini", vendor: "gemini",
    defaultBaseUrl: "https://generativelanguage.googleapis.com/v1beta", defaultProtocol: "gemini_generate_content",
    defaultModel: "gemini-3.1-pro-preview", apiKeyPlaceholder: "AIza...",
    supports: ["streaming", "tools", "thinking", "json", "vision"]
  },
  {
    id: "custom", displayName: "自定义 Provider", vendor: "custom",
    defaultBaseUrl: "http://localhost:8000/v1", defaultProtocol: "custom_openai_compatible",
    defaultModel: "", apiKeyPlaceholder: "sk-...",
    supports: ["streaming"]
  }
];
