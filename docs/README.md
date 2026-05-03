# NovelForge 文档中心

**NovelForge** 是一款基于 Tauri 2 + React 19 的 Windows 桌面小说创作辅助工具。本文档中心提供完整的技术文档、架构设计、API 规范和开发指南。

## 📚 文档导航

### 核心文档

1. **[项目总览 (README.md)](../README.md)**
   - 项目概览与快速开始
   - 核心功能介绍
   - 项目结构说明
   - 环境要求与安装指南
   - 开发与构建流程

2. **[架构文档 (windows-desktop-architecture.md)](architecture/windows-desktop-architecture.md)**
   - 技术栈与分层设计
   - 前端架构（React + Zustand + TanStack Query）
   - 后端架构（Tauri + Rust + SQLite）
   - AI Pipeline 流程
   - 数据库设计（应用级 + 项目级）
   - 文件系统布局
   - 服务层设计（20+ 服务）
   - 命令面摘要（100+ 命令）

3. **[UI 设计文档 (ui-design-spec.md)](ui/ui-design-spec.md)**
   - 页面信息架构（15 个页面）
   - 页面功能与交互详解
   - 组件规范
   - 数据源映射
   - 关键交互规范
   - UI 风险点

4. **[运行时流程文档 (runtime-process-spec.md)](runtime/runtime-process-spec.md)**
   - 运行时角色（Main / Renderer / API / Service）
   - 启动流程
   - 主流程时序（项目创建/打开、章节写作、AI 任务、上下文抽取、Provider/模型/路由管理）
   - 失败处理策略
   - 错误码与恢复建议

5. **[API 集成文档 (api-integration-spec.md)](api/api-integration-spec.md)**
   - 集成原则
   - 前端 API 入口（`src/api/*`）
   - Command 契约（按模块分类）
   - Pipeline 事件协议（`ai:pipeline:event`）
   - 标准错误结构
   - 最小回归链路
   - Compatibility 命令收敛计划

## 🎯 文档使用指南

### 新手入门
1. 先阅读 [项目总览](../README.md) 了解项目基本情况
2. 查看 [架构文档](architecture/windows-desktop-architecture.md) 理解技术架构
3. 参考 [UI 设计文档](ui/ui-design-spec.md) 了解页面功能
4. 根据需要查阅 [API 集成文档](api/api-integration-spec.md)

### 开发人员
- **前端开发**：重点阅读 UI 设计文档 + API 集成文档
- **后端开发**：重点阅读架构文档 + 运行时流程文档
- **全栈开发**：按顺序阅读所有核心文档

### 维护人员
- 代码变更时必须同步更新相关文档
- 遵循 [AGENTS.md](../AGENTS.md) 中的文档维护规则
- 确保文档描述的是已实现行为，非计划性描述

## 📖 专题文档

### AI 相关
- [AI Pipeline 架构](architecture/windows-desktop-architecture.md#6-ai-架构当前)
- [AI 任务流程](runtime/runtime-process-spec.md#43-编辑器-aipipeline-主链路)
- [AI 命令与事件](api/api-integration-spec.md#47-ai--pipeline--context)

### 数据库相关
- [数据库设计](architecture/windows-desktop-architecture.md#5-数据与存储协议)
- [迁移管理](architecture/windows-desktop-architecture.md#52-项目级数据库databaseprojectsqlite)

### 编辑器相关
- [编辑器功能](ui/ui-design-spec.md#42-章节页与编辑器chapterspageeditorpage)
- [保存与草稿流程](runtime/runtime-process-spec.md#42-章节写作与保存)
- [上下文系统](runtime/runtime-process-spec.md#45-上下文抽取与人工确认入库)

## 🔄 文档更新原则

### 变更即更新
代码行为变化时，必须同步更新相关文档：
- 命令新增/删除 → 更新架构文档 + API 集成文档
- 页面新增/删除 → 更新 UI 设计文档
- 流程变化 → 更新运行时流程文档
- 数据库 schema 变化 → 更新架构文档

### 责任分工
- **架构文档**：后端/Tauri 实现负责人
- **UI 设计文档**：前端实现负责人
- **运行时流程文档**：主流程串联开发负责人
- **API 集成文档**：前后端接口变更提交人

### 验收清单
- ✅ 链接路径可打开
- ✅ 命令名、字段名、错误码与代码一致
- ✅ 描述的是已实现行为，非计划性描述
- ✅ 事件协议与 UI 消费逻辑一致
- ✅ 代码示例可运行
- ✅ 版本号与更新日期正确

## 📝 文档版本

| 文档 | 版本 | 状态 | 最后更新 |
|------|------|------|----------|
| 项目总览 | v1.0 | 生产就绪 | 2026-05-03 |
| 架构文档 | v0.7 | 生产就绪 | 2026-05-03 |
| UI 设计文档 | v0.7 | 生产就绪 | 2026-05-03 |
| 运行时流程文档 | v0.8 | 生产就绪 | 2026-05-03 |
| API 集成文档 | v0.8 | 生产就绪 | 2026-05-03 |

## 🔗 相关资源

- **开发规范**：[AGENTS.md](../AGENTS.md) - AI 编码规范与最佳实践
- **Claude 指南**：[CLAUDE.md](../CLAUDE.md) - Claude 特定开发指南
- **项目主页**：https://novelforge.app
- **问题反馈**：https://github.com/novelforge/desktop/issues

## 📧 联系方式

- **邮箱**：support@novelforge.app
- **技术支持**：tech@novelforge.app

---

**NovelForge** - 让创作更高效，让故事更精彩。

*最后更新：2026-05-03*
