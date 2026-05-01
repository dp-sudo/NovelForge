---
id: relationship.review
name: 关系审阅
description: 审阅角色关系网络并回填关系边，识别阶段跳变、动机冲突与剧情脱节风险，给出可执行修复方案
version: 2
source: builtin
category: review
tags: [关系, 角色, 审阅]
inputSchema:
  type: object
  properties:
    userInstruction: { type: string }
    projectContext: { type: string }
outputSchema:
  type: object
  properties:
    summary: { type: string }
    relationGraph:
      type: object
requiresUserConfirmation: false
writesToProject: true
author: NovelForge
icon: "🕸️"
createdAt: 2026-04-29
updatedAt: 2026-04-30
skillClass: review
bundleIds: [bundle.emotion-progression, bundle.character-expression]
alwaysOn: false
triggerConditions: [relationship.review]
requiredContexts: [chapter, canon, state]
stateWrites: [relationship.relationship]
automationTier: confirm
sceneTags: [dialogue, emotion]
affectsLayers: [canon, state, promise, recent_continuity]
---

# 关系审阅

## 目标

将“角色关系是否合理”拆成可检查指标，避免关系线跳变、空转和无因推进。

## 关系类型分类与定义

1. 血缘：亲属链与家族责任绑定。
2. 友情：基于共同经历和信任累积。
3. 爱情：情感依赖与长期承诺冲突并存。
4. 敌对：目标对立且存在持续博弈。
5. 师徒：知识与权力不对称关系。
6. 竞争：同目标赛道上的资源/地位争夺。
7. 隐藏关系：表层关系与真实关系不一致。

每条关系都要标注：显性关系、隐性关系、关系驱动事件。

## 关系发展阶段判定

标准阶段：陌生 -> 认识 -> 熟悉 -> 信任 -> 深厚
对立链路：陌生 -> 警惕 -> 冲突 -> 敌对 -> 破局/决裂

阶段判定依据（至少命中2项）：

1. 信息共享深度。
2. 风险共担程度。
3. 利益绑定程度。
4. 情绪暴露程度。

## 关系跳变识别与修正

跳变定义：阶段跨越超过1级且无可见触发事件。

修正方法：

1. 补触发事件：新增关键场景解释阶段变化。
2. 降级结论：把当前关系判定降回可支持阶段。
3. 拆分路径：先建立临时合作，再转长期信任。

## 关系网络构建与可视化规范

最少输出关系图要素：

1. 节点：角色名 + 当前阶段。
2. 边：关系类型 + 强度(1-5) + 最新触发事件。
3. 方向：单向依赖或双向互动。
4. 热点：冲突集中节点（高度数角色）。

## 动机冲突识别（Want vs Need）

每个核心角色必须区分：

1. Want：角色主观追求。
2. Need：角色真正需要的成长方向。

冲突判定：

1. want 与 need 完全一致 -> 剧情阻力不足。
2. want 与 need 完全对立 -> 容易形成强驱动。

审阅重点：关系变化是否由want/need冲突触发，而非作者硬推。

## 关系与剧情推进互动

1. 关系应触发剧情，而不是只在对话里“被提到”。
2. 每条主关系线至少绑定一个剧情节点（冲突、选择或代价）。
3. 高强度关系变化应伴随剧情成本（受伤、失去、背叛、牺牲之一）。

## 案例与修正示例

Case A（跳变）：
第8章前两人互不信任，第9章直接“生死之交”。

修正：
在第9章前补“共同承担生死风险”的事件，或将第9章关系改为“战术同盟”。

Case B（动机脱节）：
角色A口头仇恨角色B，却持续无代价帮助B。

修正：
补充隐藏动机（共同敌人/债务/亲属关联）并在关系边上标注“隐藏关系”。

<!-- PROMPT_TEMPLATE_START -->
你是一名长篇小说关系网络审阅员。请根据项目上下文输出可执行的关系审阅报告。

[项目上下文]
{projectContext}

[附加要求]
{userInstruction}

执行要求：
1. 识别并分类主要关系：血缘/友情/爱情/敌对/师徒/竞争/隐藏。
2. 对每条核心关系判定阶段（陌生->认识->熟悉->信任->深厚 或对立链路）。
3. 检测阶段跳变，定位触发事件缺口，并给出修复方案。
4. 检测角色want与need冲突是否支撑关系变化。
5. 说明关系对剧情推进的影响：哪些关系在推进，哪些关系在空转。
6. 必须只输出一个 JSON 对象，不要 Markdown 代码块，不要解释文本，不要前后缀。
7. 字段必须包含：
   - summary
   - relationGraph
   - stageAssessments
   - jumpRisks
   - motivationConflicts
   - plotImpact
   - fixesByPriority
8. relationGraph 必须包含 edges 数组；每个 edge 至少包含：
   - sourceName（必须使用角色库中已有角色名或别名）
   - targetName（必须使用角色库中已有角色名或别名）
   - relationshipType
   - description
9. 输出内容用于自动回填关系图，禁止只给抽象诊断而缺失可落库 edges。
<!-- PROMPT_TEMPLATE_END -->
