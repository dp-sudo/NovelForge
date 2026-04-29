---
id: dashboard.review
name: 仪表盘诊断
description: 基于项目全局状态生成创作阶段诊断，量化进度、风险与优先级执行序列
version: 1
source: builtin
category: review
tags: [仪表盘, 诊断, 优先级]
inputSchema:
  type: object
  properties:
    userInstruction: { type: string }
    projectContext: { type: string }
outputSchema:
  type: object
  properties:
    report: { type: string }
requiresUserConfirmation: false
writesToProject: false
author: NovelForge
icon: "📊"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 仪表盘诊断

## 目标

将“写到哪了、哪里危险、先修什么”结构化输出，服务一键生成链路中的快速决策。

## 创作进度评估维度

必须输出以下四项完成度（0-100）：

1. 情节完成度：主线节点覆盖率、关键冲突是否落地。
2. 角色发展度：主角弧和核心配角弧是否形成阶段推进。
3. 伏笔回收率：已种伏笔中已回收比例与逾期比例。
4. 世界观完整度：核心规则是否具备边界、代价、冲突场景。

## 风险识别模型

风险分类：

1. 信息缺失：关键设定或节点未定义。
2. 逻辑漏洞：因果、动机、时间线不成立。
3. 节奏失衡：高潮堆叠或长期平铺。
4. 前后矛盾：术语、关系、规则冲突。

风险评分模型：

riskScore = severity(1-5) x impact(1-5) x urgency(1-5)

分级：

1. P0（>=60）：立即修复，否则继续生成会放大错误。
2. P1（30-59）：本轮创作内修复。
3. P2（<30）：可排入后续批次。

## 优先级排序框架

对每条问题计算排序键：

1. 紧急度：是否阻断下一步生成。
2. 重要度：对主线质量影响。
3. 依赖关系：是否为其他问题前置。

排序规则：先依赖、后紧急、再重要。

## 阶段诊断标准

1. 起步期：蓝图未稳，角色和规则空洞。
2. 展开期：章节增长快，但一致性风险开始积累。
3. 收束期：回收密度高，时间线和关系线最容易崩。
4. 出稿期：内容完整，终检和导出风险为主。

每次诊断必须判定当前阶段并说明依据。

## 建议生成方法论

建议必须满足：

1. 可执行：有明确动作和预期结果。
2. 小步快跑：优先最小修复单元。
3. 对齐一键生成：避免引入复杂人工流程。

建议模板：

1. action：做什么。
2. whyNow：为什么现在做。
3. expectedGain：修复收益。
4. dependency：依赖前置。
5. doneCriteria：完成判定。

## 分阶段针对性建议

1. 起步期：先补“主线冲突+主角动机+规则边界”三件套。
2. 展开期：每3章做一次关系/时间线体检。
3. 收束期：建立伏笔回收清单，按风险分优先回收。
4. 出稿期：先修P0一致性，再做文风润色。

<!-- PROMPT_TEMPLATE_START -->
你是一名长篇小说创作仪表盘诊断顾问。请根据项目上下文输出阶段诊断与优先级执行建议。

[项目上下文]
{projectContext}

[附加要求]
{userInstruction}

执行要求：
1. 量化四项进度：情节完成度、角色发展度、伏笔回收率、世界观完整度（0-100）。
2. 用风险模型识别问题并计算riskScore，分为P0/P1/P2。
3. 按“依赖->紧急->重要”排序给出修复序列。
4. 判定当前创作阶段（起步/展开/收束/出稿）并给出阶段专属建议。
5. 所有建议必须可执行并可验收（含doneCriteria）。

输出格式：仅输出 JSON 对象，字段必须包含：
- stageAssessment
- progressScores
- riskFindings
- priorityQueue
- actionPlan
- nextCheckpoint

禁止输出解释性前言、禁止Markdown代码块。
<!-- PROMPT_TEMPLATE_END -->
