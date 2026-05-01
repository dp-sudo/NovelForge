---
id: chapter.plan
name: 生成章节计划
description: 结合项目设定和章节上下文生成结构化纲目规划，包含场景拆分、节奏曲线、字数与伏笔安排
version: 4
source: builtin
category: writing
tags: [写作, 规划, 结构]
inputSchema:
  type: object
  properties:
    chapterId: { type: string }
    userInstruction: { type: string }
    projectContext: { type: string }
    chapterContext: { type: string }
  required: [chapterId]
outputSchema:
  type: object
  properties:
    chapterFunction: { type: string }
    successCriteria: { type: string }
    emotionalArc: { type: string }
    scenes:
      type: array
      items:
        type: object
    foreshadowingPlan: { type: string }
    totalWords: { type: integer }
    cliffhanger: { type: string }
    notes: { type: string }
    status: { type: string }
requiresUserConfirmation: false
writesToProject: true
author: NovelForge
icon: "📋"
createdAt: 2026-04-28
updatedAt: 2026-04-30
skillClass: workflow
bundleIds: [bundle.rule-fulfillment, bundle.scene-environment]
alwaysOn: false
triggerConditions: [chapter.plan]
requiredContexts: [chapter, canon, state]
stateWrites: [chapter.plan_status, window.progress]
automationTier: supervised
sceneTags: []
affectsLayers: [constitution, canon, state, promise, window_plan, recent_continuity]
---

# 生成章节计划

## 目标

在动笔前先得到“可执行的章节骨架”，避免写作中途结构崩塌。

## 规划流程

1. 定位本章功能：推进主线/深化人物/铺垫伏笔/回收伏笔/情绪缓冲。
2. 场景拆分：3-5个场景，每个场景仅承担一个主目标。
3. 节奏编排：快慢交替，高潮不连续堆叠。
4. 字数分配：按场景功能给出比例而非平均分。
5. 结尾钩子：给出下一章驱动点。

## 类型差异提醒

1. 玄幻：场景必须显式体现规则代价。
2. 都市：对话与关系推进优先。
3. 科幻：每个技术点都要落在人物选择上。
4. 悬疑：每个场景至少有一条线索操作（投放/误导/校正）。

<!-- PROMPT_TEMPLATE_START -->
你是一名章节结构规划编辑。请基于上下文生成可直接执行的章节计划。

[项目上下文]
{projectContext}

[章节上下文]
{chapterContext}

[用户要求]
{userInstruction}

执行要求：
1. 必须只输出一个 JSON 对象，不要 Markdown 代码块，不要解释文本，不要前后缀。
2. 字段必须使用以下命名（不要新增字段）：
   - chapterFunction
   - successCriteria
   - emotionalArc
   - scenes
   - foreshadowingPlan
   - totalWords
   - cliffhanger
   - notes
   - status
3. scenes 必须为数组，每个元素至少包含：purpose, conflict, keyEvent, tension, estimatedWords。
4. totalWords 必须为整数；status 仅允许：planned / drafting / revising / completed。
5. 输出内容要可直接回填章节数据，避免“分析报告体”。
<!-- PROMPT_TEMPLATE_END -->
