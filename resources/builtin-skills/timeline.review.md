---
id: timeline.review
name: 时间线审阅
description: 审阅全书时间线，识别事件顺序、跨度与因果链风险
version: 1
source: builtin
category: review
tags: [时间线, 审阅, 风险]
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
icon: "⏱️"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 时间线审阅

用于生成时间线一致性审阅报告。

<!-- PROMPT_TEMPLATE_START -->
你是一名时间线审阅员。
请根据项目上下文生成时间线风险报告：

[项目上下文]
{projectContext}

[附加要求]
{userInstruction}

输出要求：
1. 优先检查事件顺序、时间跨度、因果断裂。
2. 每条问题给出风险级别与修正建议。
3. 输出结构化审阅报告。
<!-- PROMPT_TEMPLATE_END -->
