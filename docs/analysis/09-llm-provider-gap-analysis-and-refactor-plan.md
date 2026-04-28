# NovelForge LLM Provider 集成：差距分析与重构方案

> 本文档基于 `docs/novelforge_llm_provider_integration_spec_v_1.md`（以下简称"Spec"）
> 对比当前代码实现状态，定位导致 **"AI 建议生成失败"** 的根本原因，
> 并提供分阶段的重构方案。

**分析日期：** 2026-04-28  
**分析范围：** 前端 TypeScript + Rust 后端全链路  
**当前状态：** AI 调用链路不通（"AI 建议生成失败"）

---

## 目录

1. [问题概述：为什么 AI 建议生成失败](#1-问题概述为什么-ai-建议生成失败)
2. [根本原因追踪](#2-根本原因追踪)
3. [Spec 实现差距全表](#3-spec-实现差距全表)
4. [P0 级 Bug 详细分析](#4-p0-级-bug-详细分析)
5. [P1 级架构差距](#5-p1-级架构差距)
6. [P2 级缺失功能](#6-p2-级缺失功能)
7. [分阶段重构方案](#7-分阶段重构方案)
8. [风险与注意事项](#8-风险与注意事项)
9. [附录：全链路调用流程图](#9-附录全链路调用流程图)

---

## 1. 问题概述：为什么 AI 建议生成失败

### 用户操作路径

```
SettingsPage: 填写 API Key → 保存 Provider → OpenRouter 返回 {ok}
    ↓
EditorPage: 点击 "生成草稿" / "续写" / "改写"
    ↓
结果: "AI 建议生成失败"
```

### 一句话结论

**根本原因：保存 Provider 时没有自动创建任务路由（Task Route）。**

用户在设置页保存 Provider 后，系统只保存了 Provider 配置 + API Key。当在编辑器点击 AI 功能时，`AiService::resolve_route("chapter_draft")` 查询 `llm_task_routes` 表——但该表为空——直接返回 `"TASK_ROUTE_NOT_FOUND"` 错误。

**用户必须在「任务路由」Tab 中手动为每个任务类型指定 Provider/Model，AI 调用才能正常工作。** 此设计缺陷是导致"AI 建议生成失败"的唯一直接原因。

---

## 2. 根本原因追踪

### 全链路追溯：从点击到错误

```
EditorPage.tsx
  └─ handleAiCommand("chapter_draft")
      └─ streamAiChapterTask({ projectRoot, chapterId, taskType: "chapter_draft" })
          └─ Tauri invoke → stream_ai_chapter_task (ai_commands.rs:136)
              │
              ├─ 1. collect_chapter_context() → OK
              ├─ 2. PromptBuilder::build_chapter_draft() → OK
              ├─ 3. UnifiedGenerateRequest { model: "default", task_type: Some("chapter_draft"), provider_id: None }
              │
              └─ 4. AiService::stream_generate(req) (ai_service.rs:206)
                  └─ resolve_request_target(req) (ai_service.rs:120)
                      │
                      ├─ provider_hint = "" (None)
                      ├─ should_route_by_task = true
                      │
                      └─ resolve_route("chapter_draft") (ai_service.rs:105)
                          ├─ 查询 llm_task_routes 表
                          ├─ task_type = "chapter_draft" 的行 → NOT FOUND
                          │
                          └─ 返回 Err(AppErrorDto {
                              code: "TASK_ROUTE_NOT_FOUND",
                              message: "No route configured for task type 'chapter_draft'",
                              recoverable: true
                          })
```

### 为什么路由表为空？

`save_provider` 命令（settings_commands.rs:125）的执行流程：

```
save_provider (settings_commands.rs:125)
  └─ SettingsService::save_provider(config, api_key) (settings_service.rs:49)
      ├─ validate_provider_config() → OK
      ├─ credential_manager::save_api_key() → key saved
      └─ app_database::upsert_provider() → provider saved to llm_providers
  └─ AiService::reload_provider() → adapter registered in runtime cache
  ✗  NEVER creates task routes ← 这就是问题！
```

**前端同样没有自动创建路由的逻辑。** 初始化时路由 Tab 显示的是空的表单，需要用户逐个保存。

### 连锁并发症

即使修复了路由问题，还有第二个严重问题：

**`stream_generate()` 的 tokio::spawn 任务静默吞掉所有错误。**

```rust
// ai_service.rs:220
tokio::spawn(async move {
    for (attempt_provider, attempt_model) in attempts {
        // ...尝试注册、调用...
        if adapter.stream_text(attempt_req, tx.clone()).await.is_ok() {
            return;
        }
        // 如果 stream_text 返回 Err，静默继续下一个尝试
    }
    // 所有尝试都失败后，spawn 任务直接退出
    // mpsc::Receiver 永远收不到任何消息
    // 前端在 AsyncGenerator 中永久等待下一个 chunk
});
```

这意味着：**即使解决了路由问题，如果实际 API 调用失败（网络错误、认证失败、模型不存在），前端将永远卡在 "streaming" 状态，无法超时也无法显示错误。**

---

## 3. Spec 实现差距全表

| # | Spec 要求 | 当前状态 | 优先级 | 影响 |
|---|---|---|---|---|
| 1 | 保存 Provider 后自动创建默认任务路由 | ❌ 未实现 | **P0** | AI 调用全链路断裂 |
| 2 | 流式调用错误回传前端 | ❌ `stream_generate` 静默吞错误 | **P0** | 卡死在"streaming"状态 |
| 3 | 启动时预加载已配置的 Provider | ❌ 启动时 adapter 注册表为空 | **P0** | 首次调用延迟 + 新增错误路径 |
| 4 | 内置模型注册表 `llm-model-registry.json` | ❌ 文件不存在 | **P1** | 模型信息依赖硬编码 |
| 5 | API Key 掩码 `sk-1234••••••••••••efgh` | ⚠️ 只显示首4+尾4 | **P1** | 不符合安全预期 |
| 6 | 中文错误提示（§20.1） | ❌ 全部英文 | **P1** | 用户不友好 |
| 7 | `detect_capabilities()` 真实探测 | ❌ 乐观返回 true | **P1** | 能力报告不准确 |
| 8 | `StreamChunk` 支持错误事件 | ❌ 只含 content/finish_reason/request_id | **P1** | 无法在流中传递错误 |
| 9 | MiniMax 默认改为 `Bearer` 认证 | ⚠️ authMode="bearer" 已设但对 AnthropicAdapter 默认分支不匹配 | **P1** | MiniMax 可能认证失败 |
| 10 | Remote registry 热更新 UI | ⚠️ 后端命令存在但前端无入口 | **P2** | 模型列表无自动更新 |
| 11 | 模型列表展示与能力面板 | ❌ 刷新后不展示结果 | **P2** | 用户看不到注册的模型 |
| 12 | Thinking/Reasoning 内容折叠 | ❌ 未处理 | **P2** | reasoning 内容暴露 |
| 13 | Token 预算计算 | ❌ 未实现 | **P2** | 无上下文长度保护 |
| 14 | 任务路由自动降级（fallback） | ⚠️ 后端支持但未测试 | **P2** | fallback 路径可能不通 |
| 15 | 自定义 Provider 能力检测 | ❌ 只测文本，不测其余能力 | **P2** | 能力报告不完整 |

---

## 4. P0 级 Bug 详细分析

### Bug #1: 保存 Provider 后未自动创建任务路由

**位置：**
- `src-tauri/src/commands/settings_commands.rs:125` (`save_provider`)
- `src-tauri/src/services/settings_service.rs:49` (`save_provider`)

**表现：** 用户在设置页保存 Provider 后，`llm_task_routes` 表为空。所有 AI 命令（`stream_ai_chapter_task`、`generate_blueprint_suggestion`、`ai_generate_character` 等）都调用 `resolve_route()`，返回 `TASK_ROUTE_NOT_FOUND`。

**修复方案（最小改动）：**

在 `save_provider` 命令成功保存 Provider 后，自动为所有 9 个任务类型创建默认路由：

```rust
// 在 settings_commands.rs save_provider 中，reload_provider 之后：
let default_model = saved.default_model.as_deref().unwrap_or("");
if !default_model.is_empty() {
    let conn = app_database::open_or_create()?;
    let task_types = [
        "chapter_draft", "chapter_continue", "chapter_rewrite",
        "prose_naturalize", "character.create", "world.generate",
        "consistency.scan", "blueprint.generate_step", "plot.generate",
    ];
    let now = crate::infra::time::now_iso();
    for tt in &task_types {
        let existing = app_database::load_task_routes(&conn)?;
        if !existing.iter().any(|r| r.task_type == *tt) {
            let route = TaskRoute {
                id: uuid::Uuid::new_v4().to_string(),
                task_type: tt.to_string(),
                provider_id: saved.id.clone(),
                model_id: default_model.to_string(),
                fallback_provider_id: None,
                fallback_model_id: None,
                max_retries: 1,
                created_at: Some(now.clone()),
                updated_at: Some(now.clone()),
            };
            app_database::upsert_task_route(&conn, &route, &now)?;
        }
    }
}
```

如果不想破坏 `SettingsService` 的无状态特性，也可以在 `save_provider` 命令层（settings_commands.rs）实现。

### Bug #2: 流式调用错误被静默吞噬

**位置：** `src-tauri/src/services/ai_service.rs:220-246`

**表现：** `stream_generate()` 启动一个 `tokio::spawn` 任务，其中所有错误都被 `continue` 或 `if is_ok()` 条件静默忽略。如果所有尝试都失败，spawn 任务直接结束。前端 `mpsc::Receiver` 永不会收到任何 chunk，也不会收到错误信号。AsyncGenerator 永久挂起。

**现有的 AiStreamEvent 类型（前端）** 包含了 `type: 'error'` 事件，但后端从未发送该事件。

**修复方案：**

1. 扩展 `StreamChunk` 结构体，增加 `error` 字段（Option<String>）
2. 在 spawn 任务中，即使遇到错误也通过 `tx` 发送错误信号
3. 或者在 `stream_generate` 中使用 `oneshot` channel 并行返回错误

最小改动方案：在 `StreamChunk` 中加入 `error` 字段：

```rust
// llm_types.rs
pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
    pub request_id: String,
    pub error: Option<String>,  // 新增
}
```

在 spawn 任务中发送错误：

```rust
if service.ensure_provider_registered(&attempt_provider).await.is_err() {
    let _ = tx.send(StreamChunk {
        content: String::new(),
        finish_reason: None,
        request_id: String::new(),
        error: Some("Provider not registered".to_string()),
    }).await;
    continue;
}
```

### Bug #3: 启动时 Provider 未预加载

**位置：** `src-tauri/src/state.rs:26` (`AiService` 在 `AppState::default()` 中初始化)

**表现：** `AiService.default()` 创建空的 `HashMap<String, Box<dyn LlmService>>`。第一个 AI 请求必须经过 `ensure_provider_registered()` 的懒加载路径，该路径：
1. 打开 app-level 数据库
2. 查询 Provider 配置
3. 加载 Credential Manager API Key
4. 创建 Adapter 实例
5. 写入 runtime cache

这是一个同步的数据库 + 系统调用，耗时长且如果数据库损坏或 Credential Manager 不可用，首次 AI 调用会失败。

**修复方案：** 在 `lib.rs` 的 `setup` 回调中预加载所有 enabled 的 Provider：

```rust
// lib.rs setup 中
let state = AppState::default();
app.manage(state);
// 延迟预加载（不阻塞启动）
let app_handle = app.handle().clone();
tokio::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    if let Ok(conn) = app_database::open_or_create() {
        if let Ok(providers) = app_database::load_all_providers(&conn) {
            for provider in &providers {
                // reload_provider 会注册 adapter
                let _ = state.ai_service.reload_provider(&provider.id).await;
            }
        }
    }
});
```

但注意 `AppState` 需要从 `app.manage()` 之后提取。在 Tauri 2 中，可以通过 `app.state::<AppState>()` 获取。

---

## 5. P1 级架构差距

### Gap #4: 内置模型注册表文件缺失

**Spec 要求：** §19.3 — `resources/model-registry/llm-model-registry.json`

**现状：** 文件不存在。`ModelRegistryService` 的 `refresh_provider_models()` 尝试从 API 拉取模型列表，如果失败则回退到 `default_model`。没有"内置注册表 → 远程补丁 → 用户覆盖"的层级合并。

**影响：** 用户保存 Provider 后，"刷新模型"操作可能什么都刷不到，`llm_models` 表保持为空。但这不影响核心 AI 调用（因为路由只需要 provider_id + model_id 字符串，不需要 `llm_models` 中有记录）。

### Gap #5: API Key 掩码不符规范

**Spec 要求：**
```
原始：sk-proj-1234567890abcdefghijklmn
显示：sk-proj-1234••••••••••••lmn
```

**现状：** `mask_api_key()` 在 `settings_service.rs:153`：
```rust
fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}••••{}", &key[..4], &key[key.len() - 4..])
    }
}
```
显示为 `sk-p••••q2a`（首4+尾4），而不是前6+中间掩码+后4。

**影响：** 用户无法区分 `sk-proj-...` 和 `sk-ant-...` 类型的 key。

### Gap #6: 错误信息未本地化

**Spec §20.1 要求中文提示：**
| 错误 | 用户提示 |
|---|---|
| API Key 缺失 | 请先在模型设置中填写 API Key。 |
| 模型不存在 | 当前模型名不可用，请刷新模型列表或检查拼写。 |
| 上下文超限 | 当前上下文过长，建议减少章节范围或启用摘要压缩。 |

**现状：** `LlmError::from()` 全部返回英文（如 `"API key not configured for this provider"`）。前端没有翻译层。

**影响：** 用户看到英文错误提示，不友好。但这不是功能性问题。

### Gap #7: `detect_capabilities()` 过于乐观

**Spec §13.1.4 要求：** 逐项测试文本、流式、JSON、Schema、工具、视觉。

**现状：** `OpenAiCompatibleAdapter::detect_capabilities()` 只测试了文本生成。如果文本测试通过，streaming/json/tools 直接设为 `true`，不做实际探测。

**影响：** 如果 Provider 不支持 streaming 但能力报告说支持，调用会失败。

### Gap #8: `StreamChunk` 缺少错误字段

**Spec §13.2.7 要求的统一流式事件：**
```typescript
interface AiStreamEvent {
  type: 'start' | 'delta' | 'thinking_delta' | 'tool_call' | 'done' | 'error';
  error?: LlmError;
}
```

**现状：** Rust 端 `StreamChunk` 没有 `error` 字段。前端 `AiStreamEvent` 有 `type: 'error'` 但从未触发。

**修复方案：** 见 Bug #2 的修复方案。

### Gap #9: AnthropicAdapter 默认认证模式对 MiniMax 不匹配

**MiniMax Spec §9.1：** 认证方式 `Authorization: Bearer ${MINIMAX_API_KEY}`  
**AnthropicAdapter：** Anthropic 原生使用 `x-api-key` 认证头  

在 `build_headers()` 中：
```rust
match self.config.auth_mode.as_str() {
    "bearer" => vec![("Authorization", "Bearer {key}")],  // MiniMax 走这里
    "x-api-key" | "x_api_key" => vec![("x-api-key", key)],
    _ => vec![("x-api-key", key)],  // Anthropic 原生走这里
}
```

**现状：** MiniMax preset 已设置 `authMode: "bearer"`，所以 MiniMax 用户的适配器使用 Bearer token。**代码逻辑正确。** 但当用户修改 `authMode` 为 `x-api-key` 时，MiniMax 将无法认证。

**风险：** 低。除非用户手动修改 MiniMax 的 `authMode`。但自定义 Anthropic-compatible Provider 可能需要选择不同的认证模式，如果选错则无法工作。

---

## 6. P2 级缺失功能

### Gap #10: Remote Registry 无前端入口

`check_remote_registry` 和 `apply_registry_update` 命令在后端存在，但设置页没有任何 UI 入口。用户无法触发远程注册表更新。

### Gap #11: 刷新模型后无展示

`handleRefresh()` 调用 `refreshProviderModels()` 后只显示一行统计文字，不展示实际模型列表。没有"能力报告"面板。

### Gap #12: Thinking/Reasoning 内容未处理

Spec §9.3 要求："MiniMax 的响应可能包含 `<think>...</think>` 或 reasoning content，产品应支持隐藏 / 折叠推理内容。"

现状：Adapter 不做任何 reasoning content 检测和处理，直接拼接所有文本输出。

### Gap #13: Token 预算未实现

Spec §16 要求的 `TokenBudgetPlan` 全部未实现。没有上下文长度检查和预算控制。

### Gap #14: 任务路由 Fallback 路径未充分测试

后端的 `generate_text()` 和 `stream_generate()` 都有 fallback 逻辑，但由于 Bug #1（路由不存在），fallback 路径从未被执行过。可能存在未发现的 bug。

### Gap #15: 自定义 Provider 能力检测不完整

Spec §13.1.4 的 8 步能力检测流程只实现了第 2 步（短文本测试）。其他能力都乐观返回 true。

---

## 7. 分阶段重构方案

### 第一阶段：紧急修复（修复"AI 建议生成失败"）

| # | 任务 | 文件 | 复杂度 |
|---|---|---|---|
| **1.1** | `save_provider` 成功后自动创建默认任务路由 | `settings_commands.rs` | ★☆☆ |
| **1.2** | `StreamChunk` 增加 `error` 字段，spawn 任务发送错误信号 | `llm_types.rs`, `ai_service.rs` | ★★☆ |
| **1.3** | 前端处理流式错误事件：`ai:stream-error:{requestId}` | `EditorPage.tsx`, `aiApi.ts` | ★★☆ |
| **1.4** | 流式调用添加前端超时保护（30秒无 chunk 自动报错） | `aiApi.ts` | ★☆☆ |

**预计工时：** 4-6 小时

### 第二阶段：基础设施加固

| # | 任务 | 文件 | 复杂度 |
|---|---|---|---|
| **2.1** | 启动时延迟预加载已配置 Provider | `lib.rs` | ★★☆ |
| **2.2** | 创建内置模型注册表 JSON | `resources/model-registry/` | ★★☆ |
| **2.3** | API Key 掩码改为 spec 格式（前6+中间掩码+后4） | `settings_service.rs` | ★☆☆ |
| **2.4** | LlmError 消息中文化 | `llm_types.rs` | ★☆☆ |
| **2.5** | `detect_capabilities()` 增加真实探测 | `openai_compatible.rs`, `anthropic.rs` | ★★★ |

**预计工时：** 6-8 小时

### 第三阶段：功能完善

| # | 任务 | 文件 | 复杂度 |
|---|---|---|---|
| **3.1** | 模型列表展示 UI + 能力报告面板 | `SettingsPage.tsx` | ★★★ |
| **3.2** | Remote Registry 更新 UI | `SettingsPage.tsx`, `settingsApi.ts` | ★★☆ |
| **3.3** | Thinking/Reasoning 内容检测与折叠 | 各 adapter `parse_response` | ★★☆ |
| **3.4** | Token 预算计算 | 新建 `token_budget.rs` | ★★★ |
| **3.5** | 自定义 Anthropic-compatible Provider 认证模式检测 | `anthropic.rs` | ★★☆ |

**预计工时：** 12-16 小时

### 增量重构建议

**建议 1：** 不要一次性重写整个 AI 层。当前架构（Adapter Pattern + AiService 路由）是正确的。

**建议 2：** 第一阶段修复后，核心 AI 调用链路就可以工作了。第二阶段和第三阶段可以按需推进。

**建议 3：** 测试策略——每个 Provider 需要一个 mock server 的集成测试：
- `tests/openai_compatible_test.rs` — 用 `wiremock` 模拟 `chat/completions`
- `tests/anthropic_test.rs` — 模拟 SSE 流
- `tests/gemini_test.rs` — 模拟 `generateContent`

---

## 8. 风险与注意事项

### API Key 安全红线

Spec §23 开发红线必须严格遵守：
| 红线 | 当前遵守情况 |
|---|---|
| API Key 不写入数据库明文字段 | ✅ 已使用 Windows Credential Manager |
| API Key 不写入项目目录 | ✅ |
| API Key 不进入日志 | ⚠️ `tauriClient.ts` 日志中有 `redactApiKey()` 但需确认全覆盖 |
| API Key 不进入导出文件 | ✅ |
| UI 只显示掩码 | ✅ |
| 测试连接时不打印完整请求头 | ⚠️ `log_ai_call` 可能记录请求信息，需检查 |
| 错误上报不携带 Authorization Header | ⚠️ 需确认日志中没有记录 header |

### 迁移路径

**不要同时部署所有改动。** 建议按阶段推进，每阶段完成后验证：
1. 第一阶段：用户保存 Provider → 创建路由 → AI 调用成功
2. 第二阶段：启动加载正常，模型列表可刷新
3. 第三阶段：远程注册表热更新、Token 预算、Thinking 处理

### 未解决问题

- **多 Provider 并行路由策略：** 当有多个 Provider 都配置了相同的 task_type 时，如何选择？当前代码以 `llm_task_routes` 中第一个匹配的为准（`find()`）。
- **Provider 健康检查：** 没有后台健康检查机制。如果一个 Provider 持续失败，没有自动降级到备用 Provider。
- **用户配置的默认路由：** 没有"使用 X 作为所有任务的默认 Provider"一键设置。

---

## 9. 附录：全链路调用流程图

### 修复后的期望调用链路

```
EditorPage
  └─ streamAiChapterTask(input)
      └─ Tauri invoke → stream_ai_chapter_task
          ├─ context_service.collect_chapter_context() → context
          ├─ PromptBuilder::build_chapter_draft(context, instruction) → prompt
          ├─ UnifiedGenerateRequest { model: "default", task_type: Some("chapter_draft") }
          │
          └─ AiService::stream_generate(req)
              └─ resolve_request_target(req)
                  └─ resolve_route("chapter_draft")
                      ├─ llm_task_routes 查询 → 找到 (provider_id="minimax", model_id="MiniMax-M2.7")
                      │
                      └─ Ok(("minimax", "MiniMax-M2.7", Some(route)))
              │
              ├─ ensure_provider_registered("minimax")
              │   ├─ adapters HashMap 中有? No → reload_provider("minimax")
              │   │   ├─ load config from app DB (no api_key)
              │   │   ├─ load_api_key from Credential Manager → "sk-cp-..."
              │   │   ├─ AnthropicAdapter::new(config with key)
              │   │   └─ insert into adapters HashMap
              │   └─ OK
              │
              ├─ AnthropicAdapter::stream_text(req, tx)
              │   ├─ POST https://api.minimaxi.com/anthropic/v1/messages
              │   │   Headers: { Authorization: "Bearer sk-cp-...", anthropic-version: "2023-06-01" }
              │   │   Body: { model: "MiniMax-M2.7", stream: true, ... }
              │   │
              │   ├─ SSE: event: content_block_delta / data: {"delta":{"text":"..."}} 
              │   │   → tx.send(StreamChunk { content: "..." })
              │   │
              │   └─ SSE: event: message_stop → tx.send(StreamChunk { finish_reason: "end_turn" })
              │
              ├─ tokio::spawn: app.emit("ai:stream-chunk:{id}", chunk)
              │
              └─ return request_id to frontend

Frontend receives request_id → listens for "ai:stream-chunk:{id}" events
  └─ Each chunk → appendAiPreviewContent(delta)
  └─ Done event → setAiStreamStatus("completed")
  └─ Error event (新增) → setAiStreamStatus("error") with error message
```

### 修复后的数据结构

```mermaid
flowchart TD
    A[SettingsPage: Save Provider] --> B[settings_commands::save_provider]
    B --> C[upsert_provider in llm_providers]
    B --> D[save_api_key in Credential Manager]
    B --> E[reload_provider → register adapter]
    B --> F[★ NEW: create default task routes]
    F --> G[INSERT 9 routes into llm_task_routes]
    
    H[EditorPage: AI Generate] --> I[stream_ai_chapter_task]
    I --> J[resolve_route→llm_task_routes]
    J --> K[找到 provider_id + model_id]
    K --> L[ensure_provider_registered]
    L --> M[adapter.stream_text]
    M --> N[SSE stream → chunks]
    N --> O[★ NEW: error→tx.send(error chunk)]
    O --> P[frontend receives content or error]
```

---

*本文档由分析生成，建议在实施重构前由团队成员评审确认。*
