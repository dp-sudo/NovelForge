# 文火 NovelForge：LLM Provider 接入设计文档 v1.0

> 文档定位：用于指导文火 NovelForge Windows 桌面端接入多个大模型供应商。  
> 适用模块：模型设置页、Provider Adapter、AI 调用服务、上下文调度、模型能力检测、Token 预算管理、API Key 安全存储。  
> 更新时间：2026-04-26  
> 注意：模型、价格、上下文、限流和接口参数会持续变化，正式产品必须支持 **“手动刷新模型列表”**、**“内置模型注册表热更新”**、**“自定义 OpenAI-compatible Provider”** 与 **“自定义 Anthropic-compatible Provider”**。

---

# 1. 接入目标

文火 NovelForge 需要支持以下 LLM 供应商：

## 1.1 中国 LLM 供应商

1. DeepSeek
2. Kimi / Moonshot AI
3. 智谱 GLM / BigModel / Z.ai
4. MiniMax

## 1.2 全球 LLM 供应商

1. OpenAI
2. Anthropic Claude
3. Google Gemini

## 1.3 自定义提供商

1. OpenAI-compatible Provider
2. 可选扩展：Anthropic-compatible Provider
3. 可选扩展：Google Gemini-compatible Provider

---

# 2. 总体接入原则

## 2.1 不把所有供应商强行当成 OpenAI

虽然很多供应商提供 OpenAI-compatible 接口，但仍然存在以下差异：

- 模型名不同。
- 最大上下文不同。
- 最大输出不同。
- thinking / reasoning 参数不同。
- JSON 结构化输出能力不同。
- Tool Calling 兼容程度不同。
- 流式输出字段不同。
- 多模态输入格式不同。
- 错误码结构不同。
- 内容安全返回字段不同。

因此系统内部必须设计统一抽象层，而不是在业务代码里直接调用某一家 SDK。

## 2.2 统一抽象，分供应商适配

业务层只调用统一接口：

```ts
LlmService.generateText(request)
LlmService.streamText(request)
LlmService.generateStructured(request)
LlmService.callWithTools(request)
LlmService.testConnection(providerId)
```

底层根据 Provider 类型选择 Adapter：

```text
NovelForge AI Service
  ├── OpenAIAdapter
  ├── AnthropicAdapter
  ├── GeminiAdapter
  ├── DeepSeekAdapter
  ├── KimiAdapter
  ├── ZhipuAdapter
  ├── MiniMaxAdapter
  └── CustomOpenAICompatibleAdapter
```

## 2.3 API Key 安全原则

1. API Key 只允许保存在 Windows Credential Manager 或本地加密密钥库中。
2. API Key 不得写入项目目录。
3. API Key 不得进入日志。
4. API Key 不得进入导出文件。
5. UI 中只显示掩码，例如 `sk-d98••••••••••••q2a`。
6. 测试连接时不打印完整请求头。
7. 错误上报不得携带 Authorization Header。

---

# 3. 统一 Provider 配置模型

## 3.1 ProviderConfig

```ts
export type ProviderProtocol =
  | 'openai_responses'
  | 'openai_chat_completions'
  | 'anthropic_messages'
  | 'gemini_generate_content'
  | 'custom_openai_compatible'
  | 'custom_anthropic_compatible';

export interface LlmProviderConfig {
  id: string;
  displayName: string;
  vendor: 'deepseek' | 'kimi' | 'zhipu' | 'minimax' | 'openai' | 'anthropic' | 'gemini' | 'custom';
  protocol: ProviderProtocol;
  baseUrl: string;
  endpointPath?: string;
  apiKeySecretRef: string;
  apiKeyDisplayMask?: string;
  defaultModel: string;
  enabled: boolean;
  timeoutMs: number;
  connectTimeoutMs: number;
  maxRetries: number;
  retryBackoffMs: number;
  createdAt: string;
  updatedAt: string;
}
```

## 3.2 ModelConfig

```ts
export interface LlmModelConfig {
  id: string;
  providerId: string;
  modelName: string;
  displayName: string;
  contextWindowTokens: number;
  maxOutputTokens: number | null;
  inputModalities: Array<'text' | 'image' | 'audio' | 'video' | 'pdf'>;
  outputModalities: Array<'text' | 'json' | 'image' | 'audio' | 'video'>;
  supportsStreaming: boolean;
  supportsTools: boolean;
  supportsJsonObject: boolean;
  supportsJsonSchema: boolean;
  supportsThinking: boolean;
  supportsReasoningEffort: boolean;
  supportsPromptCache: boolean;
  supportsBatch: boolean;
  defaultTemperature?: number;
  temperatureRange?: [number, number];
  defaultTopP?: number;
  topPRange?: [number, number];
  defaultMaxOutputTokens?: number;
  recommendedFor: Array<'chapter_draft' | 'rewrite' | 'planning' | 'consistency_scan' | 'agent' | 'coding' | 'cheap_fast' | 'long_context'>;
  status: 'recommended' | 'available' | 'legacy' | 'deprecated' | 'unknown';
  notes?: string;
}
```

## 3.3 UnifiedGenerateRequest

```ts
export interface UnifiedGenerateRequest {
  providerId: string;
  model: string;
  taskType:
    | 'blueprint_generate'
    | 'character_generate'
    | 'world_generate'
    | 'plot_generate'
    | 'chapter_plan'
    | 'chapter_draft'
    | 'chapter_continue'
    | 'chapter_rewrite'
    | 'prose_naturalize'
    | 'consistency_scan'
    | 'custom';
  systemPrompt?: string;
  messages: UnifiedMessage[];
  responseFormat?: 'text' | 'json_object' | 'json_schema';
  jsonSchema?: Record<string, unknown>;
  tools?: UnifiedTool[];
  toolChoice?: 'auto' | 'none' | { name: string };
  stream?: boolean;
  temperature?: number;
  topP?: number;
  maxOutputTokens?: number;
  reasoning?: {
    enabled?: boolean;
    effort?: 'none' | 'minimal' | 'low' | 'medium' | 'high' | 'xhigh' | 'max';
    budgetTokens?: number;
    summary?: 'auto' | 'concise' | 'detailed' | 'none';
  };
  metadata?: Record<string, string>;
}
```

## 3.4 UnifiedGenerateResponse

```ts
export interface UnifiedGenerateResponse {
  requestId: string;
  providerId: string;
  model: string;
  text?: string;
  json?: unknown;
  toolCalls?: UnifiedToolCall[];
  reasoningText?: string;
  finishReason?: string;
  usage?: {
    inputTokens?: number;
    outputTokens?: number;
    reasoningTokens?: number;
    totalTokens?: number;
    cachedInputTokens?: number;
  };
  raw?: unknown;
}
```

---

# 4. API Key 输入与展示规范

## 4.1 UI 输入字段

模型设置页每个供应商应显示：

| 字段 | 是否必填 | 说明 |
|---|---|---|
| Provider 名称 | 是 | 如 DeepSeek、OpenAI、自定义 OpenAI Compatible |
| API Key | 是 | 密钥输入框，默认隐藏 |
| Base URL | 是 | 供应商默认值可预填 |
| Model | 是 | 可手动输入，也可从模型列表选择 |
| API 协议 | 是 | OpenAI Chat、OpenAI Responses、Anthropic Messages、Gemini GenerateContent |
| 上下文窗口 | 建议填 | 可从内置注册表自动带出 |
| 最大输出 Token | 建议填 | 用于 Token 预算和 UI 限制 |
| 流式输出 | 可选 | 默认开启 |
| Thinking / Reasoning | 可选 | 供应商支持时显示 |
| JSON 输出 | 可选 | 供应商支持时显示 |
| Tool Calling | 可选 | 供应商支持时显示 |

## 4.2 API Key 示例格式

以下只作为 UI 占位符，不是真实密钥：

| 供应商 | 占位符示例 |
|---|---|
| DeepSeek | `sk-d98••••••••••••••••••••x7a` |
| Kimi / Moonshot | `sk-moonshot-••••••••••••••••` |
| 智谱 GLM | `zhipu-••••••••••••••••` 或 `YOUR_API_KEY` |
| MiniMax | `minimax-••••••••••••••••` |
| OpenAI | `sk-proj-••••••••••••••••` |
| Anthropic | `sk-ant-api03-••••••••••••••••` |
| Google Gemini | `AIza••••••••••••••••` |
| 自定义 OpenAI-compatible | `sk-••••••••••••••••` |

## 4.3 掩码策略

输入后保存：

```text
原始：sk-proj-1234567890abcdefghijklmn
显示：sk-proj-1234••••••••••••lmn
```

如果长度不足 12 位：

```text
显示：••••••••
```

---

# 5. 供应商总览表

| 供应商 | 推荐协议 | 默认 Base URL | 主要端点 | 推荐默认模型 |
|---|---|---|---|---|
| DeepSeek | OpenAI Chat Completions | `https://api.deepseek.com` | `/chat/completions` | `deepseek-v4-flash` |
| Kimi / Moonshot | OpenAI Chat Completions | `https://api.moonshot.ai/v1` | `/chat/completions` | `kimi-k2.6` |
| 智谱 GLM | OpenAI-style Chat Completions | `https://open.bigmodel.cn/api/paas/v4` | `/chat/completions` | `glm-5.1` |
| MiniMax | Anthropic-compatible 优先；OpenAI-compatible 可选 | `https://api.minimax.io/anthropic` 或 `https://api.minimax.io` | `/anthropic/v1/messages` 或 `/v1/chat/completions` | `MiniMax-M2.7` |
| OpenAI | Responses API 优先 | `https://api.openai.com/v1` | `/responses` | `gpt-5.5` |
| Anthropic | Messages API | `https://api.anthropic.com/v1` | `/messages` | `claude-sonnet-4-6` 或 `claude-opus-4-7` |
| Google Gemini | Gemini GenerateContent | `https://generativelanguage.googleapis.com/v1beta` | `/models/{model}:generateContent` | `gemini-3.1-pro-preview` |
| 自定义 | OpenAI-compatible | 用户填写 | 通常 `/chat/completions` | 用户填写 |

---

# 6. DeepSeek 接入设计

## 6.1 基本配置

| 项目 | 值 |
|---|---|
| 供应商名称 | DeepSeek |
| Vendor ID | `deepseek` |
| 推荐协议 | OpenAI Chat Completions |
| OpenAI 格式 Base URL | `https://api.deepseek.com` |
| Anthropic 格式 Base URL | `https://api.deepseek.com/anthropic` |
| Chat Endpoint | `/chat/completions` |
| 认证方式 | `Authorization: Bearer ${DEEPSEEK_API_KEY}` |
| API Key 环境变量建议 | `DEEPSEEK_API_KEY` |

## 6.2 官方模型

| 模型 ID | 定位 | 上下文窗口 | 最大输出 | 状态 | 建议用途 |
|---|---|---:|---:|---|---|
| `deepseek-v4-flash` | 快速、低成本、通用 Agent | 1,000,000 | 384,000 | 推荐默认 | 章节草稿、续写、角色生成、普通审稿 |
| `deepseek-v4-pro` | 更强推理与复杂任务 | 1,000,000 | 384,000 | 推荐高级 | 全书规划、复杂一致性审查、长上下文分析 |
| `deepseek-chat` | 兼容旧模型名 | 1,000,000 | 384,000 | 即将弃用 | 仅兼容旧配置 |
| `deepseek-reasoner` | 兼容旧模型名 | 1,000,000 | 384,000 | 即将弃用 | 仅兼容旧配置 |

## 6.3 重要说明

1. `deepseek-chat` 与 `deepseek-reasoner` 已进入弃用路径，应提示用户迁移到 `deepseek-v4-flash` 或 `deepseek-v4-pro`。
2. DeepSeek V4 支持 Thinking / Non-Thinking 双模式。
3. 推荐在文火中将 `deepseek-v4-flash` 作为默认模型，将 `deepseek-v4-pro` 作为高级审稿与复杂规划模型。

## 6.4 可控参数

| 参数 | 类型 | 说明 | NovelForge 建议 |
|---|---|---|---|
| `model` | string | 模型 ID | 必填 |
| `messages` | array | OpenAI 格式消息 | 必填 |
| `stream` | boolean | 是否流式输出 | 默认 true |
| `thinking` | object | 是否启用思考 | 复杂任务启用，普通续写可关闭 |
| `reasoning_effort` | string | 推理强度 | `low` / `medium` / `high`，复杂检查用 high |
| `response_format` | object | JSON 输出 | 一致性检查使用 |
| `tools` | array | 工具调用 | Agent 工作流使用 |

## 6.5 NovelForge 推荐预设

```json
{
  "provider": "deepseek",
  "model": "deepseek-v4-flash",
  "baseUrl": "https://api.deepseek.com",
  "protocol": "openai_chat_completions",
  "contextWindowTokens": 1000000,
  "maxOutputTokens": 384000,
  "supportsStreaming": true,
  "supportsThinking": true,
  "supportsTools": true,
  "supportsJsonObject": true,
  "recommendedFor": ["chapter_draft", "chapter_continue", "consistency_scan", "long_context"]
}
```

---

# 7. Kimi / Moonshot AI 接入设计

## 7.1 基本配置

| 项目 | 值 |
|---|---|
| 供应商名称 | Kimi / Moonshot AI |
| Vendor ID | `kimi` |
| 推荐协议 | OpenAI Chat Completions |
| Base URL | `https://api.moonshot.ai/v1` |
| Chat Endpoint | `/chat/completions` |
| 认证方式 | `Authorization: Bearer ${MOONSHOT_API_KEY}` |
| API Key 环境变量建议 | `MOONSHOT_API_KEY` |

## 7.2 官方模型

| 模型 ID | 定位 | 上下文窗口 | 最大输出 | 状态 | 建议用途 |
|---|---|---:|---:|---|---|
| `kimi-k2.6` | 最新、最强、多模态、Agent / Coding / Thinking | 256,000 | 未统一公开；示例常用 32K | 推荐默认 | 长篇创作、Agent、代码、图文理解 |
| `kimi-k2.5` | 上一代强模型 | 256,000 | 未统一公开 | 可用 | 备用模型 |
| `kimi-k2-0905-preview` | K2 旧预览 | 256,000 | 未统一公开 | 将停用 | 不建议新接入 |
| `kimi-k2-turbo-preview` | K2 高速旧预览 | 256,000 | 未统一公开 | 将停用 | 不建议新接入 |
| `kimi-k2-thinking` | K2 长思考模型 | 256,000 | 未统一公开 | 将停用 | 仅兼容旧配置 |
| `kimi-k2-thinking-turbo` | K2 长思考高速模型 | 256,000 | 未统一公开 | 将停用 | 仅兼容旧配置 |
| `moonshot-v1-8k` | 旧文本生成 | 8,000 | 未统一公开 | Legacy | 短文本 |
| `moonshot-v1-32k` | 旧文本生成 | 32,000 | 未统一公开 | Legacy | 中长文本 |
| `moonshot-v1-128k` | 旧文本生成 | 128,000 | 未统一公开 | Legacy | 长文本 |

## 7.3 重要说明

1. `kimi-k2.6` 是首选模型。
2. `kimi-k2.6` 默认启用 Thinking，可通过 `thinking: {"type": "disabled"}` 关闭。
3. Kimi K2.6 / K2.5 对采样参数限制较严格：`temperature`、`top_p`、`n`、`presence_penalty`、`frequency_penalty` 通常应使用官方固定值或不要显式传入。
4. 使用 Thinking + Tool Calling 时，需要保留并回传 `reasoning_content`，否则多步工具调用可能失败或效果下降。

## 7.4 可控参数

| 参数 | 类型 | 说明 | NovelForge 建议 |
|---|---|---|---|
| `model` | string | 模型 ID | `kimi-k2.6` |
| `messages` | array | OpenAI 格式消息 | 必填 |
| `stream` | boolean | 流式输出 | Thinking 任务建议 true |
| `max_tokens` | integer | 输出上限，包含 reasoning_content 与最终文本 | 长任务建议 ≥ 16000 |
| `thinking` | object | `enabled` / `disabled` | 章节草稿可开启；快速改写可关闭 |
| `tools` | array | 工具调用 | Agent 任务使用 |
| `tool_choice` | string | Thinking 模式下通常只能 `auto` 或 `none` | 默认 auto |

## 7.5 NovelForge 推荐预设

```json
{
  "provider": "kimi",
  "model": "kimi-k2.6",
  "baseUrl": "https://api.moonshot.ai/v1",
  "protocol": "openai_chat_completions",
  "contextWindowTokens": 256000,
  "maxOutputTokens": 32768,
  "supportsStreaming": true,
  "supportsThinking": true,
  "supportsTools": true,
  "supportsJsonObject": true,
  "recommendedFor": ["chapter_draft", "agent", "coding", "multimodal"]
}
```

> 注：`maxOutputTokens: 32768` 是产品侧安全默认值，不等同于官方统一最大输出声明。正式实现应允许用户覆盖。

---

# 8. 智谱 GLM / BigModel / Z.ai 接入设计

## 8.1 基本配置

| 项目 | 值 |
|---|---|
| 供应商名称 | 智谱 GLM / BigModel / Z.ai |
| Vendor ID | `zhipu` |
| 推荐协议 | OpenAI-style Chat Completions |
| 通用 Base URL | `https://open.bigmodel.cn/api/paas/v4` |
| 通用 Chat Endpoint | `/chat/completions` |
| Coding 专用 Base URL | `https://open.bigmodel.cn/api/coding/paas/v4` |
| 认证方式 | `Authorization: Bearer ${ZHIPU_API_KEY}` |
| API Key 环境变量建议 | `ZHIPU_API_KEY` |

## 8.2 官方文本模型

| 模型 ID | 定位 | 上下文窗口 | 最大输出 | 状态 | 建议用途 |
|---|---|---:|---:|---|---|
| `glm-5.1` | 最新旗舰 | 200,000 | 128,000 | 推荐默认 | 高质量章节、复杂规划、Agent |
| `glm-5` | 高智能基座 | 200,000 | 128,000 | 推荐 | 规划、审稿、Agent |
| `glm-5-turbo` | 任务增强基座 | 200,000 | 128,000 | 可用 | 长任务执行 |
| `glm-4.7` | 高智能模型 | 200,000 | 128,000 | 可用 | 通用写作、推理、工具调用 |
| `glm-4.7-flashx` | 轻量高速 | 200,000 | 128,000 | 可用 | 快速改写、续写 |
| `glm-4.6` | 强性能 | 200,000 | 128,000 | 可用 | 通用任务 |
| `glm-4.5-air` | 高性价比 | 128,000 | 96,000 | 可用 | 成本敏感写作 |
| `glm-4.5-airx` | 高性价比极速版 | 128,000 | 96,000 | 可用 | 低延迟任务 |
| `glm-4-long` | 超长输入 | 1,000,000 | 4,000 | 可用 | 全书扫描、长文摘要，不适合长输出 |
| `glm-4-flashx-250414` | 高速低价 | 128,000 | 16,000 | 可用 | 简短任务 |
| `glm-4.7-flash` | 免费模型 | 200,000 | 128,000 | 可用 | 体验与低成本 |

## 8.3 视觉模型

| 模型 ID | 定位 | 上下文窗口 | 最大输出 | 建议用途 |
|---|---|---:|---:|---|
| `glm-5v-turbo` | 多模态 Coding 基座 | 200,000 | 128,000 | 图片 / UI / 多模态辅助 |
| `glm-4.6v` | 视觉推理 | 128,000 | 32,000 | 图片理解、前端复刻辅助 |
| `glm-4.6v-flash` | 免费视觉推理 | 128,000 | 32,000 | 低成本视觉任务 |

## 8.4 可控参数

| 参数 | 类型 | 说明 | NovelForge 建议 |
|---|---|---|---|
| `model` | string | 模型 ID | 默认 `glm-5.1` |
| `messages` | array | 对话消息 | 必填 |
| `do_sample` | boolean | 是否采样 | 创作 true，严谨检查 false |
| `temperature` | number | 随机性 | 创作 0.7-1.0，检查 0.2-0.5 |
| `top_p` | number | 核采样 | 与 temperature 二选一 |
| `max_tokens` | integer | 最大输出 | 章节草稿按目标字数估算 |
| `stream` | boolean | 流式输出 | 默认 true |
| `thinking` | object | 是否思考 | 复杂任务启用，轻量任务关闭 |
| `tools` | array | 工具调用 | Agent 任务使用 |
| `tool_choice` | string | 工具选择，目前多为 auto | 默认 auto |
| `response_format` | object | JSON 模式 | 一致性检查、资产抽取使用 |

## 8.5 Thinking 策略

| 任务 | 建议 |
|---|---|
| 章节草稿 | `thinking.enabled`，temperature 0.8-1.0 |
| 续写 | 可关闭 thinking，提高速度 |
| 世界观规划 | 开启 thinking |
| 一致性检查 | 开启 thinking，低 temperature |
| 去 AI 味 | 关闭 thinking 或低 thinking |
| Agent 工具调用 | 开启 thinking，并保留 reasoning content |

## 8.6 NovelForge 推荐预设

```json
{
  "provider": "zhipu",
  "model": "glm-5.1",
  "baseUrl": "https://open.bigmodel.cn/api/paas/v4",
  "endpointPath": "/chat/completions",
  "protocol": "openai_chat_completions",
  "contextWindowTokens": 200000,
  "maxOutputTokens": 128000,
  "supportsStreaming": true,
  "supportsThinking": true,
  "supportsTools": true,
  "supportsJsonObject": true,
  "recommendedFor": ["planning", "chapter_draft", "consistency_scan", "agent"]
}
```

---

# 9. MiniMax 接入设计

## 9.1 基本配置

| 项目 | 值 |
|---|---|
| 供应商名称 | MiniMax |
| Vendor ID | `minimax` |
| 推荐协议 | Anthropic-compatible，OpenAI-compatible 可选 |
| Anthropic-compatible Base URL | `https://api.minimax.io/anthropic` |
| OpenAI-compatible Base URL | `https://api.minimax.io` |
| OpenAI Chat Endpoint | `/v1/chat/completions` |
| 认证方式 | `Authorization: Bearer ${MINIMAX_API_KEY}` |
| API Key 环境变量建议 | `MINIMAX_API_KEY` |

## 9.2 官方模型

| 模型 ID | 定位 | 上下文窗口 | 最大输出 | 状态 | 建议用途 |
|---|---|---:|---:|---|---|
| `MiniMax-M2.7` | 最新主力，复杂工程、办公交付、角色交互 | 204,800 | OpenAI-compatible 文档参数示例上限 2,048；Anthropic-compatible 未统一明示 | 推荐默认 | Agent、复杂写作、角色互动 |
| `MiniMax-M2.7-highspeed` | M2.7 高速版 | 204,800 | 同上 | 推荐高速 | 低延迟续写、批量任务 |
| `MiniMax-M2.5` | 高性能旧主力 | 204,800 | 未统一明示 | 可用 | 备用 |
| `MiniMax-M2.5-highspeed` | M2.5 高速版 | 204,800 | 未统一明示 | 可用 | 低延迟 |
| `MiniMax-M2.1` | 代码与推理 | 204,800 | 未统一明示 | 可用 | 代码 / 工程任务 |
| `MiniMax-M2.1-highspeed` | M2.1 高速版 | 204,800 | 未统一明示 | 可用 | 低延迟 |
| `MiniMax-M2` | Agentic / 推理旧模型 | 204,800 | 128,000，包括 CoT | Legacy | 不建议新默认 |
| `M2-her` | 对话与角色扮演模型 | 未统一明示 | 未统一明示 | 可选 | 角色扮演、沉浸式对话 |

## 9.3 重要说明

1. MiniMax 同时提供 OpenAI-compatible 和 Anthropic-compatible 接口。
2. 对于文火 NovelForge，建议优先实现 Anthropic-compatible，因为其文档明确支持 thinking、tools、tool_choice、max_tokens、stream、system 等参数。
3. OpenAI-compatible 端点的 `max_completion_tokens` 文档显示上限为 2048，可能不适合长章节生成；若用于长篇写作，应在设置页提示用户。
4. MiniMax 的响应可能包含 `<think>...</think>` 或 reasoning content，产品应支持隐藏 / 折叠推理内容。

## 9.4 可控参数

| 参数 | 协议 | 说明 | NovelForge 建议 |
|---|---|---|---|
| `model` | OpenAI / Anthropic | 模型 ID | `MiniMax-M2.7` |
| `messages` | OpenAI / Anthropic | 消息列表 | 必填 |
| `system` | Anthropic | 系统提示词 | 推荐使用 |
| `stream` | OpenAI / Anthropic | 流式输出 | 默认 true |
| `max_tokens` | Anthropic | 最大输出 | 按任务设置 |
| `max_completion_tokens` | OpenAI | 输出上限 | 注意官方文档约束 |
| `temperature` | OpenAI / Anthropic | 随机性，范围通常 (0, 1] | 创作 1，审稿 0.3-0.6 |
| `top_p` | OpenAI / Anthropic | 核采样 | 默认 0.95 |
| `tools` | Anthropic | 工具定义 | Agent 使用 |
| `tool_choice` | Anthropic | 工具选择 | auto |
| `thinking` | Anthropic | 推理内容 | Agent / 复杂规划启用 |

## 9.5 NovelForge 推荐预设

```json
{
  "provider": "minimax",
  "model": "MiniMax-M2.7",
  "baseUrl": "https://api.minimax.io/anthropic",
  "protocol": "anthropic_messages",
  "contextWindowTokens": 204800,
  "maxOutputTokens": 32768,
  "supportsStreaming": true,
  "supportsThinking": true,
  "supportsTools": true,
  "supportsJsonObject": false,
  "recommendedFor": ["agent", "chapter_draft", "character_dialogue", "coding"]
}
```

> 注：`maxOutputTokens: 32768` 是产品侧默认值，不是官方统一最大输出声明。

---

# 10. OpenAI 接入设计

## 10.1 基本配置

| 项目 | 值 |
|---|---|
| 供应商名称 | OpenAI |
| Vendor ID | `openai` |
| 推荐协议 | Responses API |
| Base URL | `https://api.openai.com/v1` |
| Responses Endpoint | `/responses` |
| Chat Completions Endpoint | `/chat/completions` |
| 认证方式 | `Authorization: Bearer ${OPENAI_API_KEY}` |
| API Key 环境变量建议 | `OPENAI_API_KEY` |

## 10.2 官方模型

| 模型 ID | 定位 | 上下文窗口 | 最大输出 | 状态 | 建议用途 |
|---|---|---:|---:|---|---|
| `gpt-5.5` | 最新旗舰，复杂专业工作、代码、Agent | 1,050,000 | 128,000 | 推荐默认高级 | 全书级分析、复杂章节、Agent |
| `gpt-5.5-pro` | 更高智能，更慢、更贵 | 1,050,000 | 128,000 | 高级 | 极复杂审稿、架构规划、长任务 |
| `gpt-5.4` | 更低成本的强模型 | 1,050,000 | 128,000 | 可用 | 成本敏感高级任务 |
| `gpt-5.4-pro` | GPT-5.4 高级版 | 1,050,000 | 128,000 | 可用 | 高质量规划 |
| `gpt-5.4-mini` | 小型高性价比 | 视官方模型页 | 视官方模型页 | 可用 | 批量改写、摘要、检查 |
| `gpt-5.4-nano` | 低成本高吞吐 | 视官方模型页 | 视官方模型页 | 可用 | 标签、分类、短摘要 |

## 10.3 推荐协议

OpenAI 新模型优先使用 Responses API，而不是 Chat Completions：

```text
POST https://api.openai.com/v1/responses
```

原因：

1. 更适合 reasoning models。
2. 支持工具、托管工具、文件搜索、Web 搜索。
3. 支持 reasoning effort。
4. 支持长任务与 background mode。
5. 支持更完整的多步 Agent 状态管理。

## 10.4 可控参数

| 参数 | 说明 | NovelForge 建议 |
|---|---|---|
| `model` | 模型 ID | `gpt-5.5` |
| `input` | Responses API 输入 | 必填 |
| `instructions` | 顶层系统指令 | 可用于文火固定系统提示 |
| `reasoning.effort` | 推理强度 | 默认 medium，低延迟用 low，复杂任务 high/xhigh |
| `reasoning.summary` | 推理摘要 | 调试模式可 auto，普通用户隐藏 |
| `max_output_tokens` | 最大输出，包含推理与最终输出预算 | 必填或按任务设置 |
| `tools` | 工具 | Agent 使用 |
| `tool_choice` | 工具选择 | auto |
| `text.format` / `response_format` | 结构化输出 | 一致性检查、资产抽取使用 |
| `stream` | 流式输出 | 默认 true |
| `previous_response_id` | 续接上次响应 | 多轮 Agent 使用 |
| `background` | 后台长任务 | GPT-5.5 Pro 复杂任务建议 |
| `include` | 包含额外内容 | 可用于 encrypted reasoning items |

## 10.5 NovelForge 推荐预设

```json
{
  "provider": "openai",
  "model": "gpt-5.5",
  "baseUrl": "https://api.openai.com/v1",
  "protocol": "openai_responses",
  "contextWindowTokens": 1050000,
  "maxOutputTokens": 128000,
  "supportsStreaming": true,
  "supportsThinking": true,
  "supportsReasoningEffort": true,
  "supportsTools": true,
  "supportsJsonObject": true,
  "supportsJsonSchema": true,
  "supportsPromptCache": true,
  "recommendedFor": ["long_context", "agent", "planning", "consistency_scan", "chapter_draft"]
}
```

---

# 11. Anthropic Claude 接入设计

## 11.1 基本配置

| 项目 | 值 |
|---|---|
| 供应商名称 | Anthropic Claude |
| Vendor ID | `anthropic` |
| 推荐协议 | Anthropic Messages API |
| Base URL | `https://api.anthropic.com/v1` |
| Messages Endpoint | `/messages` |
| 认证方式 | `x-api-key: ${ANTHROPIC_API_KEY}` |
| API Version Header | `anthropic-version: 2023-06-01` |
| API Key 环境变量建议 | `ANTHROPIC_API_KEY` |

## 11.2 官方模型

| 模型 ID | 定位 | 上下文窗口 | 最大输出 | 状态 | 建议用途 |
|---|---|---:|---:|---|---|
| `claude-opus-4-7` | 最新最强通用模型，复杂推理与 Agent coding | 1,000,000 | 128,000 | 推荐高级 | 全书级审稿、复杂 Agent、架构规划 |
| `claude-sonnet-4-6` | 速度与智能平衡 | 1,000,000 | 64,000 | 推荐默认 | 章节写作、规划、审稿、Agent |
| `claude-haiku-4-5-20251001` | 快速、低成本、近前沿 | 200,000 | 64,000 | 推荐低成本 | 摘要、改写、分类、轻量检查 |
| `claude-opus-4-6` | 上代 Opus，仍强 | 1,000,000 | 128,000 | 可用 | 高质量复杂任务 |

## 11.3 重要说明

1. 同步 Messages API 的最大输出通常按模型区分。
2. Message Batches API 对部分模型可通过 beta header 支持更高输出。
3. Claude Sonnet 4.6 与 Opus 4.7 / 4.6 支持长上下文与更复杂 Agent 工作流。
4. Claude 的工具调用与 thinking 内容需要按 Anthropic Messages API 原样维护上下文。

## 11.4 可控参数

| 参数 | 类型 | 说明 | NovelForge 建议 |
|---|---|---|---|
| `model` | string | 模型 ID | 默认 `claude-sonnet-4-6` |
| `system` | string / array | 系统提示 | 文火固定系统提示放这里 |
| `messages` | array | 用户 / 助手消息 | 必填 |
| `max_tokens` | integer | 最大输出 | 必填 |
| `stream` | boolean | 流式输出 | 默认 true |
| `temperature` | number | 随机性 | 创作中等，检查较低 |
| `top_p` | number | 核采样 | 一般不与 temperature 同时调 |
| `tools` | array | 工具定义 | Agent 使用 |
| `tool_choice` | object/string | 工具选择 | auto |
| `thinking` | object | 思考模式 | 复杂任务启用 |
| `output_config.effort` | string | 新模型推理强度 | low / medium / high / max 等 |
| `metadata` | object | 元信息 | 可填 requestId，不填正文 |

## 11.5 NovelForge 推荐预设

```json
{
  "provider": "anthropic",
  "model": "claude-sonnet-4-6",
  "baseUrl": "https://api.anthropic.com/v1",
  "endpointPath": "/messages",
  "protocol": "anthropic_messages",
  "contextWindowTokens": 1000000,
  "maxOutputTokens": 64000,
  "supportsStreaming": true,
  "supportsThinking": true,
  "supportsTools": true,
  "supportsJsonObject": false,
  "supportsPromptCache": true,
  "recommendedFor": ["chapter_draft", "planning", "agent", "consistency_scan", "long_context"]
}
```

---

# 12. Google Gemini 接入设计

## 12.1 基本配置

| 项目 | 值 |
|---|---|
| 供应商名称 | Google Gemini |
| Vendor ID | `gemini` |
| 推荐协议 | Gemini GenerateContent API |
| Base URL | `https://generativelanguage.googleapis.com/v1beta` |
| GenerateContent Endpoint | `/models/{model}:generateContent` |
| Stream Endpoint | `/models/{model}:streamGenerateContent` |
| 认证方式 | `x-goog-api-key: ${GEMINI_API_KEY}` |
| API Key 环境变量建议 | `GEMINI_API_KEY` |

## 12.2 官方模型

| 模型 ID | 定位 | 输入上下文 | 最大输出 | 状态 | 建议用途 |
|---|---|---:|---:|---|---|
| `gemini-3.1-pro-preview` | 最新 Pro，复杂推理、Agent、Coding、多模态 | 1,000,000 | 64,000 | 推荐高级 | 全书规划、复杂审稿、多模态 |
| `gemini-3-flash-preview` | 3 系列高性能 Flash | 1,000,000 | 64,000 | 推荐默认 | 章节草稿、低延迟 Agent |
| `gemini-3.1-flash-lite-preview` | 成本效率模型 | 1,000,000 | 64,000 | 推荐低成本 | 批量摘要、检查、分类 |
| `gemini-2.5-pro` | GA 高能力模型 | 1,000,000 | 视模型页 | 稳定可用 | 生产环境备选 |
| `gemini-2.5-flash` | GA 性价比模型 | 视模型页 | 视模型页 | 稳定可用 | 生产环境低延迟 |

## 12.3 Gemini 3 关键控制参数

| 参数 | 说明 | NovelForge 建议 |
|---|---|---|
| `thinkingConfig.thinkingLevel` | 控制思考深度 | 章节规划 high，快速改写 low |
| `media_resolution` | 控制图像 / PDF / 视频处理分辨率与 token 消耗 | 普通图片 high，PDF medium，长视频 low |
| `temperature` | Gemini 3 官方建议保持默认 1.0 | 不建议随意降低 |
| `tools` | Google Search、File Search、Code Execution、URL Context、函数调用等 | Agent 使用 |
| `responseMimeType` | 可指定 JSON | 结构化输出使用 |
| `responseSchema` | JSON Schema | 一致性检查、资产抽取使用 |
| `cachedContent` | 上下文缓存 | 大型项目上下文复用 |

## 12.4 Thinking Level 映射

| NovelForge 统一值 | Gemini 3 映射 |
|---|---|
| `none` | 不建议；Gemini 3 使用 `minimal` 近似 |
| `minimal` | `minimal` |
| `low` | `low` |
| `medium` | `medium` |
| `high` | `high` |
| `xhigh` | `high`，并提高输出预算 |

## 12.5 NovelForge 推荐预设

```json
{
  "provider": "gemini",
  "model": "gemini-3.1-pro-preview",
  "baseUrl": "https://generativelanguage.googleapis.com/v1beta",
  "protocol": "gemini_generate_content",
  "contextWindowTokens": 1000000,
  "maxOutputTokens": 64000,
  "supportsStreaming": true,
  "supportsThinking": true,
  "supportsReasoningEffort": true,
  "supportsTools": true,
  "supportsJsonObject": true,
  "supportsJsonSchema": true,
  "supportsPromptCache": true,
  "recommendedFor": ["long_context", "planning", "consistency_scan", "multimodal", "agent"]
}
```

---

# 13. 自定义 Provider 设计

文火 NovelForge 必须支持两类自定义提供商：

1. **自定义 OpenAI-compatible Provider**
2. **自定义 Anthropic-compatible Provider**

这样可以兼容自建网关、企业代理、LiteLLM、OneAPI / NewAPI、OpenRouter、私有部署模型、Ollama、LM Studio、vLLM，以及未来提供 Anthropic Messages API 兼容接口的平台。

---

## 13.1 自定义 OpenAI-compatible Provider

### 13.1.1 目标

允许用户接入：

1. 自建 vLLM。
2. Ollama OpenAI-compatible server。
3. LM Studio。
4. OneAPI / NewAPI / LiteLLM。
5. OpenRouter。
6. SiliconFlow。
7. 火山方舟、阿里云百炼、腾讯混元等提供 OpenAI-compatible 的平台。
8. 公司内部 OpenAI-compatible 代理网关。

### 13.1.2 UI 配置字段

| 字段 | 示例 | 说明 |
|---|---|---|
| Provider 名称 | `My Local vLLM` | 用户自定义 |
| Base URL | `http://localhost:8000/v1` | 不包含 `/chat/completions` 时由系统追加 |
| API Key | `sk-local-xxx` | 可为空，取决于服务 |
| Endpoint 类型 | Chat Completions / Responses | 默认 Chat Completions |
| Model | `qwen3-235b-a22b` | 用户填写或从 `/models` 刷新 |
| Context Window | `262144` | 用户填写或从注册表匹配 |
| Max Output Tokens | `32768` | 用户填写 |
| Streaming | true | 是否支持流式 |
| Tool Calling | true / false | 用户声明或测试检测 |
| JSON Mode | true / false | 用户声明或测试检测 |
| JSON Schema | true / false | 用户声明或测试检测 |
| Vision | true / false | 用户声明 |
| Thinking 参数模式 | none / openai / deepseek / kimi / zhipu | 用于参数映射 |
| 模型列表刷新方式 | `/models` / 手动输入 / 远程注册表 | 自定义 Provider 可选 |

### 13.1.3 自定义 OpenAI-compatible Provider 数据结构

```json
{
  "id": "custom-openai-local-vllm",
  "displayName": "Local vLLM",
  "vendor": "custom",
  "protocol": "custom_openai_compatible",
  "baseUrl": "http://localhost:8000/v1",
  "endpointPath": "/chat/completions",
  "apiKeySecretRef": "secret://custom-openai-local-vllm",
  "defaultModel": "qwen3-235b-a22b",
  "enabled": true,
  "timeoutMs": 120000,
  "maxRetries": 1,
  "modelRefresh": {
    "mode": "openai_models_endpoint",
    "modelsPath": "/models",
    "lastRefreshedAt": null
  }
}
```

### 13.1.4 OpenAI-compatible 自动能力检测

点击「测试并检测能力」时，系统执行：

1. `GET /models`，如果可用，读取模型列表。
2. 用短 prompt 测试普通文本输出。
3. 用 `stream: true` 测试流式输出。
4. 用 `response_format: {"type":"json_object"}` 测试 JSON。
5. 用 JSON Schema 测试结构化输出。
6. 用简单 `tools` 测试 Tool Calling。
7. 如果用户启用 Vision，测试图片输入。
8. 生成能力报告并写入 ModelConfig。

---

## 13.2 自定义 Anthropic-compatible Provider

### 13.2.1 目标

允许用户接入任何兼容 Anthropic Messages API 的服务，例如：

1. MiniMax Anthropic-compatible 接口。
2. LiteLLM Anthropic-compatible 路由。
3. OpenRouter 的 Anthropic-compatible 网关。
4. 公司内部 Claude / Anthropic 代理网关。
5. 自建将其他模型转换为 Anthropic Messages 格式的 API Gateway。
6. 未来新增的 Anthropic-compatible 第三方平台。

### 13.2.2 推荐协议

```text
POST {baseUrl}/messages
```

或：

```text
POST {baseUrl}/{endpointPath}
```

默认：

```text
baseUrl = https://example.com/v1
endpointPath = /messages
```

### 13.2.3 认证方式

自定义 Anthropic-compatible Provider 必须支持多种认证头，因为不同网关可能不完全一致：

| 认证模式 | Header |
|---|---|
| Anthropic 原生 | `x-api-key: ${API_KEY}` |
| Bearer 兼容 | `Authorization: Bearer ${API_KEY}` |
| 自定义 Header | 用户填写 Header 名称和值 |

默认使用 Anthropic 原生认证：

```text
x-api-key: ${API_KEY}
anthropic-version: 2023-06-01
```

### 13.2.4 UI 配置字段

| 字段 | 示例 | 说明 |
|---|---|---|
| Provider 名称 | `My Anthropic Gateway` | 用户自定义 |
| Base URL | `https://gateway.example.com/v1` | 不包含 `/messages` 时由系统追加 |
| Endpoint Path | `/messages` | 默认 `/messages` |
| API Key | `sk-ant-xxx` | 可为空，取决于服务 |
| Auth 模式 | `x-api-key` / `bearer` / `custom` | 默认 `x-api-key` |
| Anthropic Version | `2023-06-01` | 默认值，可修改 |
| Beta Headers | `context-1m-2025-08-07` | 可选，用于 1M context / extended output 等 beta |
| Model | `claude-sonnet-4-6` | 用户填写或手动刷新 |
| Context Window | `1000000` | 用户填写或注册表匹配 |
| Max Output Tokens | `64000` | 用户填写 |
| Streaming | true | 是否支持 SSE 流式 |
| Thinking | true / false | 是否支持 `thinking` 参数 |
| Tool Calling | true / false | 是否支持 `tools` |
| Prompt Cache | true / false | 是否支持 cache control |
| JSON 策略 | tool_schema / prompt_only | Anthropic 无通用 JSON Mode，推荐 tool schema |

### 13.2.5 自定义 Anthropic-compatible Provider 数据结构

```json
{
  "id": "custom-anthropic-gateway",
  "displayName": "Company Anthropic Gateway",
  "vendor": "custom",
  "protocol": "custom_anthropic_compatible",
  "baseUrl": "https://gateway.example.com/v1",
  "endpointPath": "/messages",
  "apiKeySecretRef": "secret://custom-anthropic-gateway",
  "defaultModel": "claude-sonnet-4-6",
  "enabled": true,
  "timeoutMs": 180000,
  "maxRetries": 2,
  "anthropic": {
    "version": "2023-06-01",
    "authMode": "x_api_key",
    "betaHeaders": ["context-1m-2025-08-07"],
    "customHeaders": []
  },
  "modelRefresh": {
    "mode": "manual_or_registry",
    "modelsPath": null,
    "lastRefreshedAt": null
  }
}
```

### 13.2.6 Anthropic-compatible 请求映射

统一请求：

```ts
UnifiedGenerateRequest
```

转换为 Anthropic Messages：

```json
{
  "model": "claude-sonnet-4-6",
  "system": "系统提示词",
  "messages": [
    {
      "role": "user",
      "content": "用户输入"
    }
  ],
  "max_tokens": 4096,
  "stream": true,
  "temperature": 0.7,
  "tools": [],
  "tool_choice": { "type": "auto" }
}
```

### 13.2.7 Anthropic-compatible 流式事件解析

需要支持以下事件类型：

| Event | 说明 |
|---|---|
| `message_start` | 消息开始 |
| `content_block_start` | 内容块开始 |
| `content_block_delta` | 文本 / 工具 / thinking 增量 |
| `content_block_stop` | 内容块结束 |
| `message_delta` | usage / stop_reason 更新 |
| `message_stop` | 消息结束 |
| `error` | 错误事件 |

统一转换为：

```ts
interface AiStreamEvent {
  requestId: string;
  type: 'start' | 'delta' | 'thinking_delta' | 'tool_call' | 'done' | 'error';
  delta?: string;
  reasoningDelta?: string;
  toolCall?: UnifiedToolCall;
  error?: LlmError;
}
```

### 13.2.8 Anthropic-compatible JSON 输出策略

Anthropic-compatible Provider 不应假设支持 OpenAI 的 `response_format`。

优先级：

1. **工具调用结构化输出**：定义一个 `emit_json` tool，让模型以 tool input 形式返回结构化数据。
2. **Prompt-only JSON**：通过提示词要求只输出 JSON。
3. **本地 JSON 修复**：解析失败后进行本地修复或二次修复请求。

### 13.2.9 Anthropic-compatible 自动能力检测

点击「测试并检测能力」时，系统执行：

1. 发送最小 Messages 请求：`请回答 OK`。
2. 测试 `stream: true`。
3. 测试 `tools` 和 `tool_choice`。
4. 测试 `thinking` 参数，如果失败则自动关闭。
5. 测试 Prompt Cache Headers，如果失败则关闭。
6. 测试 beta headers 是否被接受。
7. 保存能力报告。

---

## 13.3 自定义 Provider 风险提示

UI 应提示：

> 自定义 Provider 的兼容性取决于服务端实现。即使接口路径相同，也可能不支持 JSON Mode、JSON Schema、Tool Calling、Thinking、Vision、Prompt Cache 或标准流式格式。请先点击“测试并检测能力”。

## 13.4 自定义 Provider 最低可用标准

自定义 Provider 至少需要满足：

1. 可保存配置。
2. 可测试连接。
3. 可完成普通文本生成。
4. 可返回标准化错误。
5. 不泄露 API Key。
6. 如果不支持流式输出，应自动回退为非流式。
7. 如果不支持 JSON / Tool Calling，应在能力面板中显示为不支持。

# 14. 统一 Reasoning / Thinking 参数映射

## 14.1 统一 UI 控件

模型设置页不直接显示各家复杂参数，而显示：

| UI 字段 | 选项 |
|---|---|
| 推理模式 | 自动 / 关闭 / 低 / 中 / 高 / 极高 |
| 速度优先 | 开 / 关 |
| 成本优先 | 开 / 关 |
| 展示思考内容 | 从不 / 调试模式 / 总是折叠 |
| 保留推理上下文 | 自动 / 开 / 关 |

## 14.2 映射表

| 统一值 | OpenAI | DeepSeek | Kimi | 智谱 GLM | Anthropic | Gemini |
|---|---|---|---|---|---|---|
| 关闭 | `reasoning.effort=none` | `thinking.type=disabled` | `thinking.type=disabled` | `thinking.type=disabled` | 不传 thinking 或关闭 | `thinkingLevel=minimal` |
| 低 | `low` | `reasoning_effort=low` | 默认或 disabled | thinking enabled + 低输出预算 | `output_config.effort=low` | `thinkingLevel=low` |
| 中 | `medium` | `reasoning_effort=medium` | 默认 thinking | thinking enabled | `output_config.effort=medium` | `thinkingLevel=medium` |
| 高 | `high` | `reasoning_effort=high` | thinking enabled + max_tokens≥16000 | thinking enabled | `output_config.effort=high` | `thinkingLevel=high` |
| 极高 | `xhigh` | `reasoning_effort=high` + 更大预算 | thinking enabled + 更大预算 | thinking enabled + 更大预算 | `output_config.effort=max` | `thinkingLevel=high` + 更大预算 |

---

# 15. 统一 JSON / 结构化输出策略

## 15.1 使用场景

1. 一致性检查。
2. 角色卡生成。
3. 世界规则抽取。
4. 旧稿资产抽取。
5. 章节摘要生成。
6. 伏笔与叙事义务抽取。

## 15.2 Provider 映射

| 供应商 | JSON Object | JSON Schema | 建议实现 |
|---|---|---|---|
| OpenAI | 支持 | 支持 | 优先使用结构化输出 |
| DeepSeek | 支持 JSON Output | 视兼容性 | 使用 `response_format` |
| Kimi | OpenAI-compatible，支持情况需实测 | 需实测 | 先 JSON Object，再本地校验 |
| 智谱 GLM | 支持 `response_format: {type: json_object}` | 需实测 | 一致性检查可用 |
| MiniMax | OpenAI route 需实测，Anthropic route 无统一 JSON schema | 需实测 | Prompt 约束 + 本地 JSON 修复 |
| Anthropic | 可通过提示和工具 schema 实现 | 工具输入 schema 强 | 推荐工具化结构化输出 |
| Gemini | 支持 responseMimeType / responseSchema | 支持 | 推荐用于资产抽取 |

## 15.3 本地 JSON 修复流程

```text
模型输出
  ↓
尝试 JSON.parse
  ↓ 成功
Schema 校验
  ↓ 失败
调用 json_repair_prompt 或本地 repair
  ↓
再次校验
  ↓ 仍失败
返回原文 + 错误提示 + 不写入项目
```

---

# 16. Token 预算策略

## 16.1 Token Budget 输入

每次调用前估算：

```ts
interface TokenBudgetPlan {
  modelContextWindow: number;
  reservedForOutput: number;
  reservedForReasoning: number;
  availableForInput: number;
  projectContextTokens: number;
  userPromptTokens: number;
  shouldCompress: boolean;
  shouldRetrieveLess: boolean;
}
```

## 16.2 推荐输出预算

| 任务 | 推荐 max output |
|---|---:|
| 一句话建议 | 512-1024 |
| 角色卡 | 2048-4096 |
| 世界规则 | 2048-4096 |
| 章节计划 | 4096-8192 |
| 章节草稿 3000 中文字 | 8192-12000 |
| 章节草稿 8000 中文字 | 16000-24000 |
| 一致性检查 JSON | 4096-16000 |
| 全书摘要 | 16000-64000 |

## 16.3 长上下文策略

| 模型上下文 | 策略 |
|---|---|
| ≤128K | 必须使用检索式上下文，不能塞全书 |
| 200K-256K | 可塞蓝图 + 当前卷 + 相关章节摘要 |
| 1M | 可做全书级审查，但仍建议分批，因为成本高、延迟高 |

---

# 17. NovelForge 默认模型推荐

## 17.1 国内默认组合

| 任务 | 默认模型 | 备用模型 |
|---|---|---|
| 快速续写 | `deepseek-v4-flash` | `glm-4.7-flashx` |
| 高质量章节草稿 | `glm-5.1` | `kimi-k2.6` |
| 长上下文全书审查 | `deepseek-v4-pro` | `glm-4-long` |
| 角色对话 / 情绪 | `MiniMax-M2.7` | `kimi-k2.6` |
| 结构化资产抽取 | `glm-5.1` | `deepseek-v4-flash` |
| 多模态理解 | `glm-5v-turbo` | `kimi-k2.6` |

## 17.2 全球默认组合

| 任务 | 默认模型 | 备用模型 |
|---|---|---|
| 综合最强 | `gpt-5.5` | `claude-opus-4-7` |
| 高质量写作 / 审稿 | `claude-sonnet-4-6` | `gpt-5.5` |
| 超长上下文 | `gpt-5.5` | `gemini-3.1-pro-preview` |
| 多模态分析 | `gemini-3.1-pro-preview` | `gpt-5.5` |
| 成本敏感批处理 | `claude-haiku-4-5` | `gemini-3.1-flash-lite-preview` |
| Agent / 工具调用 | `gpt-5.5` | `claude-sonnet-4-6` |

---

# 18. 模型设置页 UI 设计

## 18.1 页面结构

```text
设置
└── 模型供应商
    ├── Provider 列表
    │   ├── DeepSeek
    │   ├── Kimi
    │   ├── 智谱 GLM
    │   ├── MiniMax
    │   ├── OpenAI
    │   ├── Anthropic
    │   ├── Google Gemini
    │   └── 自定义 Provider
    └── Provider 详情
        ├── 基础配置
        ├── 模型列表
        ├── 能力开关
        ├── 参数预设
        ├── 测试连接
        └── 使用场景默认分配
```

## 18.2 每个供应商卡片

显示：

- 供应商名称。
- 当前默认模型。
- 连接状态。
- API Key 是否已保存。
- 支持能力图标：流式、JSON、工具、思考、多模态、长上下文。
- 最近一次测试时间。
- 最近一次模型列表刷新时间。
- 模型注册表版本。

卡片按钮：

- 「配置」
- 「测试连接」
- 「刷新模型列表」
- 「查看能力报告」
- 「设为默认」

## 18.3 手动刷新模型列表流程

用户在模型设置页点击「刷新模型列表」时：

```text
点击刷新模型列表
  ↓
检查 Provider 是否启用
  ↓
检查 API Key / Base URL / Endpoint 是否完整
  ↓
优先调用供应商官方模型列表接口
  ↓
如果无官方模型列表接口，则读取内置模型注册表
  ↓
如果是自定义 Provider，则尝试 /models 或用户配置的模型列表路径
  ↓
合并本地用户自定义模型
  ↓
标记新增 / 变更 / 已弃用 / 不可用模型
  ↓
用户确认是否更新默认模型
  ↓
写入 llm_models 与刷新记录
```

刷新结果 UI 需要显示：

| 字段 | 说明 |
|---|---|
| 新增模型 | 本次发现但本地不存在的模型 |
| 更新模型 | 上下文、最大输出、能力等发生变化 |
| 已弃用模型 | 注册表或官方文档标记 deprecated |
| 保留的自定义模型 | 用户手动添加，不被自动删除 |
| 刷新失败原因 | 网络、认证、接口不兼容等 |

## 18.4 测试连接流程

```text
点击“测试连接”
  ↓
检查 API Key 是否存在
  ↓
发送最小请求：请回答 OK
  ↓
检查响应格式
  ↓
可选测试 stream
  ↓
可选测试 JSON
  ↓
可选测试 tool calling
  ↓
显示能力报告
```

## 18.4 使用场景默认模型分配

设置页增加“任务路由”：

| 任务类型 | 可选默认模型 |
|---|---|
| 章节草稿 | 用户选择 |
| 章节续写 | 用户选择 |
| 局部改写 | 用户选择 |
| 去 AI 味 | 用户选择 |
| 角色生成 | 用户选择 |
| 世界观生成 | 用户选择 |
| 一致性检查 | 用户选择 |
| 全书审稿 | 用户选择 |
| 多模态分析 | 用户选择 |

---

# 19. 模型列表刷新与内置模型注册表热更新设计

## 19.1 设计目标

模型信息变化很快，因此文火 NovelForge 不能把模型列表永久硬编码在代码里。系统必须同时支持：

1. **手动刷新模型列表**：用户在设置页主动刷新某个 Provider 的模型列表。
2. **内置模型注册表热更新**：软件从官方维护的远程 JSON 注册表拉取最新模型元数据。
3. **本地模型覆盖**：用户可以手动修改上下文、最大输出、能力开关。
4. **自定义模型保留**：用户手动添加的模型不会被远程注册表删除。
5. **离线可用**：没有网络时仍可使用随安装包内置的模型注册表。

## 19.2 模型信息来源优先级

从高到低：

1. 用户手动覆盖配置。
2. 官方 Provider `/models` 或等价模型列表接口返回。
3. NovelForge 远程模型注册表。
4. NovelForge 安装包内置模型注册表。
5. 用户手动输入。

合并规则：

```text
finalModel = builtInRegistryModel
  + remoteRegistryPatch
  + providerLiveModelPatch
  + userOverride
```

## 19.3 内置模型注册表文件

安装包内置：

```text
resources/model-registry/llm-model-registry.json
```

远程热更新：

```text
https://updates.novelforge.app/llm-model-registry.json
```

> 实际域名以后替换为正式官网域名。MVP 可以先使用 GitHub Releases / 静态 CDN。

## 19.4 注册表 JSON Schema

```json
{
  "schemaVersion": "1.0.0",
  "registryVersion": "2026.04.26.001",
  "updatedAt": "2026-04-26T12:00:00+08:00",
  "minAppVersion": "0.1.0",
  "providers": [
    {
      "vendor": "deepseek",
      "displayName": "DeepSeek",
      "defaultBaseUrl": "https://api.deepseek.com",
      "protocols": ["openai_chat_completions", "anthropic_messages"],
      "models": [
        {
          "modelName": "deepseek-v4-flash",
          "displayName": "DeepSeek V4 Flash",
          "contextWindowTokens": 1000000,
          "maxOutputTokens": 384000,
          "inputModalities": ["text"],
          "outputModalities": ["text", "json"],
          "supportsStreaming": true,
          "supportsTools": true,
          "supportsJsonObject": true,
          "supportsJsonSchema": false,
          "supportsThinking": true,
          "supportsReasoningEffort": true,
          "supportsPromptCache": false,
          "status": "recommended",
          "recommendedFor": ["chapter_draft", "long_context", "consistency_scan"]
        }
      ]
    }
  ],
  "signing": {
    "algorithm": "ed25519",
    "signature": "base64-signature"
  }
}
```

## 19.5 热更新安全要求

1. 远程注册表必须使用 HTTPS。
2. 注册表必须有签名。
3. 客户端必须校验签名后再应用。
4. 注册表更新失败不得影响已有模型配置。
5. 注册表不得携带用户 API Key、个人数据或遥测 ID。
6. 注册表只更新模型元数据，不执行代码。
7. 用户可在设置中关闭自动检查注册表更新。

## 19.6 热更新触发时机

| 触发方式 | 说明 |
|---|---|
| 软件启动后延迟检查 | 启动 30 秒后检查，不阻塞启动 |
| 用户手动检查 | 设置页点击「检查模型注册表更新」 |
| Provider 设置页刷新 | 点击某供应商「刷新模型列表」时同步检查注册表 |
| 版本更新后 | 安装新版本后首次启动检查 |

## 19.7 热更新频率

默认策略：

- 自动检查：每 7 天最多一次。
- 手动检查：不限制，但应做 30 秒防抖。
- 请求超时：10 秒。
- 失败重试：最多 1 次。

## 19.8 模型列表刷新策略

### 19.8.1 官方模型列表接口优先

如果 Provider 支持模型列表接口：

| Provider | 刷新方式 |
|---|---|
| OpenAI | `GET /v1/models` |
| OpenAI-compatible | `GET {baseUrl}/models` |
| 自定义 OpenAI-compatible | `GET {baseUrl}/models` 或用户配置路径 |
| Anthropic | 优先注册表；如后续官方接口可用再接入 |
| 自定义 Anthropic-compatible | 优先手动 / 注册表；如果网关提供 `/models` 可配置 |
| Gemini | 可调用模型列表接口或使用注册表 |
| DeepSeek / Kimi / 智谱 / MiniMax | 若官方列表接口不可用或不完整，则使用注册表 + 用户手动模型 |

### 19.8.2 合并策略

| 情况 | 处理 |
|---|---|
| 新模型出现 | 添加到 `llm_models`，状态 `available` 或 `recommended` |
| 模型上下文变化 | 更新，但保留用户覆盖字段 |
| 模型最大输出变化 | 更新，但保留用户覆盖字段 |
| 模型被标记 deprecated | 状态设为 `deprecated`，不自动删除 |
| 模型从官方列表消失 | 状态设为 `unknown`，提示用户确认 |
| 用户手动添加模型 | 标记 `source=user`，永不被自动删除 |

## 19.9 数据库表补充

### 19.9.1 llm_model_registry_state

```sql
CREATE TABLE llm_model_registry_state (
  id TEXT PRIMARY KEY,
  registry_version TEXT,
  registry_updated_at TEXT,
  last_checked_at TEXT,
  last_applied_at TEXT,
  source TEXT NOT NULL DEFAULT 'bundled',
  signature_valid INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  error_message TEXT
);
```

### 19.9.2 llm_model_refresh_logs

```sql
CREATE TABLE llm_model_refresh_logs (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL,
  refresh_type TEXT NOT NULL,
  status TEXT NOT NULL,
  added_count INTEGER NOT NULL DEFAULT 0,
  updated_count INTEGER NOT NULL DEFAULT 0,
  deprecated_count INTEGER NOT NULL DEFAULT 0,
  unknown_count INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  error_message TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES llm_providers(id)
);
```

### 19.9.3 llm_models 字段扩展

`llm_models` 需要增加：

```sql
ALTER TABLE llm_models ADD COLUMN source TEXT NOT NULL DEFAULT 'registry';
ALTER TABLE llm_models ADD COLUMN user_overridden INTEGER NOT NULL DEFAULT 0;
ALTER TABLE llm_models ADD COLUMN last_seen_at TEXT;
ALTER TABLE llm_models ADD COLUMN registry_version TEXT;
```

## 19.10 相关服务

### 19.10.1 ModelRegistryService

方法：

```ts
checkRemoteRegistryUpdate(): Promise<RegistryCheckResult>
applyRegistryUpdate(registryJson): Promise<RegistryApplyResult>
loadBundledRegistry(): Promise<ModelRegistry>
verifyRegistrySignature(registryJson): Promise<boolean>
mergeRegistryModels(providerId): Promise<ModelMergeResult>
```

### 19.10.2 ModelRefreshService

方法：

```ts
refreshProviderModels(providerId): Promise<ModelRefreshResult>
refreshAllEnabledProviders(): Promise<ModelRefreshSummary>
detectCustomProviderCapabilities(providerId): Promise<CapabilityReport>
mergeLiveModelsWithRegistry(providerId, liveModels): Promise<ModelMergeResult>
```

## 19.11 Provider Adapter 实现任务拆分

### 19.11.1 第一阶段：MVP 必须实现

1. OpenAI-compatible 基础 Adapter。
2. DeepSeek Adapter。
3. Kimi Adapter。
4. 智谱 GLM Adapter。
5. OpenAI Adapter。
6. 自定义 OpenAI-compatible Provider。
7. API Key 加密保存。
8. 模型设置 UI。
9. 手动刷新模型列表基础能力。
10. 本地内置模型注册表读取。
11. 流式输出统一事件。
12. 统一错误结构。

### 19.11.2 第二阶段：Beta 实现

1. Anthropic Adapter。
2. Gemini Adapter。
3. MiniMax Anthropic-compatible Adapter。
4. 自定义 Anthropic-compatible Provider。
5. Provider 能力检测。
6. `/models` 模型列表刷新。
7. 远程模型注册表热更新。
8. 注册表签名校验。
9. JSON Schema 统一校验。
10. Tool Calling 统一抽象。
11. Thinking 内容折叠与保留。

### 19.11.3 第三阶段：v1.0 实现

1. 模型价格管理。
2. Token 成本估算。
3. 多模型自动路由。
4. 失败自动降级。
5. 并发限流。
6. Batch API。
7. 长任务后台执行。
8. 供应商能力检测定时刷新。

# 20. 统一错误结构

```ts
export interface LlmError {
  providerId: string;
  model?: string;
  code:
    | 'missing_api_key'
    | 'invalid_api_key'
    | 'insufficient_quota'
    | 'rate_limited'
    | 'model_not_found'
    | 'context_length_exceeded'
    | 'max_output_exceeded'
    | 'content_policy_violation'
    | 'network_timeout'
    | 'stream_interrupted'
    | 'invalid_json_response'
    | 'unsupported_feature'
    | 'unknown';
  message: string;
  rawStatusCode?: number;
  retryable: boolean;
  suggestedAction?: string;
}
```

## 20.1 错误提示示例

| 错误 | 用户提示 |
|---|---|
| API Key 缺失 | 请先在模型设置中填写 API Key。 |
| 模型不存在 | 当前模型名不可用，请刷新模型列表或检查拼写。 |
| 上下文超限 | 当前上下文过长，建议减少章节范围或启用摘要压缩。 |
| 输出超限 | 当前输出预算过小，建议增大最大输出 Token。 |
| 限流 | 供应商限流，请稍后重试或切换模型。 |
| JSON 解析失败 | 模型返回的结构化数据不合法，已保留原始结果，请重试。 |

---

# 21. 数据库表设计补充

LLM 接入需要在原有项目数据库之外，增加应用级模型配置数据库。建议这些表保存到应用级配置数据库，而不是单个小说项目数据库，避免每个项目重复配置 API Key 与 Provider。

## 21.1 llm_providers

```sql
CREATE TABLE llm_providers (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  vendor TEXT NOT NULL,
  protocol TEXT NOT NULL,
  base_url TEXT NOT NULL,
  endpoint_path TEXT,
  api_key_secret_ref TEXT,
  auth_mode TEXT NOT NULL DEFAULT 'bearer',
  auth_header_name TEXT,
  anthropic_version TEXT,
  beta_headers TEXT,
  custom_headers TEXT,
  default_model TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  timeout_ms INTEGER NOT NULL DEFAULT 120000,
  connect_timeout_ms INTEGER NOT NULL DEFAULT 15000,
  max_retries INTEGER NOT NULL DEFAULT 2,
  model_refresh_mode TEXT NOT NULL DEFAULT 'registry',
  models_path TEXT,
  last_model_refresh_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

## 21.2 llm_models

```sql
CREATE TABLE llm_models (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL,
  model_name TEXT NOT NULL,
  display_name TEXT,
  context_window_tokens INTEGER,
  max_output_tokens INTEGER,
  input_modalities TEXT,
  output_modalities TEXT,
  supports_streaming INTEGER NOT NULL DEFAULT 0,
  supports_tools INTEGER NOT NULL DEFAULT 0,
  supports_json_object INTEGER NOT NULL DEFAULT 0,
  supports_json_schema INTEGER NOT NULL DEFAULT 0,
  supports_thinking INTEGER NOT NULL DEFAULT 0,
  supports_reasoning_effort INTEGER NOT NULL DEFAULT 0,
  supports_prompt_cache INTEGER NOT NULL DEFAULT 0,
  supports_batch INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'available',
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES llm_providers(id)
);
```

## 21.3 llm_task_routes

```sql
CREATE TABLE llm_task_routes (
  id TEXT PRIMARY KEY,
  task_type TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
  fallback_provider_id TEXT,
  fallback_model_id TEXT,
  reasoning_mode TEXT,
  max_output_tokens INTEGER,
  temperature REAL,
  top_p REAL,
  stream INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

---

# 22. LLM Agent 开发执行提示词

```text
你是文火 NovelForge 的 LLM Provider 接入 Agent。请按照《LLM Provider 接入设计文档 v1.0》实现模型供应商接入层。优先实现统一 ProviderConfig、ModelConfig、UnifiedGenerateRequest、UnifiedGenerateResponse、LlmError、OpenAI-compatible Adapter、DeepSeek Adapter、Kimi Adapter、Zhipu Adapter、OpenAI Responses Adapter、自定义 OpenAI-compatible Provider、自定义 Anthropic-compatible Provider，以及 API Key 的 Windows Credential Manager 安全存储。所有 Provider 调用都必须经过统一 LlmService，业务层不得直接调用任何供应商 SDK。实现时必须支持流式输出、错误标准化、API Key 脱敏、模型能力注册、手动刷新模型列表、内置模型注册表读取、远程模型注册表热更新、任务路由和 fallback。每完成一个 Adapter，必须提供测试连接、普通文本生成、流式生成、JSON 输出、错误处理测试。text
你是文火 NovelForge 的 LLM Provider 接入 Agent。请按照《LLM Provider 接入设计文档 v1.0》实现模型供应商接入层。优先实现统一 ProviderConfig、ModelConfig、UnifiedGenerateRequest、UnifiedGenerateResponse、LlmError、OpenAI-compatible Adapter、DeepSeek Adapter、Kimi Adapter、Zhipu Adapter、OpenAI Responses Adapter，以及 API Key 的 Windows Credential Manager 安全存储。所有 Provider 调用都必须经过统一 LlmService，业务层不得直接调用任何供应商 SDK。实现时必须支持流式输出、错误标准化、API Key 脱敏、模型能力注册、任务路由和 fallback。每完成一个 Adapter，必须提供测试连接、普通文本生成、流式生成、JSON 输出、错误处理测试。
```

# 23. 统一开发红线

1. 不得把真实 API Key 写入数据库明文字段。
2. 不得把 API Key 写入项目目录。
3. 不得在日志中打印 Authorization Header。
4. 不得在 UI 中显示完整 API Key。
5. 不得假设所有 OpenAI-compatible Provider 都支持工具调用。
6. 不得假设所有模型都支持 JSON Schema。
7. 不得把 reasoning_content 原样展示给普通用户，默认折叠或隐藏。
8. 不得让 AI 结果自动覆盖用户正文。
9. 不得在上下文超限时静默截断关键设定。
10. 不得把旧模型作为新用户默认模型。

