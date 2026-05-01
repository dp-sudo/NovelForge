---
id: chapter.rewrite
name: 改写选区
description: 在保留事实信息的前提下按用户意图改写选区，提升表达质量与叙事适配度
version: 3
source: builtin
category: writing
tags: [写作, 改写, 编辑]
inputSchema:
  type: object
  properties:
    selectedText: { type: string }
    userInstruction: { type: string }
    projectContext: { type: string }
  required: [selectedText]
outputSchema:
  type: object
  properties:
    rewritten: { type: string }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "🔄"
createdAt: 2026-04-28
updatedAt: 2026-04-29
skillClass: workflow
bundleIds: [bundle.character-expression, bundle.emotion-progression]
alwaysOn: false
triggerConditions: [chapter.rewrite]
requiredContexts: [chapter, canon, state]
stateWrites: [chapter.revision]
automationTier: supervised
sceneTags: [dialogue, emotion]
affectsLayers: [canon, state, recent_continuity]
---

# 改写选区

## 目标

在不改剧情事实的前提下，让文本风格、节奏或语气更贴合当前章节需求。

## 改写约束

1. 不改人名、关系、时间、地点、事件顺序。
2. 不新增设定，不删关键信息。
3. 不把“改写”变成“重写剧情”。

## 执行流程

1. 识别用户改写目标（风格/节奏/对话/压缩/细节）。
2. 提取事实锚点并锁定。
3. 在锚点不变前提下改写表达层。
4. 逐条回检事实锚点是否全保留。

## 常见错误与修正

1. 错误：为了自然把事实删掉。
修正：先列锚点再写。
2. 错误：加入未出现设定。
修正：新增信息必须来自原句隐含含义，不能凭空扩展。
3. 错误：语气改了但人物声线跑偏。
修正：保留角色口头偏好和句法习惯。

<!-- PROMPT_TEMPLATE_START -->
你是一名小说文本改写编辑。请按用户要求改写选区，但必须保持事实不变。

[项目上下文]
{projectContext}

[选中文本]
{selectedText}

[用户要求]
{userInstruction}

执行要求：
1. 先锁定事实锚点：人名、关系、时间、地点、事件顺序。
2. 仅改表达层（语气、节奏、措辞、细节密度），不得改剧情事实。
3. 如用户要求与事实保护冲突，以事实保护优先，并通过表达策略折中。
4. 输出文本应自然、可直接替换原段。

输出格式：仅输出改写后的正文，不要前言、不要解释、不要列清单。
<!-- PROMPT_TEMPLATE_END -->
