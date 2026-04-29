---
id: dashboard.review
name: 仪表盘诊断
description: 基于项目总体状态生成阶段诊断和优先级建议
version: 1
source: builtin
category: review
tags: [仪表盘, 诊断, 优先级]
inputSchema:
  type: object
  properties:
    userInstruction: { type: string }
outputSchema:
  type: object
  properties:
    report: { type: string }
requiresUserConfirmation: false
writesToProject: false
author: NovelForge
icon: "📊"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 仪表盘诊断

用于生成创作阶段诊断报告。

<!-- PROMPT_TEMPLATE_START -->
你是一名创作进度诊断顾问。
请根据项目上下文输出阶段诊断与优先级建议：

[项目上下文]
{projectContext}

[附加要求]
{userInstruction}

输出要求：
1. 识别当前最关键的 3 个风险点。
2. 给出按优先级排序的执行建议。
3. 输出结构化诊断报告。
<!-- PROMPT_TEMPLATE_END -->
