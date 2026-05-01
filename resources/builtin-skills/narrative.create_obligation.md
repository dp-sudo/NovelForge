---
id: narrative.create_obligation
name: 创建叙事义务
description: 根据用户描述生成可追踪的叙事义务，覆盖伏笔类型、种植策略、回收计划与风险评估
version: 2
source: builtin
category: narrative
tags: [叙事, 伏笔, 回收]
inputSchema:
  type: object
  properties:
    userDescription: { type: string }
    projectContext: { type: string }
  required: [userDescription]
outputSchema:
  type: object
  properties:
    obligation: { type: object }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "🧵"
createdAt: 2026-04-29
updatedAt: 2026-04-30
skillClass: workflow
bundleIds: [bundle.rule-fulfillment]
alwaysOn: false
triggerConditions: [narrative.create_obligation]
requiredContexts: [constitution, canon, promise]
stateWrites: []
automationTier: supervised
sceneTags: []
affectsLayers: [constitution, canon, promise, window_plan]
---

# 创建叙事义务

## 目标

把“埋伏笔”从灵感行为改为可追踪、可回收、可评估风险的结构化义务管理。

## 伏笔类型分类（必须标注）

1. 明线伏笔：读者能意识到“这句话有事”，但不知道具体回收方式。
2. 暗线伏笔：当下像日常细节，回收时才显出意义。
3. 反讽伏笔：角色判断与真实走向相反，回收时制造认知反差。
4. 双关伏笔：同一句话在前后语境含义不同，回收时形成二次解释。

## 伏笔种植技术

### 1) 位置选择

1. 章节开头：适合明线伏笔，抓住读者注意。
2. 场景切换处：适合暗线伏笔，降低“刻意感”。
3. 冲突缓冲段：适合双关伏笔，读者防备心较低。

### 2) 显隐程度

1. 高显性（明线）：读者会记住，但猜中风险高。
2. 中显性（推荐默认）：读者隐约记住，回收惊喜和合理性平衡。
3. 低显性（暗线）：回收冲击大，但忘记风险高。

### 3) 暗示强度（1-5）

1. 1-2：背景级提示，几乎不打断主叙事。
2. 3：能被注意但不抢戏，建议默认。
3. 4-5：强提醒，适合关键主线转折。

## 回收时机判定逻辑

按以下顺序判断：

1. 因果成熟：触发条件是否已出现？
2. 情绪窗口：角色和读者是否已进入“可承受信息释放”的节点？
3. 叙事收益：现在回收能否同时推进主线和角色弧？

若三项中有两项不成立，延后回收；禁止“为了收而收”。

## 回收方式选择

1. 直接揭示：用明确事件揭晓真相，适合主线关键义务。
2. 反转揭示：先给错误解释再翻转，适合悬疑与权谋。
3. 行为兑现：不解释，通过角色行动让伏笔成立，适合情感线。
4. 代价回收：回收伴随损失，适合提升重量感。

## 未回收风险评估模型（RAG）

风险分 = 影响范围(1-5) x 读者记忆度(1-5) x 延迟时长系数(1-3)

1. 低风险（1-15）：支线细节，可后续补回。
2. 中风险（16-40）：人物动机或小主线，需在 2-5 章内回收。
3. 高风险（41-75）：核心因果或主题义务，必须优先回收。

## 叙事义务追踪表模板

每条义务都要生成以下字段：

1. obligationId：唯一ID。
2. obligationType：明线/暗线/反讽/双关。
3. seedLocation：埋点章节与段落。
4. seedSignal：具体埋点句。
5. triggerCondition：回收触发条件。
6. payoffWindow：建议回收区间（章节范围）。
7. payoffMode：直接/反转/行为/代价。
8. linkedPlotNode：关联剧情节点。
9. linkedCharacterArc：关联角色弧光节点。
10. riskScore：风险分。
11. riskLevel：low/medium/high。
12. fallbackPlan：若延期回收，如何补救。

## 与剧情节点和角色弧的关系

1. 每个关键伏笔至少绑定一个剧情节点，否则容易“漂浮”。
2. 每个角色主弧至少有一条“内在冲突型”伏笔，否则成长显突兀。
3. 主线回收时优先触发角色选择，不要只做信息公布。

## 常见错误与修正

1. 错误：伏笔只“神秘”，不“有用”。
修正：补写触发条件和回收收益字段。
2. 错误：回收只解释过去，不推动现在。
修正：让回收直接改变角色决策或冲突态势。
3. 错误：高显性伏笔过密，读者疲劳。
修正：同一章高显性伏笔不超过2条。

## 前后对比示例

Before：
“她看了一眼旧怀表，神色复杂。”（没有说明用途，后续也未绑定）

After：
“她拇指在怀表裂痕上停了一秒，那道裂痕和三年前案发现场门锁上的刮痕形状一致。”
并绑定：triggerCondition=案卷重启；payoffWindow=第12-14章；linkedPlotNode=旧案真凶浮出。

<!-- PROMPT_TEMPLATE_START -->
你是一名长篇小说叙事结构编辑。请把用户设想转为“可追踪、可回收、可评估风险”的叙事义务条目。

[项目上下文]
{projectContext}

[用户设想]
{userDescription}

执行要求：
1. 必须只输出一个 JSON 对象，不要 Markdown 代码块，不要解释文本，不要前后缀。
2. 字段必须使用以下命名（不要新增字段）：
   - obligationType
   - description
   - plantedChapterId
   - expectedPayoffChapterId
   - actualPayoffChapterId
   - payoffStatus
   - severity
   - relatedEntities (array of string)
3. description 中必须包含：埋点信号、触发条件、回收窗口、回收方式、延期补救方案。
4. payoffStatus 建议使用：open / in_progress / fulfilled / dropped。
5. severity 仅允许：low / medium / high；内容需可直接入库并在叙事义务列表展示。
<!-- PROMPT_TEMPLATE_END -->
