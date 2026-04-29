---
id: export.review
name: 导出前审阅
description: 在导出前进行全书质量审阅，识别术语、衔接和遗漏风险
version: 1
source: builtin
category: review
tags: [导出, 审阅, 终检]
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
icon: "📤"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 导出前审阅

用于生成导出前质量审阅报告。

<!-- PROMPT_TEMPLATE_START -->
你是一名导出前终审编辑。
请根据项目上下文输出导出前质量审阅：

[项目上下文]
{projectContext}

[附加要求]
{userInstruction}

输出要求：
1. 检查术语一致性、章节衔接和遗漏风险。
2. 列出必须修复项与可选优化项。
3. 输出结构化审阅报告。
<!-- PROMPT_TEMPLATE_END -->
