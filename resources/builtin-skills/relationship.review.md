---
id: relationship.review
name: 关系审阅
description: 审阅角色关系网络，识别关系断层、跳变和动机冲突
version: 1
source: builtin
category: review
tags: [关系, 角色, 审阅]
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
icon: "🕸️"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 关系审阅

用于生成角色关系一致性审阅报告。

<!-- PROMPT_TEMPLATE_START -->
你是一名角色关系审阅员。
请根据项目上下文生成关系一致性报告：

[项目上下文]
{projectContext}

[附加要求]
{userInstruction}

输出要求：
1. 检查关系缺失、关系跳变和冲突动机不足。
2. 给出可执行修复建议。
3. 输出结构化审阅报告。
<!-- PROMPT_TEMPLATE_END -->
