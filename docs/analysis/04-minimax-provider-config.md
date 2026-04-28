# MiniMax LLM Provider 接入配置与测试计划 v1.0

> 生成日期：2026-04-28
> 目标：配置 MiniMax-M2.7 并通过 Anthropic-compatible 协议测试 AI 流式输出

---

## 1. Provider 配置信息

### 1.1 连接参数

| 参数 | 值 |
|---|---|
| Provider ID | `minimax` |
| 供应商名称 | MiniMax |
| Vendor | `minimax` |
| 协议 | `anthropic_messages` |
| Base URL | `https://api.minimaxi.com/anthropic` |
| Endpoint | `/messages` |
| 默认模型 | `MiniMax-M2.7` |
| API Key | 已提供（掩码：`sk-cp-MIJf••••••••iEE`） |
| 认证方式 | `bearer`（`Authorization: Bearer ${KEY}`） |
| 超时 | 180,000ms |
| 连接超时 | 15,000ms |
| 最大重试 | 2 |

### 1.2 能力配置

| 能力 | 支持 |
|---|---|
| 流式输出 | ✅ 是 |
| Thinking | ✅ 是（Anthropic-compatible 原生支持） |
| 工具调用 | ✅ 是 |
| JSON 输出 | ❌ 否（Anthropic 协议无原生 JSON 模式，需 tool-based） |
| Prompt Cache | ❌ 需测试 |
| 上下文窗口 | 204,800 tokens |
| 最大输出 | 32,768 tokens（产品安全默认值） |

---

## 2. Rust 端适配器路由逻辑

根据 `build_adapter()` 函数（`settings_service.rs:232-246`），MiniMax 供应商被路由到 `AnthropicAdapter`：

```rust
match config.vendor.as_str() {
    "anthropic" | "minimax" => Box::new(AnthropicAdapter::new(config)),
    // ...
}
```

`AnthropicAdapter`（`adapters/anthropic.rs`）使用 `auth_mode` 字段来决定认证方式：
- `"bearer"` → `Authorization: Bearer ${KEY}`
- `"x-api-key"` → `x-api-key: ${KEY}`

对于 MiniMax，应使用 `"bearer"` 模式。

### 2.1 测试连接请求（Rust 端）

`test_connection()` 发送最小请求：
```json
POST https://api.minimaxi.com/anthropic/messages
Content-Type: application/json
Authorization: Bearer sk-cp-...
anthropic-version: 2023-06-01

{
  "model": "MiniMax-M2.7",
  "max_tokens": 10,
  "messages": [{"role": "user", "content": "ping"}]
}
```

---

## 3. 前端配置入口

在应用的 **设置 → 模型供应商** 页面中：

### 3.1 操作步骤

1. 打开 NovelForge 应用
2. 进入「设置」→「模型供应商」
3. MiniMax 卡片点击「配置」
4. 填写：
   - **Base URL**: `https://api.minimaxi.com/anthropic`
   - **API Key**: `sk-cp-MIJfyxIA-519-6ZfPgMi8T3IMFpry_dtzQR2LeWE5jw1uedMcEVom0SMHPQUXvMaR1JSS3awsfaKKcRzb53RpOcbLdf7rrVOWbBCSiVp0o5y12sDeG4PiEE`
   - **API 协议**: `Anthropic Messages`
   - **Model**: `MiniMax-M2.7`
   - **上下文窗口**: `204800`
   - **最大输出 Token**: `32768`
5. 点击「测试连接」验证连通性
6. 状态显示「连接成功！」后，点击「保存」

### 3.2 后续验证

完成配置后，在章节编辑器中：
1. 创建或打开一个章节
2. 在 AI 指令栏点击「生成草稿」
3. 验证流式输出是否正确显示
4. 验证「续写」「改写」「去 AI 味」功能

---

## 4. 通过 Rust 测试直接验证 API 连通性

如果需要在命令行验证，可以写一个简单的 Rust 测试：

```rust
#[tokio::test]
async fn test_minimax_connection() {
    use crate::adapters::anthropic::AnthropicAdapter;
    use crate::adapters::llm_types::ProviderConfig;

    let config = ProviderConfig {
        id: "minimax-test".into(),
        display_name: "MiniMax Test".into(),
        vendor: "minimax".into(),
        protocol: "anthropic_messages".into(),
        base_url: "https://api.minimaxi.com/anthropic".into(),
        endpoint_path: Some("/messages".into()),
        api_key: Some(std::env::var("MINIMAX_API_KEY").unwrap()),
        auth_mode: "bearer".into(),
        auth_header_name: None,
        anthropic_version: Some("2023-06-01".into()),
        beta_headers: None,
        custom_headers: None,
        default_model: Some("MiniMax-M2.7".into()),
        timeout_ms: 180000,
        connect_timeout_ms: 15000,
        max_retries: 2,
        model_refresh_mode: Some("registry".into()),
        models_path: None,
        last_model_refresh_at: None,
    };

    let adapter = AnthropicAdapter::new(config);
    let result = adapter.test_connection().await;
    assert!(result.is_ok(), "MiniMax connection failed: {:?}", result);
}
```

---

## 5. 故障排查

| 问题 | 可能原因 | 解决方案 |
|---|---|---|
| 连接超时 | 网络不可达 / Base URL 错误 | 检查 `https://api.minimaxi.com/anthropic` 是否可访问 |
| 401 Unauthorized | API Key 错误或过期 | 重新获取 API Key |
| 404 Not Found | Endpoint 路径错误 | 尝试 `/v1/messages` 而不是 `/messages` |
| 模型不存在 | `MiniMax-M2.7` 名称不正确 | 尝试 `/v1/models` 查看可用模型 |
| 流式中断 | 网络不稳定 / 超时设置过小 | 增加 `timeoutMs` 到 300000 |
| JSON 解析错误 | 响应格式与预期不符 | 检查 `anthropic-version` 头 |

---

## 6. 注意

- MiniMax OpenAI-compatible 端点的 `max_completion_tokens` 上限为 2048，不适合长章节生成
- 推荐始终使用 Anthropic-compatible 端点（`https://api.minimaxi.com/anthropic`）进行长文本生成
- 响应可能包含 `<think>...</think>` 或 reasoning content，UI 应支持隐藏 / 折叠推理内容
