---
id: blueprint.generate_step
name: 生成蓝图步骤
description: 基于项目设定为指定蓝图步骤生成可执行方案，支持阻塞诊断与多方案评估
version: 4
source: builtin
category: utility
tags: [蓝图, 规划, 生成]
inputSchema:
  type: object
  properties:
    stepKey: { type: string }
    stepTitle: { type: string }
    userInstruction: { type: string }
    projectContext: { type: string }
  required: [stepKey, stepTitle]
outputSchema:
  type: object
  description: 按 stepKey 返回对应蓝图字段的对象（字段值均为字符串）
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "🎯"
createdAt: 2026-04-28
updatedAt: 2026-04-30
skillClass: workflow
bundleIds: [bundle.rule-fulfillment]
alwaysOn: false
triggerConditions: [blueprint.generate_step]
requiredContexts: [constitution, canon]
stateWrites: []
automationTier: supervised
sceneTags: []
affectsLayers: [constitution, canon, promise]
---

# 生成蓝图步骤

## 目标

在作者卡住时，快速给出“可立即执行”的步骤方案，而不是空泛方向。

## 步骤类型差异化策略

根据 stepKey/stepTitle 先判定步骤类型：

1. 世界构建类：优先规则边界、代价与冲突潜力。
2. 角色构建类：优先欲望/恐惧/矛盾与弧光节点。
3. 主线推进类：优先因果链和冲突升级路径。
4. 章节执行类：优先场景拆分、节奏和字数配比。
5. 终局收束类：优先伏笔回收和主题闭环。

## 创作阻塞诊断模型

常见阻塞类型：

1. 选择困难：有多个方向但无法决策。
2. 信息缺口：关键设定缺失导致无法推进。
3. 节奏失衡：知道写什么但不知道怎么排布。
4. 风格漂移：内容能写但调性偏离项目。

每次输出先判定阻塞类型，再给对应解法。

## 多方案评估框架（必须执行）

对每个候选方案打分（1-5）：

1. 一致性：与现有设定冲突概率。
2. 推进力：对主线推进效率。
3. 情绪张力：读者情绪回报。
4. 实现成本：写作复杂度与补设定成本。
5. 复用价值：对后续章节的可持续贡献。

推荐方案 = 综合分最高，且“一致性>=4”。

## 场景化方案示例（新增）

### 示例A：世界构建步骤卡住

场景：用户要写“灵力为何衰竭”，但只有一句设想。

1. 方案1（历史灾变）：将衰竭绑定百年前大战，优点是厚重，缺点是需补历史线。
2. 方案2（制度垄断）：将衰竭解释为宗门垄断灵脉，优点是能直接制造阶层冲突。
3. 方案3（自然周期）：灵力周期性衰退，优点是可与终局倒计时结合。

推荐：方案2（推进主线冲突最快，一致性风险最低）。

### 示例B：角色步骤卡住

场景：主角“想复仇”，但行为总是摇摆。

1. 方案1：补“想要=复仇，需要=放下控制欲”的内在矛盾。
2. 方案2：改为“想要证明自己”，降低复仇权重。
3. 方案3：引入“复仇对象可能无辜”的道德冲突。

推荐：方案3（更能驱动角色弧光）。

### 示例C：章节执行步骤卡住

场景：第12章“很重要但写不动”。

1. 方案1：拆3场景（对峙->误判->反转）。
2. 方案2：拆5场景（铺垫->冲突->失利->缓冲->钩子）。
3. 方案3：双线并行（主角线+反派线）再合流。

推荐：方案2（风险可控，节奏更稳）。

## 输出原则

1. 每个方案都要包含“可直接写入蓝图的文本草案”。
2. 必须指出风险与触发条件，避免误用。
3. 最终给出首选与备选，便于一键确认。

<!-- PROMPT_TEMPLATE_START -->
你是一名小说蓝图推进教练。请围绕指定步骤给出“可直接落地”的方案。

[项目上下文]
{projectContext}

[步骤Key]
{stepKey}

[步骤标题]
{stepTitle}

[用户要求]
{userInstruction}

执行要求：
1. 必须只输出一个 JSON 对象，不要 Markdown 代码块，不要解释文本，不要前后缀。
2. 根据 stepKey 选择字段集合，并确保所有字段都输出且为非空字符串。
3. 内容要可直接写入对应蓝图表单，避免“方案分析/评分表/步骤清单”这类外层描述。
4. 不允许新增未定义字段。

字段集合（按 stepKey）：
- step-01-anchor：
  coreInspiration, coreProposition, coreEmotion, targetReader, sellingPoint, readerExpectation
- step-02-genre：
  mainGenre, subGenre, narrativePov, styleKeywords, rhythmType, bannedStyle
- step-03-premise：
  oneLineLogline, threeParagraphSummary, beginning, middle, climax, ending
- step-04-characters：
  protagonist, antagonist, supportingCharacters, relationshipSummary, growthArc
- step-05-world：
  worldBackground, rules, locations, organizations, inviolableRules
- step-06-glossary：
  personNames, placeNames, organizationNames, terms, aliases, bannedTerms
- step-07-plot：
  mainGoal, stages, keyConflicts, twists, climax, ending
- step-08-chapters：
  volumeStructure, chapterList, chapterGoals, characters, plotNodes
<!-- PROMPT_TEMPLATE_END -->
