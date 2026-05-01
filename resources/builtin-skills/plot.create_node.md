---
id: plot.create_node
name: 创建剧情节点
description: 根据用户描述生成结构化剧情节点，强调因果驱动、冲突强度与节点网络可扩展性
version: 4
source: builtin
category: plot
tags: [剧情, 主线, 节点]
inputSchema:
  type: object
  properties:
    userDescription: { type: string }
    projectContext: { type: string }
  required: [userDescription]
outputSchema:
  type: object
  properties:
    plotNode: { type: object }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "🔗"
createdAt: 2026-04-28
updatedAt: 2026-04-30
skillClass: workflow
bundleIds: [bundle.rule-fulfillment]
alwaysOn: false
triggerConditions: [plot.create_node]
requiredContexts: [canon, state, window_plan]
stateWrites: [window.progress]
automationTier: supervised
sceneTags: []
affectsLayers: [canon, state, promise, window_plan]
---

# 创建剧情节点

## 目标

把单个事件设计成“能接前、能推后、能挂网”的剧情节点，而不是孤立桥段。

## 节点设计原则

1. 因果优先：节点必须回答“为何发生”。
2. 冲突驱动：无冲突节点应降级为过渡信息。
3. 信息增量：每节点至少新增一个关键变化。
4. 伏笔闭环：回收与种植要成对管理。

## 节点网络构建方法（新增）

### Step 1：节点分层

1. 主干节点（A类）：主线不可删除节点。
2. 支撑节点（B类）：解释动机、关系、代价。
3. 缓冲节点（C类）：节奏调节与情绪回落。

### Step 2：边类型定义

1. 因果边：A导致B。
2. 信息边：A提供B所需信息。
3. 情感边：A改变角色关系张力。
4. 伏笔边：A种植，B回收。

### Step 3：网络健康检查

1. 孤岛检查：无入边且无出边节点 -> 删除或并入。
2. 单线过密：连续3个同类型冲突节点 -> 插入缓冲或反转。
3. 回收断裂：种植边无终点 -> 标为高风险。
4. 角色失踪：核心角色连续多节点缺席且无解释 -> 补支撑节点。

### Step 4：可视化最小字段

每节点至少输出：

1. nodeId
2. layer(A/B/C)
3. incomingEdges
4. outgoingEdges
5. payoffWindow

## 冲突类型与使用

1. 人物 vs 人物：直接对抗，张力高。
2. 人物 vs 环境：生存与资源压力。
3. 人物 vs 自我：深层成长驱动。
4. 人物 vs 社会：制度约束冲突。
5. 人物 vs 命运：悲剧与宿命感。

## 伏笔管理三原则

1. 种植：可见但不过曝。
2. 回收：有时机、有后果。
3. 误导：红鲱鱼需可反证。

<!-- PROMPT_TEMPLATE_START -->
你是一名剧情架构师。请根据用户设想生成可落地剧情节点，并说明其在节点网络中的位置。

[项目上下文]
{projectContext}

[用户设想]
{userDescription}

执行要求：
1. 必须只输出一个 JSON 对象，不要 Markdown 代码块，不要解释文本，不要前后缀。
2. 字段必须使用以下命名（不要新增字段）：
   - title
   - nodeType
   - goal
   - conflict
   - emotionalCurve
   - status
   - relatedCharacters (array of string)
3. status 建议使用：planning / drafted / active / resolved。
4. 若你生成了 layer/incomingEdges/outgoingEdges/payoffWindow 等扩展信息，必须同时汇总进 goal/conflict 中，保证可直接入库展示。
5. 不要输出“步骤说明/网络分析报告”，仅输出可入库字段。
<!-- PROMPT_TEMPLATE_END -->
