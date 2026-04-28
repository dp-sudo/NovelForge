---
id: context.collect
name: 收集上下文
description: 内部服务技能 — 在生成或检查前自动收集当前章节相关的项目资产上下文，供 LLM 调用填充模板变量
version: 3
source: builtin
category: utility
tags: [上下文, 内部, 服务]
inputSchema:
  type: object
  properties:
    chapterId: { type: string }
    scope: { type: string, enum: [current_chapter, full] }
  required: [chapterId]
outputSchema:
  type: object
  properties: {}
requiresUserConfirmation: false
writesToProject: false
promptStrategy: replace
author: NovelForge
icon: "📚"
createdAt: 2026-04-28
updatedAt: 2026-04-28
---

# 收集上下文

## 说明

内部服务技能，由 ContextService 自动调用。不会出现在 AI Command Bar 中向用户展示，也不消耗 LLM 调用配额。

## 采集范围

根据当前章节 ID 和 scope 参数，从项目数据库中采集：

| 上下文类型 | 数据来源 | 填充变量 |
|------------|----------|----------|
| 项目元数据 | project.json | {projectContext} |
| 当前章节信息 | chapters 表 | {chapterContext} |
| 本章涉及的角色 | characters 表 | — |
| 世界观规则 | world_rules 表 | — |
| 当前剧情节点 | plot_nodes 表 | — |
| 前 N 章概要 | chapters 表 | — |

## scope 参数行为

- `current_chapter` — 只收集与本章直接相关的上下文（角色、地点、当前剧情弧）
- `full` — 收集所有项目资产的完整列表

## 使用场景

```
生成章节草稿  → collect(chapterId, current_chapter) → 填充 {projectContext} + {chapterContext}
一致性扫描    → collect(chapterId, full)             → 填充 {projectContext} + {chapterContext}
续写          → collect(chapterId, current_chapter)  → 填充 {projectContext}
```
