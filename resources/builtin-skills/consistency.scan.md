---
id: consistency.scan
name: 一致性扫描
description: 扫描章节与项目设定冲突，定位角色、规则、时间线和叙事层面的高风险问题
version: 3
source: builtin
category: review
tags: [审稿, 检查, 一致性, 质量]
inputSchema:
  type: object
  properties:
    chapterId: { type: string }
    chapterContent: { type: string }
    scope: { type: string }
    projectContext: { type: string }
    chapterContext: { type: string }
  required: [chapterId]
outputSchema:
  type: object
  properties:
    issues: { type: array }
requiresUserConfirmation: false
writesToProject: true
author: NovelForge
icon: "🔍"
createdAt: 2026-04-28
updatedAt: 2026-04-29
skillClass: review
bundleIds: [bundle.rule-fulfillment]
alwaysOn: false
triggerConditions: [consistency.scan]
requiredContexts: [chapter, canon, state]
stateWrites: []
automationTier: confirm
sceneTags: []
affectsLayers: [constitution, canon, state, promise, window_plan, recent_continuity]
---

# 一致性扫描

## 目标

把“读起来怪”变成可定位、可修复、可优先级排序的问题清单。

## 扫描维度

1. 角色一致性：外貌、能力、关系、知识边界。
2. 世界规则一致性：术语、规则边界、地理历史。
3. 剧情逻辑一致性：时间顺序、因果链、信息对称。
4. 叙事一致性：视角、人称、时态稳定性。

## 分级标准

1. error：明确冲突，必须修。
2. warning：高度可疑，需确认。
3. info：轻度风险，建议关注。

## 报告原则

1. 每条问题必须有冲突证据。
2. 每条问题必须给最小修复建议。
3. 优先识别会放大后续生成风险的问题。

<!-- PROMPT_TEMPLATE_START -->
你是一名长篇小说一致性审校员。请扫描章节内容与项目设定冲突并输出结构化报告。

[项目上下文]
{projectContext}

[章节上下文]
{chapterContext}

[章节内容]
{chapterContent}

执行要求：
1. 从角色、世界规则、剧情逻辑、叙事四个维度检查冲突。
2. 每条问题必须包含：severity、dimension、location、description、conflictEvidence、suggestion。
3. 严格按error/warning/info分级。
4. 先输出会影响后续AI生成链路的高风险问题。

输出格式：仅输出 JSON 对象，字段必须包含：
- scanSummary
- issues

禁止输出解释性前言、禁止Markdown代码块。
<!-- PROMPT_TEMPLATE_END -->
