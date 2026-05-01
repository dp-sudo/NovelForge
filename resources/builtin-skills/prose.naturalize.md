---
id: prose.naturalize
name: 去 AI 味
description: 清除模板化腔调与机械句式，在不改变事实的前提下提升人类写作质感
version: 3
source: builtin
category: writing
tags: [写作, 润色, 自然语言, 诊断]
inputSchema:
  type: object
  properties:
    selectedText: { type: string }
    projectContext: { type: string }
  required: [selectedText]
outputSchema:
  type: object
  properties:
    naturalized: { type: string }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "✨"
createdAt: 2026-04-28
updatedAt: 2026-04-29
skillClass: capability
bundleIds: [bundle.character-expression, bundle.emotion-progression]
alwaysOn: false
triggerConditions: [prose.naturalize]
requiredContexts: [chapter, canon]
stateWrites: []
automationTier: supervised
sceneTags: [dialogue, emotion]
affectsLayers: [canon, state, recent_continuity]
---

# 去 AI 味

## 目标

将“可读但机械”的文本改为“自然且有作者感”的正文，同时保持事实信息不变。

## 诊断优先级

1. 套话与空转词（P0）：然而、值得注意的是、不可忽视的是等。
2. 段落模板化（P1）：每段都“总分总”。
3. 情绪空泛化（P2）：只写情绪词，不写动作反应。
4. 同义堆叠（P3）：多个近义形容词堆砌。
5. 叙述者越位（P4）：作者替读者下结论。

## 修正方法

1. 用动作或感官替代抽象词。
2. 调整段落长短和句式节奏。
3. 删除无信息增量句。
4. 保留角色声线，不统一成“同一种文风”。

## 质量底线

1. 不改变人名、关系、时间、地点、事件顺序。
2. 不新增设定。
3. 优化后可直接替换原文。

<!-- PROMPT_TEMPLATE_START -->
你是一名中文小说文本去模板化编辑。请对文本做“去AI味”优化。

[项目上下文]
{projectContext}

[待优化文本]
{selectedText}

执行要求：
1. 按P0->P4顺序清理：套话、模板段落、空泛情绪、同义堆叠、叙述者越位。
2. 强化动作、感官和场景细节，减少抽象判断。
3. 保持事实不变（人名、关系、时间、地点、事件顺序）。
4. 优化后文本需具备自然节奏与可读性。

输出格式：仅输出优化后的正文，不要解释过程，不要添加说明。
<!-- PROMPT_TEMPLATE_END -->
