# NovelForge 文档入口

本文件是 `docs/` 的统一入口，负责维护文档索引、更新责任和更新节奏。

## 1. 核心维护文档（5 份）
1. 本入口文档：`docs/README.md`
2. 架构文档：`docs/architecture/windows-desktop-architecture.md`
3. UI 设计文档：`docs/ui/ui-design-spec.md`
4. 运行流程文档：`docs/runtime/runtime-process-spec.md`
5. API 集成文档：`docs/api/api-integration-spec.md`

## 2. 参考基线文档（上游约束）
- 产品与开发总蓝图：`docs/novel_workbench_windows_lifecycle_blueprint_v_1.md`
- LLM 供应商接入规范：`docs/novelforge_llm_provider_integration_spec_v_1.md`
- 执行规范：`../AGENTS.md`

## 3. 更新责任
### 3.1 责任分工
- 架构文档：后端/Tauri 实现负责人维护。
- UI 设计文档：前端实现负责人维护。
- 运行流程文档：负责主流程串联的开发负责人维护。
- API 集成文档：前后端接口变更提交人维护。
- README 入口文档：本次变更提交人同步维护。

### 3.2 变更即更新原则
- 代码行为变了，相关文档必须同次提交更新。
- 若未更新文档，PR/提交说明必须写明原因与补齐时间。

## 4. 更新触发条件
- 新增/删除 Tauri command。
- DTO、错误码或返回结构变化。
- 页面结构或关键交互（保存/恢复/错误提示）变化。
- 本地目录协议、存储协议、导出协议变化。
- AI 接入策略、模型配置字段变化。

## 5. 更新节奏
- 日常节奏：每次功能变更同提交同步文档。
- 周期节奏：每周至少做 1 次文档一致性巡检（代码 vs 文档）。
- 里程碑节奏：每个 Sprint 结束前完成一次文档验收清单。

## 6. 文档验收清单（最小）
1. 链接路径可打开，无失效引用。
2. 文档中的接口名、字段名、错误码与当前代码一致。
3. 文档描述的是“已实现行为”，不是计划性描述。
4. 非 MVP 功能未混入 MVP 文档正文。

## 7. 版本记录
- 2026-04-27（S17）：同步发布能力落地结果，更新语义检索（向量索引）、Git 快照、授权激活与自动更新链路说明。
- 2026-04-27（S16）：同步 Beta 功能第二组落地结果，更新 DOCX/PDF/EPUB 导出、时间线页面、关系图页面、编辑器资产抽取候选说明。
- 2026-04-27（S14）：同步 LLM Provider Beta 补全结果，更新任务路由 CRUD、自定义 Provider 字段校验、Provider 真实探活、registry 安全校验闸描述。
- 2026-04-27（S13）：同步主链路收口结果，更新 API/Runtime/UI/Architecture 对 `projectRoot` 强制透传、`get_chapter_context` 与 `delete_chapter` 命令说明。
- 2026-04-27：创建统一入口，建立 5 份核心维护文档索引、责任和更新节奏。
