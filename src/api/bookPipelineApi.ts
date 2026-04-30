import {
  cancelTaskPipeline,
  streamTaskPipeline,
  type AiPipelineEvent,
  type RunTaskPipelineInput,
} from "./pipelineApi.js";

export type BookStageKey =
  | "blueprint-anchor"
  | "blueprint-genre"
  | "blueprint-premise"
  | "blueprint-characters"
  | "blueprint-world"
  | "blueprint-glossary"
  | "blueprint-plot"
  | "blueprint-chapters"
  | "character-seed"
  | "world-seed"
  | "plot-seed"
  | "glossary-seed"
  | "narrative-seed"
  | "chapter-plan";

export type BookStage = {
  key: BookStageKey;
  label: string;
  request: RunTaskPipelineInput;
};

type BlueprintStageSpec = {
  key: BookStageKey;
  label: string;
  stepKey: string;
  stepTitle: string;
};

export interface RunBookGenerationInput {
  projectRoot: string;
  ideaPrompt: string;
  chapterId?: string;
}

function normalizeChapterId(chapterId?: string): string | undefined {
  const trimmed = chapterId?.trim();
  return trimmed ? trimmed : undefined;
}

export type BookPipelineEvent =
  | {
    type: "stage-start";
    sessionId: string;
    stageKey: BookStageKey;
    stageLabel: string;
  }
  | {
    type: "stage-delta";
    sessionId: string;
    stageKey: BookStageKey;
    stageLabel: string;
    event: AiPipelineEvent;
  }
  | {
    type: "stage-done";
    sessionId: string;
    stageKey: BookStageKey;
    stageLabel: string;
    requestId: string;
  }
  | {
    type: "stage-error";
    sessionId: string;
    stageKey: BookStageKey;
    stageLabel: string;
    requestId?: string;
    errorCode?: string;
    message: string;
  };

const BLUEPRINT_STAGES: BlueprintStageSpec[] = [
  { key: "blueprint-anchor", label: "蓝图: 灵感定锚", stepKey: "step-01-anchor", stepTitle: "灵感定锚" },
  { key: "blueprint-genre", label: "蓝图: 类型策略", stepKey: "step-02-genre", stepTitle: "类型策略" },
  { key: "blueprint-premise", label: "蓝图: 故事母题", stepKey: "step-03-premise", stepTitle: "故事母题" },
  { key: "blueprint-characters", label: "蓝图: 角色工坊", stepKey: "step-04-characters", stepTitle: "角色工坊" },
  { key: "blueprint-world", label: "蓝图: 世界规则", stepKey: "step-05-world", stepTitle: "世界规则" },
  { key: "blueprint-glossary", label: "蓝图: 名词锁定", stepKey: "step-06-glossary", stepTitle: "名词锁定" },
  { key: "blueprint-plot", label: "蓝图: 剧情骨架", stepKey: "step-07-plot", stepTitle: "剧情骨架" },
  { key: "blueprint-chapters", label: "蓝图: 章节路线", stepKey: "step-08-chapters", stepTitle: "章节路线" },
];

function buildBlueprintSchemaHint(stepKey: string): string {
  switch (stepKey) {
    case "step-01-anchor":
      return `{
  "coreInspiration": "",
  "coreProposition": "",
  "coreEmotion": "",
  "targetReader": "",
  "sellingPoint": "",
  "readerExpectation": ""
}`;
    case "step-02-genre":
      return `{
  "mainGenre": "",
  "subGenre": "",
  "narrativePov": "",
  "styleKeywords": "",
  "rhythmType": "",
  "bannedStyle": ""
}`;
    case "step-03-premise":
      return `{
  "oneLineLogline": "",
  "threeParagraphSummary": "",
  "beginning": "",
  "middle": "",
  "climax": "",
  "ending": ""
}`;
    case "step-04-characters":
      return `{
  "protagonist": "",
  "antagonist": "",
  "supportingCharacters": "",
  "relationshipSummary": "",
  "growthArc": ""
}`;
    case "step-05-world":
      return `{
  "worldBackground": "",
  "rules": "",
  "locations": "",
  "organizations": "",
  "inviolableRules": ""
}`;
    case "step-06-glossary":
      return `{
  "personNames": "",
  "placeNames": "",
  "organizationNames": "",
  "terms": "",
  "aliases": "",
  "bannedTerms": ""
}`;
    case "step-07-plot":
      return `{
  "mainGoal": "",
  "stages": "",
  "keyConflicts": "",
  "twists": "",
  "climax": "",
  "ending": ""
}`;
    case "step-08-chapters":
      return `{
  "volumeStructure": "",
  "chapterList": "",
  "chapterGoals": "",
  "characters": "",
  "plotNodes": ""
}`;
    default:
      return "{}";
  }
}

function buildBlueprintInstruction(baseIdea: string, stepKey: string, stepTitle: string): string {
  return [
    `核心创意：${baseIdea}`,
    `当前步骤：${stepTitle}（${stepKey}）`,
    "必须只输出一个 JSON 对象，不要 Markdown，不要解释文本。",
    "JSON 必须完整包含以下字段，所有字段都必须是字符串且不能为空：",
    buildBlueprintSchemaHint(stepKey),
  ].join("\n");
}

function buildCharacterSeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 个核心角色，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "name": "",
  "roleType": "主角",
  "age": "",
  "gender": "",
  "identityText": "",
  "appearance": "",
  "motivation": "",
  "desire": "",
  "fear": "",
  "flaw": "",
  "arcStage": "",
  "notes": ""
}`,
  ].join("\n");
}

function buildWorldSeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 条世界规则，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "title": "",
  "category": "世界规则",
  "description": "",
  "constraintLevel": "normal",
  "examples": "",
  "contradictionPolicy": ""
}`,
  ].join("\n");
}

function buildPlotSeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 个主线剧情节点，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "title": "",
  "nodeType": "开端",
  "goal": "",
  "conflict": "",
  "emotionalCurve": "",
  "status": "规划中",
  "relatedCharacters": []
}`,
  ].join("\n");
}

function buildGlossarySeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 条核心名词，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "term": "",
  "termType": "术语",
  "aliases": [],
  "description": "",
  "locked": false,
  "banned": false
}`,
  ].join("\n");
}

function buildNarrativeSeedInstruction(baseIdea: string): string {
  return [
    `核心创意：${baseIdea}`,
    "请创建 1 条叙事义务，必须只输出 JSON 对象，不要 Markdown，不要解释。",
    "字段要求：",
    `{
  "obligationType": "明线伏笔",
  "description": "",
  "plantedChapterId": "",
  "expectedPayoffChapterId": "",
  "actualPayoffChapterId": "",
  "payoffStatus": "open",
  "severity": "medium",
  "relatedEntities": []
}`,
  ].join("\n");
}

export function buildBookStages(input: RunBookGenerationInput): BookStage[] {
  const base = input.ideaPrompt.trim();
  return BLUEPRINT_STAGES.map((stage) => ({
    key: stage.key,
    label: stage.label,
    request: {
      projectRoot: input.projectRoot,
      taskType: "blueprint.generate_step",
      userInstruction: buildBlueprintInstruction(base, stage.stepKey, stage.stepTitle),
      blueprintStepKey: stage.stepKey,
      blueprintStepTitle: stage.stepTitle,
      autoPersist: true,
      persistMode: "formal",
      automationTier: "supervised",
    },
  }));
}

export function buildPromotionStages(input: RunBookGenerationInput): BookStage[] {
  const base = input.ideaPrompt.trim();
  const chapterId = normalizeChapterId(input.chapterId);
  const stages: BookStage[] = [
    {
      key: "character-seed",
      label: "角色: 核心角色草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "character.create",
        userInstruction: buildCharacterSeedInstruction(base),
        autoPersist: true,
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
    {
      key: "world-seed",
      label: "设定: 世界规则草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "world.create_rule",
        userInstruction: buildWorldSeedInstruction(base),
        autoPersist: true,
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
    {
      key: "plot-seed",
      label: "剧情: 主线节点草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "plot.create_node",
        userInstruction: buildPlotSeedInstruction(base),
        autoPersist: true,
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
    {
      key: "glossary-seed",
      label: "名词: 核心术语草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "glossary.create_term",
        userInstruction: buildGlossarySeedInstruction(base),
        autoPersist: true,
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
    {
      key: "narrative-seed",
      label: "叙事: 义务草案",
      request: {
        projectRoot: input.projectRoot,
        taskType: "narrative.create_obligation",
        userInstruction: buildNarrativeSeedInstruction(base),
        autoPersist: true,
        persistMode: "formal",
        automationTier: "confirm",
      },
    },
  ];

  if (chapterId) {
    stages.push({
      key: "chapter-plan",
      label: "章节: 章节计划",
      request: {
        projectRoot: input.projectRoot,
        taskType: "chapter.plan",
        chapterId,
        userInstruction: base,
        autoPersist: true,
        persistMode: "formal",
        automationTier: "confirm",
      },
    });
  }
  return stages;
}

function createSessionId(): string {
  const randomPart = typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return `book-pipeline-${randomPart}`;
}

export async function* streamBookGenerationPipeline(
  input: RunBookGenerationInput,
  signal?: AbortSignal,
): AsyncGenerator<BookPipelineEvent> {
  // 问题5修复: 把离散 AI 任务编排成单一可追踪会话（分阶段 + 可中断）。
  const sessionId = createSessionId();
  const stages = buildBookStages(input);

  for (const stage of stages) {
    if (signal?.aborted) {
      yield {
        type: "stage-error",
        sessionId,
        stageKey: stage.key,
        stageLabel: stage.label,
        message: "全书编排任务已取消",
      };
      return;
    }

    yield {
      type: "stage-start",
      sessionId,
      stageKey: stage.key,
      stageLabel: stage.label,
    };

    let latestRequestId: string | undefined;
    try {
      for await (const event of streamTaskPipeline(stage.request, { timeoutMs: 180000 })) {
        latestRequestId = event.requestId;
        if (signal?.aborted) {
          if (latestRequestId) {
            await cancelTaskPipeline(latestRequestId, "book_pipeline_abort");
          }
          yield {
            type: "stage-error",
            sessionId,
            stageKey: stage.key,
            stageLabel: stage.label,
            requestId: latestRequestId,
            message: "全书编排任务已取消",
          };
          return;
        }
        if (event.type === "error") {
          yield {
            type: "stage-error",
            sessionId,
            stageKey: stage.key,
            stageLabel: stage.label,
            requestId: event.requestId,
            errorCode: event.errorCode,
            message: event.message ?? event.errorCode ?? "编排阶段执行失败",
          };
          return;
        }
        yield {
          type: "stage-delta",
          sessionId,
          stageKey: stage.key,
          stageLabel: stage.label,
          event,
        };
      }
      if (latestRequestId) {
        yield {
          type: "stage-done",
          sessionId,
          stageKey: stage.key,
          stageLabel: stage.label,
          requestId: latestRequestId,
        };
      }
    } catch (error) {
      yield {
        type: "stage-error",
        sessionId,
        stageKey: stage.key,
        stageLabel: stage.label,
        requestId: latestRequestId,
        message: error instanceof Error ? error.message : "编排阶段执行失败",
      };
      return;
    }
  }
}
