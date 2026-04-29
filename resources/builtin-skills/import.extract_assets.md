---
id: import.extract_assets
name: 导入资产抽取
description: 从导入文稿中抽取角色、规则、术语和地点资产，并提供证据与置信度
version: 3
source: builtin
category: utility
tags: [导入, 资产, 抽取]
inputSchema:
  type: object
  properties:
    content: { type: string }
  required: [content]
outputSchema:
  type: object
  properties:
    assets: { type: object }
requiresUserConfirmation: true
writesToProject: true
author: NovelForge
icon: "📥"
createdAt: 2026-04-28
updatedAt: 2026-04-29
---

# 导入资产抽取

## 目标

把非结构化文稿快速转为可落库资产，减少手工建档成本。

## 抽取原则

1. 只抽取有文本证据的信息，不凭空推断。
2. 每条资产必须附evidence片段。
3. 相同资产先合并后输出，避免重复入库。
4. 不确定信息必须标注低置信度并给原因。

## 识别特征判定标准（新增）

### 角色识别评分

满足项每项+1：

1. 有明确姓名/称谓。
2. 有动作或对话。
3. 被其他角色指代并产生互动。
4. 在两处以上出现。

判定：

1. 分数>=3：高置信角色。
2. 分数=2：中置信，需人工确认。
3. 分数<=1：不入库或低置信候选。

### 世界规则识别评分

1. 出现“必须/不能/代价/限制”等规则词。
2. 关联多个角色或事件。
3. 可抽象为可复用约束。

判定：

1. >=2项：可入worldRules。
2. <2项：先作为notes，不直接入规则库。

### 术语识别评分

1. 词汇出现频次>=2。
2. 有定义句或解释句。
3. 在不同段落保持同义。

判定：

1. 3项命中：高置信术语。
2. 2项命中：中置信术语。
3. <=1项：可能是临时措辞，不入库。

### 地点识别评分

1. 有地点名。
2. 有空间特征描写。
3. 至少承载一个事件。

判定：

1. >=2项：入locations。
2. <2项：保留为候选。

## 核心资产类型

1. 角色：姓名、身份、特征、关系线索。
2. 世界规则：能力边界、制度约束、代价机制。
3. 术语：专有名词、定义、类别。
4. 地点：地点名、功能、关联事件。

## 常见错误与修正

1. 错误：把比喻词当术语。
修正：要求术语至少出现2次或有明确定义句。
2. 错误：一次性路人误判为核心角色。
修正：补“出现次数+行为权重”判断。
3. 错误：无证据高置信输出。
修正：无证据默认降为low。

<!-- PROMPT_TEMPLATE_START -->
你是一名小说资产抽取专员。请从导入文稿中提取可落库资产。

[输入内容]
{content}

执行要求：
1. 抽取角色、世界规则、术语、地点四类资产。
2. 每条资产必须包含evidence与confidence(high/medium/low)。
3. 按识别特征评分给出判定依据（命中项）。
4. 对重复或同义资产做合并，并标注mergeReason。
5. 对低置信结果说明原因，便于用户二次确认。

输出格式：仅输出 JSON 对象，字段必须包含：
- characters
- worldRules
- terms
- locations
- mergeLog

禁止输出解释性前言、禁止Markdown代码块。
<!-- PROMPT_TEMPLATE_END -->
