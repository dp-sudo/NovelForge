# NovelForge 全链路深度审计报告
**日期**: 2026-05-02  
**审计范围**: 静态 + 动态分析, 代码质量、数据流、架构、安全性  
**代码基线**: `F:\NovelForge` @ 2026-05-02 01:54 GMT+8

---

## 一、项目总览

| 维度 | 度量 |
|------|------|
| 技术栈 | Tauri 2.x (Rust) + React 19 + TypeScript + Zustand + Tailwind CSS 4 |
| Rust 源码文件 | 44 个 (不含 target 生成代码) |
| TypeScript 源文件 | 65 个 (~45 pages/components + 20 API/stores/hooks) |
| Tauri 命令 | 84 个注册命令 |
| 后端服务 | 22 个 Service 结构体 |
| 测试文件 | 8 个集成测试 + 大量 Rust 内联测试 |
| 数据库 | SQLite (项目级 + 应用级双数据库) |
| 文档体系 | 5 份核心维护文档 + 版本记录 |

---

## 二、架构设计评估

### 2.1 分层架构 ✅ 良好

```
Frontend (React/TS)         → 仅通过 Tauri command 调用
        ↓
Commands Layer (84 cmds)    → 入参校验 + 路由到 Service
        ↓
Services Layer (22 svcs)    → 纯业务逻辑, 无状态(除 AiPipelineService)
        ↓
Infrastructure Layer        → DB, FS, Crypto, Credential, Path Utils
```

**评价**: 清晰的三层分离。Commands 做薄层参数校验，Service 做业务，Infra 做底层能力。符合单一职责原则。

### 2.2 状态管理 ✅ 良好

- **前端**: Zustand store (editorStore, projectStore, skillStore, uiStore) — 轻量、类型安全
- **后端**: `AppState` 聚合全部 Service，通过 Tauri 的 `.manage()` 注入
- **SkillRegistry**: `Arc<RwLock<SkillRegistry>>` — 线程安全，支持动态热加载
- **Pipeline 取消**: `Arc<RwLock<HashSet<String>>>` — 轻量取消信号

**评价**: 状态边界清晰。Rust 端无全局可变状态，通过 Arc 共享只读引用。

### 2.3 数据持久化设计 ✅ 良好

```
项目级 (project.sqlite):   章节、角色、世界规则、术语表、情节节点、快照、状态账本
应用级 (novelforge.db):     供应商配置、模型注册表、任务路由、编辑器设置、晋升策略
文件系统:                   章节正文 (Markdown + YAML frontmatter)、自动保存草稿、快照文件
```

**评价**: 数据库/文件系统职责分离明确。章节正文以 Markdown 落盘，元数据入 SQLite，符合"正文可脱离工具独立编辑"的设计目标。

---

## 三、代码质量评估

### 3.1 Rust 端

| 检查项 | 状态 | 说明 |
|--------|------|------|
| unwrap() 生产代码 | ✅ 0 处 | 全部使用 `?` 或 Result 传播 |
| expect() | ⚠️ 459处 | 测试代码中大量使用是正确的，但需确认生产代码中无乱用 |
| error 传播 | ✅ 统一 | AppErrorDto 结构化错误: code + message + detail + recoverable + suggestedAction |
| 类型安全 | ✅ 强类型 | 严格使用新类型模式, serde rename_all 统一 camelCase |
| 事务使用 | ✅ 正确 | 关键写操作均有事务保护 (create_chapter, delete_chapter, save_provider) |
| SQL 注入 | ✅ 安全 | 全部使用参数化查询 `params![]` |
| 原子写 | ✅ 正确 | `write_file_atomic()` → 临时文件 + rename |

### 3.2 TypeScript 端

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 类型覆盖 | ✅ 全面 | domain/types.ts 涵盖了所有 DTO；API 函数均有完整类型约束 |
| 异步处理 | ✅ 正确 | async/await + try/catch 规范 |
| React Hooks | ✅ 规范 | 依赖数组完整，useCallback/useMemo 合理使用 |
| 状态管理 | ✅ 清晰 | Zustand selector 模式，避免不必要的重渲染 |

---

## 四、安全性深度分析

### 4.1 API Key 管理 ✅ 优秀

```
优先级链:
  Windows Credential Manager (主力)
  → 失败回退到 AES-256-GCM 加密本地文件
  → 加密密钥派生自: 机器名 + 静态盐 + 版本号 (SHA-256)
  → sanitize_provider_id() 防路径穿越
  → mask_api_key() 返回时遮蔽 (前端永远看不到明文)
```

**评价**: 业界最佳实践。Keyring + 加密文件双保险。API Key 不出现在日志、项目目录、前端明文。

**⚠️ 风险点**:
- `derive_key()` 仅依赖 COMPUTERNAME/HOSTNAME 环境变量。如果鼠标环境被克隆或机器名已知，加密强度降低。建议加入 Windows DPAPI (`CryptProtectData`) 作为额外保护层。

### 4.2 路径安全 ✅ 良好

```rust
// path_utils.rs: 反路径穿越保护
resolve_project_relative_path() → 必须落在 project_root 下
→ to_posix_relative() 只存储相对路径
→ resolve_project_scoped_path() 解析时做边界检查
```

`chapter_service.rs` 中 `resolve_project_scoped_path()` 有明确的路径越界拦截。

### 4.3 URL 校验 ✅ 良好

```rust
// settings_service.rs: validate_provider_config()
→ 非 https 且非 localhost/loopback 的 URL 直接拒绝
→ base_url 自动 trim 尾部斜杠
```

### 4.4 潜在安全隐患

| 问题 | 风险等级 | 建议 |
|------|---------|------|
| `derive_key()` 仅用主机名 | 🟡 Medium | 加入 DPAPI 或 machine GUID 混合派生 |
| `credential_manager.rs` 中 keyring 失败静默降级 | 🟢 Low | 已有 log::warn，合理 |
| `component.json` 引入 Tailwind/Radix UI 供应链 | 🟢 Low | npm audit 常规维护即可 |
| 前端 `navigator.clipboard.writeText()` | 🟢 Low | 用户主动操作，可接受 |

---

## 五、AI Pipeline 全链路分析

### 5.1 流程架构 ✅ 优秀

```
Phase: validate → context → route → prompt → generate → postprocess → persist → done
         │          │        │        │         │           │           │
         │     continuity   skill   prompt    stream     normalize   persist
         │       pack      select   resolve    delta       code       task
         │   + freeze_guard  +                                 block     output
         │   + certainty     route_override                               + state
         │     zones                                                      writes
```

**特色功能**:
- **确定区 (Certainty Zones)**: 冻结区/承诺区/探索区 — 约束 AI 不违反已确定的设定
- **冻结保护 (Freeze Guard)**: 检测用户指令是否试图修改已冻结区域
- **连续性包 (Continuity Pack)**: 根据任务类型自动组装上下文层
- **场景分类 (Scene Classifier)**: 自动推断场景类型，匹配 skill bundle
- **后任务执行 (Post Tasks)**: pipeline 完成后可触发后续分析任务

### 5.2 事件协议 ✅ 完善

`ai:pipeline:event` 事件流 — 前端通过 `usePipelineStream` hook 消费，支持:
- `start` / `delta` / `progress` / `warning` / `error` / `done`
- `meta` 字段携带路由决策、skill 选择、persist 结果等结构化数据
- 取消信号通过 `PIPELINE_CANCELLED` 错误码传递

### 5.3 任务路由系统 ✅ 健壮

- 16 种核心任务类型 + `custom` 占位
- `canonicalTaskType()` 兼容 40+ 种遗留别名
- 默认路由自动补齐 (`ensure_default_task_routes_initialized`)
- route_override 机制允许 skill 动态改变路由

### 5.4 ⚠️ 风险点

| 问题 | 风险等级 | 详情 |
|------|---------|------|
| Pipeline 取消非即时 | 🟡 Medium | cancel 设置 HashSet 标记，需在下一次 `check_cancelled()` 调用时才生效。长文本生成期间取消延迟可达秒级。 |
| 自动保存与 pipeline 并发 | 🟡 Medium | `autosaveDraft()` 与 pipeline 的 `persist_task_output()` 可能同时写 `manuscript/drafts/` 目录。虽用原子写，但需确认无 TOCTOU 问题。 |
| 上下文缺失降级不阻断 | 🟢 Low | continuity pack 不完整时发出 `warning` 事件但不中断，符合设计意图。 |
| `scene_classifier` 为简单规则匹配 | 🟢 Low | 分类逻辑基于关键词匹配 ("对话"/"动作"/"战斗")，对复杂场景可能误判。 |

---

## 六、数据流与一致性分析

### 6.1 章节生命周期 ✅ 完整

```
createChapter → 创建 DB record + Markdown 文件 (原子 rollback)
saveChapterContent → 原子写文件 + 更新 DB (wordCount + version++)
autosaveDraft → 写入 drafts/{chapter}.autosave.md (独立于正式文件)
deleteChapter → 软删除 + 索引重排 + chapter_links 清理
recoverDraft → 比较文件修改时间戳判定草稿是否更新
```

### 6.2 事务完整性 ✅

- `create_chapter`: 文件先写，DB 插入失败会自动删除文件
- `delete_chapter`: 事务保护 (mark deleted + reindex + clean links)
- `save_provider`: DB + Secret 双写，失败自动回滚 secret
- `reorder_chapters`: 两阶段负索引策略避免 UNIQUE 冲突

### 6.3 ⚠️ 边界条件问题

| 问题 | 严重度 | 详情 |
|------|--------|------|
| `contentWordCount()` 计数字符 | 🟡 Info | 用 `chars().filter(!is_whitespace).count()` — 中文"字符"≠英文"word"。标题和命名将此值标为"wordCount"有误导性。 |
| 恢复草稿的 mtime 竞态 | 🟡 Low | Windows FAT/NTFS 时间戳精度有限，短间隔写草稿可能误判 mtime 相等。建议加入版本号或 checksum。 |
| Autosave 无节流保护 | 🟢 Info | `AUTOSAVE_DELAY_MS = 5000` 固定间隔，打字速度极快时每次键盘事件重置 timer，合理。 |

---

## 七、性能分析

### 7.1 设计上的性能考量 ✅

- **流式生成**: pipeline 通过 `stream_generate` + `tokio::sync::mpsc` 传输增量内容
- **Vite 分包**: react-vendor / tauri-vendor / radix-vendor / state-vendor 分离
- **向量检索**: 独立索引，支持重建，语义搜索异步非阻塞
- **Autosave 防抖**: 5 秒延迟，避免频繁 I/O
- **SQLite 连接**: 每次操作 openDatabase → 使用后隐式关闭 (非长连接)

### 7.2 ⚠️ 潜在性能瓶颈

| 问题 | 影响 | 建议 |
|------|------|------|
| 每次 DB 操作都 open/close | 🟡 Medium | SQLite 连接池或长连接可减少打开开销。当前每个 command 调用都重新 open_database。 |
| 向量索引全量重建 | 🟡 Medium | `rebuild_vector_index` 遍历所有章节重建，大数据量时耗时。考虑增量更新。 |
| 上下文采集无缓存 | 🟢 Low | `collect_chapter_context` 每次都完整查询所有关联表。热点章节可考虑短期缓存。 |
| pipeline 中 context/route/prompt 阶段无并行 | 🟢 Low | 当前串行合理，context 是 route 的前置依赖。 |

---

## 八、错误处理与可恢复性评估

### 8.1 错误分类体系 ✅ 优良

```
AppErrorDto {
    code: "DB_OPEN_FAILED" | "CHAPTER_NOT_FOUND" | "PIPELINE_CANCELLED" | ...
    message: 用户可读中文消息
    detail: 技术与调试信息
    recoverable: true/false → 前端决定是否展示重试按钮
    suggestedAction: 修复建议
}
```

### 8.2 关键场景覆盖

| 场景 | 处理 | 评级 |
|------|------|------|
| 数据库打开失败 | ✅ 明确错误 + 建议 | 良好 |
| 章节文件丢失 | ✅ 返回 NotFound 错误 | 良好 |
| AI 返回空内容 | ✅ `PIPELINE_EMPTY_OUTPUT` 错误 | 良好 |
| 网络超时 | ✅ 可重试 (recoverable: true) | 良好 |
| Pipeline 取消 | ✅ `PIPELINE_CANCELLED` + 审计记录 | 良好 |
| 草稿恢复 | ✅ 自动检测 + 用户确认 | 良好 |

### 8.3 未覆盖场景

| 问题 | 风险 | 建议 |
|------|------|------|
| 磁盘满 | 🟡 Medium | `write_file_atomic` 未特殊处理 `ENOSPC`，直接返回 io::Error |
| SQLite 数据库损坏 | 🟡 Medium | 未集成 `PRAGMA integrity_check` 定期校验 |
| 项目文件手动外部修改 | 🟢 Low | 无文件监控/Watcher 机制，但用户显式重新打开即可 |

---

## 九、测试覆盖分析

| 层级 | 测试类型 | 文件数 | 覆盖 |
|------|---------|--------|------|
| Rust unit | `#[cfg(test)]` | 散布于各 .rs 文件 | crypto, credential, database, reorder, timeline, chapter CRUD, settings, task_routing, orchestrator, skill_registry |
| TypeScript unit | `tests/*.test.ts` | ~8 个 | chapter-autosave, dev-engine-settings, model_pool_routing, 等 |
| Integration | `tests/integration/` | 若干 | 关键链路 |

**评价**: 测试覆盖了核心数据链路。Rust 端内联测试质量高（使用临时工作区 + 真实 SQLite）。TS 端测试偏少，前端组件级测试缺失。

---

## 十、文档一致性检查

**检验结果**: 
- ✅ 5 份核心维护文档均存在且结构完整
- ✅ README.md 版本记录与代码变更一致
- ✅ `lib.rs` 中注册的 84 个命令与文档描述匹配
- ✅ `state.rs` 中的 22 个 Service 与架构文档一致
- ⚠️ 文档描述的 "AI 生成预览与人工确认插入" 与当前 `AiPreviewPanel` 组件实现一致

---

## 十一、综合评分

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计 | ⭐⭐⭐⭐⭐ | 三层分离、状态管理清晰、数据持久化设计合理 |
| 代码质量 | ⭐⭐⭐⭐☆ | Rust 端强类型安全、TS 端类型覆盖完整。少量 expect 需审计 |
| 安全性 | ⭐⭐⭐⭐☆ | API Key 双保险、路径穿越防护、URL 校验。加密密钥可进一步增强 |
| AI Pipeline | ⭐⭐⭐⭐⭐ | 流式 + 取消 + 审计 + 上下文组装 + 冻区保护, 工程化程度高 |
| 错误处理 | ⭐⭐⭐⭐☆ | 结构化错误体系, recoverable 标记合理。缺失: 磁盘满/DB 损坏场景 |
| 性能 | ⭐⭐⭐⭐☆ | 流式生成、防抖 autosave。瓶颈: DB 频繁 open/close、向量全量重建 |
| 测试覆盖 | ⭐⭐⭐⭐☆ | Rust 端内联测试充分。TS 端前端组件测试有待补充 |
| 可维护性 | ⭐⭐⭐⭐☆ | 文档同步机制完善、AGENTS.md 约束清晰。代码注释适度 |

**总体评估: 高质量 MVP 项目**。架构清晰、安全实践扎实、AI Pipeline 工程化程度高。少量中低风险点可逐步收敛，不影响 MVP 交付。

---

## 十二、优先级改进建议

### 🔴 高优先级
- [无] 当前未发现阻塞性问题

### 🟡 中优先级
1. **DB 连接管理**: 引入连接池或缓存连接，避免每次操作 open/close
2. **Pipeline 取消延迟**: 考虑在 stream_generate 层增加更多 check_cancelled 调用点

### 🟢 低优先级
3. **加密密钥增强**: 混入 DPAPI 或 machine GUID
4. **向量索引增量更新**: 替代全量重建
5. **前组件测试**: 补充 EditorPage / AiPreviewPanel 关键交互测试
6. **SQLite integrity_check 定时任务**: 应用启动时或定时运行
7. **wordCount 语义调整**: 中文环境考虑改为"字符数"或使用更适合中文的计数方式
