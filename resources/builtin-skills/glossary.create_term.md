---
id: glossary.create_term
name: 创建术语条目
description: 根据用户描述生成可落库术语，覆盖分类、最小完整定义、冲突检测与跨类关联
version: 2
source: builtin
category: glossary
tags: [术语, 词条, 一致性]
inputSchema:
  type: object
  properties:
    userDescription: { type: string }
    projectContext: { type: string }
  required: [userDescription]
outputSchema:
  type: object
  properties:
    term: { type: object }
requiresUserConfirmation: true
writesToProject: false
author: NovelForge
icon: "📘"
createdAt: 2026-04-29
updatedAt: 2026-04-30
skillClass: workflow
bundleIds: [bundle.rule-fulfillment]
alwaysOn: false
triggerConditions: [glossary.create_term]
requiredContexts: [constitution, canon]
stateWrites: []
workflowStages: [extract_term, define_term, promote_term]
postTasks: []
automationTier: supervised
sceneTags: []
affectsLayers: [constitution, canon, lexicon_policy]
---

# 创建术语条目

## 目标

把“一个词”变成可长期复用、可消歧、可关联的小说知识节点，避免后期出现命名冲突和设定漂移。

## 术语分类体系（必须二级分类）

一级分类（必选其一）：

1. 人名
2. 地名
3. 组织名
4. 能力名
5. 物品名
6. 概念名
7. 法则名

二级标签（可多选）：

1. 阵营标签：主角方/反派方/中立。
2. 时间标签：古代遗留/当代出现/未来概念。
3. 风险标签：高混淆/高依赖/高剧透。

## 术语最小完整性原则（MCP）

每条术语至少包含：

1. canonicalName：规范名。
2. category：一级分类。
3. oneLineDefinition：一句话定义（不超过40字）。
4. scopeBoundary：适用边界（在哪些场景能用，哪些不能用）。
5. firstUseContext：首次出现语境。
6. forbiddenMisuse：常见误用。

缺任一项即视为“不可入库”。

## 冲突检测流程

按顺序执行：

1. 同名冲突：是否已有同名词条？
2. 近形冲突：拼写近似或发音近似是否会混淆？
3. 同义冲突：不同名是否指向同一概念？
4. 规则冲突：定义是否与世界规则、角色卡、剧情节点矛盾？

冲突结果输出：

1. no_conflict：可直接入库。
2. merge_required：应合并到既有词条。
3. rename_required：需改名后再入库。
4. redefine_required：需重写定义。

## 既有术语整合优先级

发生冲突时按优先级处理：

1. 已落地章节正文中高频术语。
2. 主线剧情依赖术语。
3. 世界规则中的基础术语。
4. 最近新增且未在正文使用的术语。

原则：优先保留“已被读者看到且依赖链更长”的术语。

## 使用语境说明要求

术语条目必须显式说明：

1. 场景语境：战斗/议会/校园/实验室等。
2. 说话主体：谁会使用该术语。
3. 语气等级：正式/口语/黑话/古语。
4. 禁用语境：哪些场合使用会违和。

## 跨类别关联建模

术语至少建立 1 条关系：

1. is_part_of：从属关系（组织->势力体系）。
2. depends_on：依赖关系（能力->法则）。
3. conflicts_with：冲突关系（概念A与法则B互斥）。
4. alias_of：别名关系。
5. evolves_to：阶段演化关系（物品旧称->新称）。

## 常见错误与修正

1. 错误：定义抽象，读完仍不知如何使用。
修正：补写具体“可用场景+禁用场景”。
2. 错误：同义新词重复造词。
修正：改为alias_of既有词条。
3. 错误：术语无上下文证据。
修正：补firstUseContext和evidence片段。

## 前后对比示例

Before：
术语：灵脉。定义：世界里的神秘能量通道。

After：
canonicalName=灵脉；category=法则名；
oneLineDefinition=天地灵气在地底形成的稳定流道；
scopeBoundary=仅在修炼与阵法场景使用，不用于日常口语；
firstUseContext=第2章宗门入门考核；
forbiddenMisuse=不可与“经脉”混用；
relations=[depends_on:聚灵阵, conflicts_with:禁灵结界]。

<!-- PROMPT_TEMPLATE_START -->
你是一名小说术语体系编辑。请把用户设想转成可入库、可检索、可消歧的术语条目。

[项目上下文]
{projectContext}

[用户设想]
{userDescription}

执行要求：
1. 必须只输出一个 JSON 对象，不要 Markdown 代码块，不要解释文本，不要前后缀。
2. 字段必须使用以下命名（不要新增字段）：
   - term
   - termType
   - aliases (array of string)
   - description
   - locked
   - banned
3. description 中必须包含：一句话定义、适用边界、首次语境、禁用误用、冲突结论与整合建议。
4. termType 只能取：人名 / 地名 / 组织名 / 能力名 / 物品名 / 概念名 / 法则名 / 术语。
5. 内容需可直接入库并在名词库展示，禁止输出“流程说明/分析报告”。
<!-- PROMPT_TEMPLATE_END -->
