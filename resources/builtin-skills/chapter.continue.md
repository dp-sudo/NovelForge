---
id: chapter.continue
name: 续写章节
description: 根据前文在同一叙事状态下续写，保证视角、节奏、信息与风格连续
version: 3
source: builtin
category: writing
tags: [写作, 续写, 连贯]
inputSchema:
  type: object
  properties:
    chapterId: { type: string }
    precedingText: { type: string }
    userInstruction: { type: string }
    projectContext: { type: string }
    chapterContext: { type: string }
  required: [chapterId, precedingText]
outputSchema:
  type: object
  properties:
    continuation: { type: string }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "📝"
createdAt: 2026-04-28
updatedAt: 2026-04-29
skillClass: workflow
bundleIds: [bundle.character-expression, bundle.emotion-progression, bundle.scene-environment]
alwaysOn: false
triggerConditions: [chapter.continue]
requiredContexts: [chapter, canon, state]
stateWrites: [chapter.progress, character.emotion, scene.environment, relationship.temperature]
automationTier: supervised
sceneTags: []
affectsLayers: [canon, state, promise, window_plan, recent_continuity]
---

# 续写章节

## 目标

保证“无拼接感”：续写开头像同一作者在同一时刻继续写下去。

## 续写前前文特征分析流程

按顺序执行：

1. 叙事状态识别：当前是动作推进、对话拉扯、情绪回落还是信息揭示。
2. 视角锁定：确认前文POV，禁止无触发切视角。
3. 节奏抽样：取前文末尾300-500字，判断长短句比例与段落长度。
4. 信息边界：列出“已知事实”和“尚未揭示信息”，防止重复和越界。
5. 情绪温度：标记当前情绪强度(1-5)，续写第一段只能上下浮动1级。

## 续写衔接技巧库

1. 动作链衔接：上一动作未闭合时，先补下一动作。
2. 对话链衔接：上一句是问句时，优先在两句内回应。
3. 情绪链衔接：先延续原情绪，再转调，不要硬切。
4. 信息链衔接：每段只新增一个关键信息点。

## 常见错误与修正

1. 错误：续写开头重复前文信息。
修正：禁止复述前文最后200字中的核心事件。
2. 错误：突然解释背景，导致节奏断裂。
修正：背景信息拆成动作中渗透，不做整段说明。
3. 错误：人物语气漂移。
修正：保留角色口头禅/句长/用词级别。
4. 错误：过渡词机械（“然后/接着/与此同时”泛滥）。
修正：用动作或感官细节实现自然过渡。

## 续写质量自检清单（输出前必须通过）

1. 首段是否紧接前文最后状态。
2. 是否新增了前文不存在的新设定（若有，是否有合理触发）。
3. 是否保持POV一致。
4. 是否避免信息重复。
5. 是否至少推进一个“事件/关系/冲突”。
6. 是否保留前文语体与节奏。

## 前后对比示例

Before：
前文："门锁咔哒一声。" 续写："第二天早晨，主角开始回忆自己的童年。"（时空硬跳）

After：
前文："门锁咔哒一声。" 续写："门缝先挤进一线冷风，随后是一只戴着黑色手套的手。"（动作连续）

<!-- PROMPT_TEMPLATE_START -->
你是一名长篇小说续写编辑。请在保持叙事连续性的前提下续写正文。

[项目上下文]
{projectContext}

[章节上下文]
{chapterContext}

[前文]
{precedingText}

[用户指令]
{userInstruction}

执行要求：
1. 先完成“前文特征分析”：叙事状态、POV、节奏、信息边界、情绪温度。
2. 续写第一段必须直接承接前文最后状态，不得时空硬跳。
3. 禁止重复前文核心信息；禁止引入无触发的新设定。
4. 至少推进一个有效变化：事件推进、关系变化或冲突升级。
5. 输出前按自检清单自查，未通过则重写。

输出格式：仅输出续写正文，不要解释过程，不要标题，不要额外注释。
<!-- PROMPT_TEMPLATE_END -->
