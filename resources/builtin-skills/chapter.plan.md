---
id: chapter.plan
name: 生成章节计划
description: 结合项目设定和章节上下文生成结构化纲目规划，包含场景拆分、节奏曲线、字数与伏笔安排
version: 3
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
    plan: { type: string }
requiresUserConfirmation: false
writesToProject: false
author: NovelForge
icon: "📋"
createdAt: 2026-04-28
updatedAt: 2026-04-29
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
1. 先给出本章功能定位与完成标准。
2. 拆分3-5个场景，每个场景给出：目的、冲突、关键事件、张力值(1-5)、预计字数。
3. 规划节奏曲线（开端->发展->转折->收束）并标注至少1个高潮点。
4. 标注伏笔处理：本章新埋伏笔与本章回收伏笔。
5. 给出章节结尾钩子和下一章承接建议。

输出格式：仅输出 JSON 对象，字段必须包含：
- chapterFunction
- successCriteria
- emotionalArc
- scenes
- foreshadowingPlan
- totalWords
- cliffhanger
- notes

禁止输出解释性前言、禁止Markdown代码块。
<!-- PROMPT_TEMPLATE_END -->
