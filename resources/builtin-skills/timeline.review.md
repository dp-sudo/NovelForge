---
id: timeline.review
name: 时间线审阅
description: 审阅全书时间线与因果链，识别时间矛盾、多线错位与回收窗口风险
version: 1
source: builtin
category: review
tags: [时间线, 审阅, 风险]
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
icon: "⏱️"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 时间线审阅

## 目标

把“看起来顺”升级为“时间、事件、因果三线一致”，避免跳时空、因果断链、年龄/季节/路程等硬伤。

## 时间线构建基本原则

1. 单位统一：统一使用“日期+时段”或“相对天数”之一。
2. 事件唯一定位：每个关键事件必须有时间锚点。
3. 因果先后明确：结果事件时间不能早于原因事件。
4. 成本可计算：旅行、恢复、修炼、调查都要有最小时长。

## 时间矛盾类型与识别方法

1. 顺序矛盾：后发生事件在前文被当作已发生。
2. 时长矛盾：任务完成时长短于最低可行时长。
3. 人物分身矛盾：同角色在同一时段出现在互斥地点。
4. 年龄/季节矛盾：年龄增长、节令变化与章节跨度不匹配。
5. 倒计时矛盾：已设定截止事件被无解释延迟。

识别动作：逐事件标注 `timeAnchor -> prerequisite -> completionWindow`，自动查逆序和重叠冲突。

## 时间跳跃处理技术

1. 显式跳跃：使用明确时间标记（如“三日后”）并补过渡结果。
2. 隐式跳跃：通过状态变化体现时间流逝（伤势愈合、物资变化、关系变化）。
3. 禁止硬跳：无时间标记且状态突变即判为风险。

## 多线程叙事时间线管理

每条线程需独立维护：

1. 主线线程（主角行动）
2. 对手线程（反派行动）
3. 支线线程（配角任务）

合流规则：

1. 合流前需检查信息同步条件是否成立。
2. 合流点必须可解释“为什么此时碰面”。
3. 多线程同时推进时，避免同一章节塞入超过2次大跳跃。

## 时间标记系统设计

建议输出两套标记：

1. StoryClock：故事内绝对时间（如Y3-M5-D12-Night）。
2. ReaderClock：读者感知顺序（Chapter-Scene编号）。

两者映射可定位“读者顺序合理但故事时序错误”的隐蔽问题。

## 因果链与时间线对应关系

每个关键事件至少包含：

1. causeEventId：前因事件。
2. effectEventId：后果事件。
3. causalDelay：因果延迟时长。
4. validity：causalDelay 是否在设定允许范围内。

## 问题案例与修正

Case A（时长矛盾）：
角色当天跨越两城并参与三场战斗。

修正：
拆分为两章，补交通手段或缩减战斗场次。

Case B（倒计时失效）：
“七天后处刑”写到第十天仍未执行。

修正：
补“处刑延期原因”节点，或前移关键救援事件时间。

<!-- PROMPT_TEMPLATE_START -->
你是一名长篇小说时间线审阅员。请根据项目上下文输出时间线与因果链风险报告。

[项目上下文]
{projectContext}

[附加要求]
{userInstruction}

执行要求：
1. 构建关键事件时间表，检查顺序、时长、地点、年龄/季节一致性。
2. 标记时间跳跃并判断是否有足够过渡。
3. 对多线程叙事检查合流逻辑和信息同步条件。
4. 对每个问题给出风险级别、冲突证据、最小修正方案。
5. 输出必须覆盖时间线与因果链映射关系。

输出格式：仅输出 JSON 对象，字段必须包含：
- summary
- eventTimeline
- contradictionFindings
- multiThreadRisks
- causalMappingRisks
- fixesByPriority

禁止输出解释性前言、禁止Markdown代码块。
<!-- PROMPT_TEMPLATE_END -->
