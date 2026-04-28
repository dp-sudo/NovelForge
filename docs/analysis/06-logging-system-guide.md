# NovelForge 日志系统使用指南 v1.0

> 生成日期：2026-04-28
> 日志系统覆盖：Rust 后端（服务层 + 命令层）+ 前端（API 调用层 + UI 事件）

---

## 1. 架构概览

```
┌─────────────────────────────────────────────┐
│  前端 (Browser/WebView)                      │
│  ┌───────────────────────────────────────┐   │
│  │ tauriClient.ts (invokeCommand)        │   │
│  │   → 每次 API 调用自动记录              │   │
│  │   → 输出到浏览器 console               │   │
│  └───────────────────────────────────────┘   │
└──────────────────────┬──────────────────────┘
                       │ Tauri invoke
┌──────────────────────▼──────────────────────┐
│  Rust 后端 (Tauri Command Layer)             │
│  ┌───────────────────────────────────────┐   │
│  │ commands/*.rs (117 个命令处理器)        │   │
│  │   → 关键命令添加了 log_ 调用            │   │
│  └───────────────────────────────────────┘   │
│  ┌───────────────────────────────────────┐   │
│  │ infra/logger.rs                        │   │
│  │   → log_command / log_user_action      │   │
│  │   → log_ai_call / log_security         │   │
│  │   → log_service / log_fs / log_db      │   │
│  └───────────────────────────────────────┘   │
│  ┌───────────────────────────────────────┐   │
│  │ tauri-plugin-log (v2.8)               │   │
│  │   → stdout 输出 (Debug 级别)           │   │
│  └───────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

---

## 2. 日志输出位置

### 2.1 Rust 后端日志

| 目标 | 路径 | 级别 |
|---|---|---|
| 控制台 (stdout) | `tauri dev` / 终端输出 | Debug+ |
| Tauri 日志 | `~/.novelforge/logs/`（由 tauri-plugin-log 管理） | Debug+ |

### 2.2 前端日志

| 目标 | 访问方式 | 级别 |
|---|---|---|
| 浏览器 DevTools Console | F12 -> Console 面板 | 全部 |
| 捕获到 Tauri 后端 | 通过 `tauri-plugin-log` 的 webview 日志捕获 | Info+ |

---

## 3. 日志标签系统

每条日志都有一个 `[TAG]` 前缀用于快速过滤：

| 标签 | 含义 | 示例 |
|---|---|---|
| `[CMD]` | Tauri 命令调用 | `[CMD] create_project | input: name="测试小说"` |
| `[SVC]` | 服务层操作 | `[SVC] chapter_service.save_chapter | chapter=ch_001` |
| `[AI]` | AI Provider 调用 | `[AI] minimax / MiniMax-M2.7 | chapter.draft` |
| `[DB]` | 数据库操作 | `[DB] INSERT(chapters) | id=ch_001` |
| `[FS]` | 文件系统操作 | `[FS] write | manuscript/chapters/ch-0001.md` |
| `[SEC]` | 安全相关事件 | `[SEC] save_provider | provider=MiniMax` |
| `[USER]` | 用户主动操作 | `[USER] create_chapter | chapter=第一章` |
| `[NAV]` | 页面导航 | `[NAV] -> Editor` |
| `[API] >>` | 前端发起 API 调用 | `[API] >> create_project` |
| `[API] <<` | 前端 API 调用成功 | `[API] << create_project (142ms)` |
| `[API] !!` | 前端 API 调用失败 | `[API] !! create_project FAILED: [ERROR_CODE] msg` |
| `[UI]` | 前端 UI 事件 | `[UI] page: Editor` |

---

## 4. 日志级别说明

| 级别 | Rust `log::` | 含义 | 是否记录 API Key |
|---|---|---|---|
| ERROR | `error!()` | 不可恢复错误/操作失败 | ❌ 不记录 |
| WARN | `warn!()` | 可恢复错误/异常状态/降级路径 | ❌ 不记录 |
| INFO | `info!()` | 关键生命周期事件/命令调用/状态转换 | ❌ 不记录 |
| DEBUG | `debug!()` | 详细操作追踪/请求/响应摘要 | ❌ 不记录 |

---

## 5. 前端 console 日志格式

### 5.1 API 调用日志

```
[2026-04-28T12:00:00.000Z] [API] >> create_project {name: "测试小说"}
[2026-04-28T12:00:00.142Z] [API] << create_project (142ms)

[2026-04-28T12:00:01.000Z] [API] >> save_chapter_content {chapterId: "ch_001", content: "[3250 chars]"}
[2026-04-28T12:00:01.050Z] [API] << save_chapter_content (50ms)

[2026-04-28T12:00:02.000Z] [API] >> list_providers
[2026-04-28T12:00:02.030Z] [API] !! list_providers (30ms) FAILED: [DB_OPEN_FAILED] Cannot open app database
```

**安全处理**：
- `apiKey` 字段自动替换为 `[REDACTED]`
- `content` 字段自动替换为 `[${length} chars]`

### 5.2 UI 事件日志

```javascript
// 在组件中调用
import { logUI } from "../../api/tauriClient.js";

logUI("page: Editor", { chapterId: "ch_001" });
// 输出: [2026-04-28T12:00:00.000Z] [UI] page: Editor | chapterId: ch_001
```

---

## 6. Rust 端日志函数

`src-tauri/src/infra/logger.rs` 提供了以下函数：

```rust
// 命令调用日志
log_command("create_project", "name=测试");

// 用户操作日志
log_user_action("save_chapter", "chapter=ch_001, words=3200");

// AI 调用日志（自动脱敏）
log_ai_call("minimax", "MiniMax-M2.7", "chapter.draft", Some(4000));

// 安全事件日志
log_security("save_provider", "provider=MiniMax");

// 服务操作日志
log_service("chapter_service", "create_chapter", "title=第一章");

// 文件操作日志
log_fs("write", "manuscript/chapters/ch-0001.md", "3250 bytes");

// 数据库操作日志
log_db("INSERT", "chapters", "id=ch_001");
```

---

## 7. 如何查看日志

### 开发模式

```bash
pnpm tauri:dev
# Rust 日志 → 终端 stdout
# 前端日志 → 浏览器 F12 → Console
```

### 生产构建

```bash
pnpm tauri build
# 日志 → ~/.novelforge/logs/ (由 tauri-plugin-log 管理)
# 可通过命令行参数 --verbose 获取更多输出
```

### 实时过滤建议

在浏览器 Console 中：
```
// 只看 API 调用
[API]

// 只看 AI 调用
[AI]

// 只看错误
[API] !!
```

终端中：
```bash
pnpm tauri:dev 2>&1 | grep "\[AI\]"    # 仅 AI 日志
pnpm tauri:dev 2>&1 | grep "\[SEC\]"   # 仅安全日志
pnpm tauri:dev 2>&1 | grep "\[ERROR\]" # 仅错误
```

---

## 8. 已添加日志的关键命令

| 命令 | 日志内容 |
|---|---|
| `create_project` | `[USER] create_project | /path/to/project` |
| `open_project` | `[USER] open_project | /path/to/project` |
| `create_chapter` | `[USER] create_chapter | chapter=第一章` |
| `save_chapter_content` | `[USER] save_chapter | chapter=ch_001, words=3200` |
| `export_chapter` | `[USER] export_chapter | format=txt, path=...` |
| `generate_ai_preview` | `[AI] preview / default | chapter.draft` |
| `stream_ai_chapter_task` | `[AI] streaming / default | chapter.draft` |
| `generate_blueprint_suggestion` | `[AI] blueprint / default | blueprint.step_01` |
| `ai_generate_character` | `[AI] character / default | character.create` |
| `ai_scan_consistency` | `[AI] consistency / default | consistency.scan` |
| `save_provider` | `[SEC] save_provider | provider=MiniMax` |
| `delete_provider` | `[SEC] delete_provider | provider_id=xxx` |
| `scan_chapter_consistency` | `[USER] consistency_scan | chapter=ch_001` |
| 所有 `invokeCommand()` | `[API] >> command_name` + `[API] << command_name (time)` |
