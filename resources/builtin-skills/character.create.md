---
id: character.create
name: 创建角色卡
description: 根据用户描述生成完整的结构化角色卡，包含基础信息、动机矛盾与阶段化弧光设计
version: 4
source: builtin
category: character
tags: [角色, 创建, 人物卡]
inputSchema:
  type: object
  properties:
    userDescription: { type: string }
    projectContext: { type: string }
  required: [userDescription]
outputSchema:
  type: object
  properties:
    characterCard: { type: object }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "👤"
createdAt: 2026-04-28
updatedAt: 2026-04-30
skillClass: workflow
bundleIds: [bundle.character-expression]
alwaysOn: false
triggerConditions: [character.create]
requiredContexts: [constitution, canon]
stateWrites: [character.involvement]
automationTier: supervised
sceneTags: []
affectsLayers: [constitution, canon, state, promise]
---

# 创建角色卡

## 目标

将用户灵感转为可持续驱动长篇剧情的角色结构，重点保证“动机可执行、弧光可分阶段推进”。

## 角色设计原则

1. 欲望驱动：角色必须有可行动的目标。
2. 内在矛盾：want 与 need 不能完全重合。
3. 缺陷真实：缺陷必须会在剧情中制造代价。
4. 成长预留：弧光要有阶段，不是结尾突然顿悟。
5. 细节具体：至少一个可识别行为习惯。

## 角色原型速查

| 原型 | 核心欲望 | 核心恐惧 | 典型矛盾 |
|------|----------|----------|----------|
| 英雄 | 证明自己 | 懦弱/不够强 | 能力越大责任越大 |
| 智者 | 追求真理 | 无知/被骗 | 知识越多越痛苦 |
| 守护者 | 保护他人 | 失去重要的人 | 保护与控制一线之隔 |
| 叛逆者 | 打破规则 | 被体制同化 | 自由与责任冲突 |
| 野心家 | 获得权力 | 被忽视 | 成功与道德的冲突 |
| 幸存者 | 活下去 | 再次受伤 | 信任与自我保护 |
| 探索者 | 寻找意义 | 安定/平庸 | 冒险与归属的矛盾 |

## 角色弧光阶段模板（新增）

每个核心角色至少输出4阶段：

1. 阶段1-起点：当前缺陷如何限制角色。
2. 阶段2-扰动：触发事件迫使角色失衡。
3. 阶段3-抉择：角色在want与need间做高代价选择。
4. 阶段4-新平衡：角色获得新认知并改变行为模式。

每阶段必须给：

1. triggerEvent（触发事件）
2. innerShift（内在变化）
3. outwardAction（外在行为）
4. failureCost（失败代价）

## 类型适配

1. 玄幻：修炼动机、力量代价、师门关系。
2. 都市：社会关系网、职业约束、现实利益。
3. 科幻：技术伦理立场与认知边界。
4. 悬疑：信息不对称与可信度层级。

<!-- PROMPT_TEMPLATE_START -->
你是一名角色设计顾问。请根据用户设想生成可用于长篇连载的结构化角色卡。

[项目上下文]
{projectContext}

[用户设想]
{userDescription}

执行要求：
1. 必须只输出一个 JSON 对象，不要 Markdown 代码块，不要解释文本，不要前后缀。
2. 所有字段都必须存在；允许空字符串，但关键字段（name, roleType, motivation）不得为空。
3. 字段必须使用以下命名（不要新增字段）：
   - name
   - aliases (array of string)
   - roleType
   - age
   - gender
   - identityText
   - appearance
   - motivation
   - desire
   - fear
   - flaw
   - arcStage
   - notes
4. 内容需可直接入库并在角色列表展示，不要输出“方案对比/说明段落”。
5. 若给出复杂设定（如 personalityProfile / arc 分阶段），必须同时汇总到上述扁平字段中。
<!-- PROMPT_TEMPLATE_END -->
