---
id: extractor.character.knowledge
name: 角色信息边界抽取
description: 抽取角色当前已知/未知信息边界，避免后续剧情越权知情
version: 1
source: builtin
category: utility
tags: [抽取, 状态账本, 信息边界]
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
icon: "🧠"
createdAt: 2026-05-02
updatedAt: 2026-05-02
skillClass: extractor
bundleIds: [bundle.rule-fulfillment]
alwaysOn: false
triggerConditions: [extractor.character.knowledge, extract_state]
requiredContexts: [chapter, state]
stateWrites: [character.knowledge]
automationTier: auto
sceneTags: [dialogue, exposition, introspection]
affectsLayers: [state]
---

# 角色信息边界抽取

抽取角色在当前章节后“已知/未知”的关键信息边界。

<!-- PROMPT_TEMPLATE_START -->
你是小说状态抽取器。请从文本中识别角色的知情边界，并输出 JSON：

{
  "states": [
    {
      "character": "角色名",
      "knowledge": "已知信息",
      "unknown": "仍未知信息",
      "evidence": "原文证据"
    }
  ]
}

输入：
{content}

仅输出 JSON，不要输出 Markdown 代码块。
<!-- PROMPT_TEMPLATE_END -->
