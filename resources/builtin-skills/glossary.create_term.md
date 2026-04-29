---
id: glossary.create_term
name: 创建术语条目
description: 根据用户描述生成可落库的术语条目，统一术语定义、别名与使用边界
version: 1
source: builtin
category: glossary
tags: [术语, 词条, 一致性]
inputSchema:
  type: object
  properties:
    userDescription: { type: string }
  required: [userDescription]
outputSchema:
  type: object
  properties:
    term: { type: object }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "📘"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 创建术语条目

用于为名词库生成结构化词条。

<!-- PROMPT_TEMPLATE_START -->
你是一名术语规范编辑。
请根据用户设想生成术语条目：

[项目上下文]
{projectContext}

[用户设想]
{userDescription}

输出要求：
1. 包含术语定义、类别、别名、使用边界。
2. 若与既有设定冲突，给出冲突说明。
3. 输出 JSON 对象，不要额外说明。
<!-- PROMPT_TEMPLATE_END -->
