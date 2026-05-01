---
id: context.collect
name: 收集上下文
description: 内部服务技能，在生成或审阅前收集章节相关上下文，保障Prompt有足够信息密度
version: 3
source: builtin
category: utility
tags: [上下文, 内部, 服务]
inputSchema:
  type: object
  properties:
    chapterId: { type: string }
    scope: { type: string, enum: [current_chapter, full] }
  required: [chapterId]
outputSchema:
  type: object
  properties: {}
requiresUserConfirmation: false
writesToProject: false
author: NovelForge
icon: "📚"
createdAt: 2026-04-28
updatedAt: 2026-04-29
skillClass: extractor
bundleIds: [bundle.scene-environment, bundle.character-expression]
alwaysOn: false
triggerConditions: [context.collect]
requiredContexts: [chapter, canon]
stateWrites: [relationship.relationship, character.involvement, scene.environment]
automationTier: auto
sceneTags: [dialogue, emotion, environment]
affectsLayers: [canon, state, recent_continuity]
---

# 收集上下文

## 说明

内部服务技能，由 ContextService 自动调用。目标是减少“AI凭空补设定”的概率，提高一键生成质量。

## 收集策略

1. current_chapter：优先当前章相关资产，降低噪声。
2. full：全量收集，适用于一致性审阅和终检。

## 必收字段

1. 项目层：题材、叙事视角、写作风格、蓝图摘要。
2. 章节层：章节目标、当前摘要、前章摘要。
3. 资产层：角色、世界规则、剧情节点、关系边。

## 变量覆盖说明（新增）

### 标准输出变量

1. projectContext：项目全局上下文拼接文本。
2. chapterContext：当前章节上下文拼接文本。

### 输入别名映射（由运行时注入）

1. userInstruction -> userDescription（创建类任务别名）
2. selectedText -> precedingText（续写任务回退别名）
3. chapterContent -> content（导入/扫描兼容别名）
4. blueprint_step_title -> stepTitle（蓝图任务别名）
5. blueprint_step_key -> stepKey（蓝图任务别名）

### 任务-变量覆盖矩阵

1. chapter.draft / chapter.plan：projectContext + chapterContext + userInstruction + targetWords。
2. chapter.continue：projectContext + chapterContext + precedingText + userInstruction。
3. consistency.scan：projectContext + chapterContext + chapterContent。
4. character/world/plot/glossary/narrative.create：projectContext + userDescription。
5. review类（timeline/relationship/dashboard/export）：projectContext + userInstruction。

## 质量要求

1. 信息可追溯：每条上下文应有来源表或来源文件。
2. 信息有边界：避免把历史噪声混入当前章生成。
3. 信息可压缩：优先提取“可驱动生成”的关键事实。
4. 变量可落地：必须能映射到任务模板中的占位符。

<!-- PROMPT_TEMPLATE_START -->
你是上下文收集服务。根据输入参数输出本次AI任务所需上下文采集计划。

[章节ID]
{chapterId}

[收集范围]
{scope}

执行要求：
1. 指定要收集的项目层、章节层、资产层数据清单。
2. 对每类数据给出来源与过滤规则。
3. 标注结果将填充到哪些标准变量（projectContext/chapterContext等）。
4. 明确本次任务需要的别名映射与覆盖变量。

输出格式：结构化文本，不要生成小说内容。
<!-- PROMPT_TEMPLATE_END -->
