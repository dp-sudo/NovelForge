---
id: world.create_rule
name: 创建世界规则
description: 根据用户描述生成可叙事化的世界规则，强调有限性、代价与冲突生成能力
version: 4
source: builtin
category: world
tags: [世界观, 规则, 设定]
inputSchema:
  type: object
  properties:
    userDescription: { type: string }
    projectContext: { type: string }
  required: [userDescription]
outputSchema:
  type: object
  properties:
    worldRule: { type: object }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "🌍"
createdAt: 2026-04-28
updatedAt: 2026-04-30
skillClass: workflow
bundleIds: [bundle.rule-fulfillment]
alwaysOn: false
triggerConditions: [world.create_rule]
requiredContexts: [constitution, canon]
stateWrites: []
workflowStages: [define_rule, validate_rule, promote_rule]
postTasks: []
automationTier: supervised
sceneTags: [environment]
affectsLayers: [constitution, canon, promise]
---

# 创建世界规则

## 目标

生成“能被剧情反复调用”的规则，而不是只存在于设定文档的背景说明。

## 有限性原则（核心）

每条规则必须同时回答四个问题：

1. 能做什么（能力上限）
2. 不能做什么（硬边界）
3. 代价是什么（资源/身体/伦理/关系）
4. 何时失效（环境/条件/反制）

若缺任一项，规则不可入库。

## 规则与剧情冲突结合方式

至少绑定一种冲突机制：

1. 资源争夺：规则使用受稀缺资源约束。
2. 伦理冲突：规则可用但道德代价高。
3. 身份冲突：不同阶层对规则的访问权不同。
4. 反制冲突：规则可被特定手段克制。

## 不同世界类型侧重点

### 玄幻/仙侠/奇幻

1. 修炼层级与突破门槛。
2. 法术/血脉的代价与反噬。
3. 宗门/势力对规则解释权。

### 都市异能

1. 异能暴露成本（法律/社会/组织追捕）。
2. 日常场景下的能力限制。
3. 能力使用对关系网络的破坏。

### 科幻

1. 技术边界与能源约束。
2. 技术普及后社会结构变化。
3. 技术失控后的灾难路径。

### 历史/架空历史

1. 制度与礼法限制。
2. 军政资源分配逻辑。
3. 信息传播速度对规则生效范围的影响。

## 规则文档叙事化技巧

1. 用“角色遭遇”解释规则，而非条文堆砌。
2. 给出“成功使用案例”和“失败惩罚案例”。
3. 明确“普通人如何感知该规则”。

## 常见错误与修正

1. 错误：规则无限强，剧情无阻力。
修正：补反制条件和失效边界。
2. 错误：规则只在说明里存在，正文不体现。
修正：增加可触发场景与行为结果字段。
3. 错误：规则互相冲突却无解释。
修正：补interactionRule和优先级。

## 前后对比示例

Before：
“该世界人人可御火。”（无边界无代价）

After：
“御火需燃烧体内灵脂，连续施术超过三次将导致经脉灼损；雨夜威力降低40%；禁灵阵可完全压制。”

<!-- PROMPT_TEMPLATE_START -->
你是一名长篇小说世界观架构师。请根据用户设想生成可直接用于剧情的世界规则。

[项目上下文]
{projectContext}

[用户设想]
{userDescription}

执行要求：
1. 必须只输出一个 JSON 对象，不要 Markdown 代码块，不要解释文本，不要前后缀。
2. 字段必须使用以下命名（不要新增字段）：
   - title
   - category
   - description
   - constraintLevel
   - relatedEntities (array of string)
   - examples
   - contradictionPolicy
3. description 中必须覆盖：能力边界、代价、失效条件、冲突机制。
4. constraintLevel 仅允许：weak / normal / strong / absolute。
5. 内容需可直接入库并在世界规则列表展示，避免输出“分析报告体”。
<!-- PROMPT_TEMPLATE_END -->
