---
id: extractor.character.appearance
name: 角色外观状态抽取
description: 从章节文本抽取角色着装、伤痕、伪装等外观变化
version: 1
source: builtin
category: utility
tags: [抽取, 状态账本, 外观]
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
icon: "🧥"
createdAt: 2026-05-02
updatedAt: 2026-05-02
skillClass: extractor
bundleIds: [bundle.character-expression]
alwaysOn: false
triggerConditions: [extractor.character.appearance, extract_state]
requiredContexts: [chapter, state]
stateWrites: [character.appearance]
automationTier: auto
sceneTags: [dialogue, action, combat]
affectsLayers: [state]
---

# 角色外观状态抽取

抽取角色在本章中出现的外观变化（服装、伤病、伪装、道具挂载）。

<!-- PROMPT_TEMPLATE_START -->
你是小说状态抽取器。请从文本中识别“角色外观状态变化”，并输出 JSON：

{
  "states": [
    {
      "character": "角色名",
      "appearance": "外观变化描述",
      "evidence": "原文证据"
    }
  ]
}

输入：
{content}

仅输出 JSON，不要输出 Markdown 代码块。
<!-- PROMPT_TEMPLATE_END -->
