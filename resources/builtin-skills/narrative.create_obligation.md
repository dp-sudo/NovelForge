---
id: narrative.create_obligation
name: 创建叙事义务
description: 根据用户描述生成叙事义务条目，标注埋点、回收与风险
version: 1
source: builtin
category: narrative
tags: [叙事, 伏笔, 回收]
inputSchema:
  type: object
  properties:
    userDescription: { type: string }
  required: [userDescription]
outputSchema:
  type: object
  properties:
    obligation: { type: object }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "🧵"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 创建叙事义务

用于生成可追踪的叙事义务条目。

<!-- PROMPT_TEMPLATE_START -->
你是一名叙事结构编辑。
请根据用户设想生成叙事义务条目：

[项目上下文]
{projectContext}

[用户设想]
{userDescription}

输出要求：
1. 说明义务类型、埋点位置、预期回收位置。
2. 标注风险等级与未回收后果。
3. 输出 JSON 对象，不要额外说明。
<!-- PROMPT_TEMPLATE_END -->
