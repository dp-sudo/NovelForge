---
id: export.review
name: 导出前审阅
description: 导出前全书终检，识别一致性、衔接、伏笔与角色弧光风险并给出最小修复序列
version: 1
source: builtin
category: review
tags: [导出, 审阅, 终检]
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
icon: "📤"
createdAt: 2026-04-29
updatedAt: 2026-04-29
---

# 导出前审阅

## 目标

在导出前拦截读者一轮即可感知的硬伤，输出“先修什么、怎么最小修”的终检方案。

## 终检流程（按顺序执行）

1. 术语一致性检查。
2. 章节衔接检查。
3. 伏笔闭环检查。
4. 角色弧光连续性检查。
5. 收尾质量评估。

任何步骤出现高风险项，直接进入 mustFix，不等待后续步骤“抵消”。

## 一、术语一致性检查（具体方法）

### 执行方法

1. 抽取高频术语：统计出现频次 >= 3 的专有词。
2. 同义归并：检查同概念多称呼（如“灵脉/灵络”）是否未声明别名关系。
3. 拼写变体检查：全角半角、繁简、大小写、空格连写差异。
4. 语义漂移检查：同一术语在不同章节定义是否变化。

### 判定标准

1. 同概念多称呼且无说明 -> error。
2. 拼写变体不影响理解但会误检索 -> warning。
3. 概念定义前后冲突 -> error。

## 二、章节衔接检查（逐项清单）

逐章节对照“上一章结尾 -> 本章开头”：

1. 时间衔接：时段是否连续，跳时是否有过渡。
2. 空间衔接：角色位置迁移是否可解释。
3. 情绪衔接：情绪温度变化是否过陡。
4. 目标衔接：角色本章行动是否承接上章目标。
5. 信息衔接：新信息是否基于前文触发。

判定：

1. 任一项断裂且影响理解 -> error。
2. 可理解但略突兀 -> warning。

## 三、伏笔闭环判定标准

将伏笔分为三类并独立评估：

1. 主线伏笔：关系主冲突与终局解释。
2. 角色伏笔：角色动机、秘密、身份。
3. 氛围伏笔：世界观细节与气氛暗示。

闭环判定：

1. 主线伏笔未回收或回收不足 -> error。
2. 角色伏笔逾期且影响行为动机 -> warning/error（按影响）。
3. 氛围伏笔可保留未回收，但不得误导核心因果 -> info/warning。

回收有效性标准：

1. 回收必须改变理解或推动事件。
2. 只“提到”不“兑现”不算回收。

## 四、角色弧光跳变识别方法

### 检查模型（Want/Need/Action）

每个核心角色检查三元链：

1. Want（表层目标）是否稳定。
2. Need（深层需求）是否有阶段推进。
3. Action（关键行动）是否由Want/Need驱动。

### 跳变判定

1. 立场突然反转且无触发事件 -> error。
2. 性格突然变形但可补触发 -> warning。
3. 行动与既有动机弱相关 -> warning。

## 五、收尾质量评估标准

收尾至少满足四项：

1. 结局清晰度：主冲突是否明确收束或明确留钩。
2. 情绪完成度：读者是否获得情绪结算（释然/震荡/期待）。
3. 信息完整度：关键问题是否给出应有答案层级。
4. 续篇牵引度：若有下一部，钩子是否来自既有因果而非硬插。

常见失分项：

1. 结尾突然“宣告式”总结。
2. 关键人物命运交代缺失。
3. 为留悬念故意掐断关键解释。

## 问题案例与修正示例

Case A（术语冲突）：
第6章“禁灵结界”，第10章写成“禁灵界壁”且未声明同义。

修正：
统一规范名为“禁灵结界”，将“禁灵界壁”登记别名并回改正文。

Case B（章节断裂）：
上章结尾“夜间追逐”，下章开头直接“三天后庆功宴”无过渡。

修正：
在下章首段补“追逐后处理与时间推进”两句过渡。

Case C（角色跳变）：
角色B长期谨慎，第18章突然公开自曝核心秘密。

修正：
补写触发事件（被迫交换、保护对象受威胁）并前置心理铺垫。

## 输出策略

1. 按 mustFix / optionalImprovements 分层。
2. mustFix 按“影响主线因果 > 影响角色动机 > 影响阅读顺滑”排序。
3. 每条问题提供最小修复方案，避免连锁重写。

<!-- PROMPT_TEMPLATE_START -->
你是一名小说导出前终审编辑。请基于项目上下文输出终检报告。

[项目上下文]
{projectContext}

[附加要求]
{userInstruction}

执行要求：
1. 用可执行方法检查五类问题：术语一致性、章节衔接、伏笔闭环、角色弧光、收尾质量。
2. 每条问题必须包含：riskLevel、category、evidence、impact、minimalFix、affectedChapters。
3. 将结果分为 mustFix 与 optionalImprovements。
4. 给出最终导出前处理顺序（按修复收益排序）。

输出格式：仅输出 JSON 对象，字段必须包含：
- summary
- mustFix
- optionalImprovements
- finalChecklist
- recommendedFixOrder

禁止输出解释性前言、禁止Markdown代码块。
<!-- PROMPT_TEMPLATE_END -->
