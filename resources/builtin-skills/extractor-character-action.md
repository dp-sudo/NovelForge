---
id: extractor.character.action
name: 角色动作状态抽取
description: 从章节文本抽取角色当前动作状态并写回 State Ledger
version: 1
source: builtin
category: utility
tags: [抽取, 状态账本, 动作]
inputSchema:
  type: object
  properties:
    content: { type: string }
  required: [content]
outputSchema:
  type: object
  properties:
    states: { type: array }
requiresUserConfirmation: false
writesToProject: true
author: NovelForge
icon: "🏃"
createdAt: 2026-05-02
updatedAt: 2026-05-02
skillClass: extractor
bundleIds: [bundle.character-expression]
alwaysOn: false
triggerConditions: [extractor.character.action, extract_state]
requiredContexts: [chapter, state]
stateWrites: [character.action]
automationTier: auto
sceneTags: [action, combat]
affectsLayers: [state]
---

# 角色动作状态抽取

抽取角色当前动作（如移动中、战斗中、静止观察）并返回结构化结果。

<!-- PROMPT_TEMPLATE_START -->
你是小说状态抽取器。请从文本中识别“角色当前动作状态”，并输出 JSON：

{
  "states": [
    {
      "character": "角色名",
      "action": "战斗中|移动中|静止观察|其他",
      "evidence": "原文证据"
    }
  ]
}

输入：
{content}

仅输出 JSON，不要输出 Markdown 代码块。
<!-- PROMPT_TEMPLATE_END -->
