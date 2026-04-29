import type {
  BlueprintStepKey,
  BlueprintStepStatus,
  ChapterStatus,
  IssueSeverity,
  IssueStatus
} from "./constants.js";

export interface CreateProjectInput {
  name: string;
  author?: string;
  genre: string;
  targetWords?: number;
  saveDirectory: string;
}

export interface ProjectJson {
  schemaVersion: string;
  appMinVersion: string;
  projectId: string;
  name: string;
  author: string;
  genre: string;
  targetWords: number;
  createdAt: string;
  updatedAt: string;
  database: string;
  manuscriptRoot: string;
  settings: {
    defaultNarrativePov: string;
    language: string;
    autosaveIntervalMs: number;
  };
}

export interface BlueprintStep {
  id: string;
  projectId: string;
  stepKey: BlueprintStepKey;
  title: string;
  content: string;
  contentPath: string;
  status: BlueprintStepStatus;
  aiGenerated: boolean;
  completedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CharacterInput {
  name: string;
  aliases?: string[];
  roleType: "主角" | "反派" | "配角" | "路人" | "组织角色";
  age?: string;
  gender?: string;
  identityText?: string;
  appearance?: string;
  motivation?: string;
  desire?: string;
  fear?: string;
  flaw?: string;
  arcStage?: string;
  lockedFields?: string[];
  notes?: string;
}

export interface WorldRuleInput {
  title: string;
  category: "世界规则" | "地点" | "组织" | "道具" | "能力" | "历史事件" | "术语";
  description: string;
  constraintLevel: "weak" | "normal" | "strong" | "absolute";
  relatedEntities?: string[];
  examples?: string;
  contradictionPolicy?: string;
}

export interface GlossaryTermInput {
  term: string;
  termType: "人名" | "地名" | "组织名" | "术语" | "别名" | "禁用词";
  aliases?: string[];
  description?: string;
  locked?: boolean;
  banned?: boolean;
  preferredUsage?: string;
}

export interface PlotNodeInput {
  title: string;
  nodeType: "开端" | "转折" | "冲突" | "失败" | "胜利" | "高潮" | "结局" | "支线";
  sortOrder: number;
  goal?: string;
  conflict?: string;
  emotionalCurve?: string;
  status?: "未使用" | "规划中" | "已写入" | "需调整";
  relatedCharacters?: string[];
}

export interface ChapterInput {
  title: string;
  summary?: string;
  targetWords?: number;
  status?: ChapterStatus;
}

export interface ChapterRecord {
  id: string;
  chapterIndex: number;
  title: string;
  summary: string;
  status: ChapterStatus;
  targetWords: number;
  currentWords: number;
  contentPath: string;
  volumeId?: string | null;
  version: number;
  updatedAt: string;
}

export interface ProviderConfigInput {
  providerName: string;
  baseUrl: string;
  model: string;
  temperature: number;
  maxTokens: number;
  stream: boolean;
  apiKey?: string;
}

export interface AiPreviewRequest {
  taskType:
    | "generate_blueprint_step"
    | "generate_chapter_draft"
    | "continue_chapter"
    | "rewrite_selection"
    | "deai_text"
    | "scan_consistency";
  userInstruction: string;
  chapterId?: string;
  selectedText?: string;
}

export interface AiPreviewResponse {
  requestId: string;
  preview: string;
  usedContext: string[];
  risks: string[];
}

export interface ConsistencyIssue {
  id: string;
  issueType: "glossary" | "character" | "world_rule" | "timeline" | "prose_style";
  severity: IssueSeverity;
  chapterId: string;
  sourceText: string;
  sourceStart?: number;
  sourceEnd?: number;
  relatedAssetType?: string;
  relatedAssetId?: string;
  explanation: string;
  suggestedFix?: string;
  status: IssueStatus;
}

export interface ExportOptions {
  includeChapterTitle?: boolean;
  includeChapterSummary?: boolean;
  separateByVolume?: boolean;
  includeWorldSettings?: boolean;
}

// ── Blueprint structured types (PRD §8.3) ──

export interface BlueprintAnchorData {
  coreInspiration: string;
  coreProposition: string;
  coreEmotion: string;
  targetReader: string;
  sellingPoint: string;
  readerExpectation: string;
}

export interface BlueprintGenreData {
  mainGenre: string;
  subGenre: string;
  narrativePov: string;
  styleKeywords: string;
  rhythmType: string;
  bannedStyle: string;
}

export interface BlueprintPremiseData {
  oneLineLogline: string;
  threeParagraphSummary: string;
  beginning: string;
  middle: string;
  climax: string;
  ending: string;
}

export interface BlueprintCharactersData {
  protagonist: string;
  antagonist: string;
  supportingCharacters: string;
  relationshipSummary: string;
  growthArc: string;
}

export interface BlueprintWorldData {
  worldBackground: string;
  rules: string;
  locations: string;
  organizations: string;
  inviolableRules: string;
}

export interface BlueprintGlossaryData {
  personNames: string;
  placeNames: string;
  organizationNames: string;
  terms: string;
  aliases: string;
  bannedTerms: string;
}

export interface BlueprintPlotData {
  mainGoal: string;
  stages: string;
  keyConflicts: string;
  twists: string;
  climax: string;
  ending: string;
}

export interface BlueprintChaptersData {
  volumeStructure: string;
  chapterList: string;
  chapterGoals: string;
  characters: string;
  plotNodes: string;
}

export type BlueprintStepData =
  | BlueprintAnchorData
  | BlueprintGenreData
  | BlueprintPremiseData
  | BlueprintCharactersData
  | BlueprintWorldData
  | BlueprintGlossaryData
  | BlueprintPlotData
  | BlueprintChaptersData;

export const BLUEPRINT_DEFAULTS: Record<string, BlueprintStepData> = {
  "step-01-anchor": { coreInspiration: "", coreProposition: "", coreEmotion: "", targetReader: "", sellingPoint: "", readerExpectation: "" },
  "step-02-genre": { mainGenre: "", subGenre: "", narrativePov: "third_limited", styleKeywords: "", rhythmType: "", bannedStyle: "" },
  "step-03-premise": { oneLineLogline: "", threeParagraphSummary: "", beginning: "", middle: "", climax: "", ending: "" },
  "step-04-characters": { protagonist: "", antagonist: "", supportingCharacters: "", relationshipSummary: "", growthArc: "" },
  "step-05-world": { worldBackground: "", rules: "", locations: "", organizations: "", inviolableRules: "" },
  "step-06-glossary": { personNames: "", placeNames: "", organizationNames: "", terms: "", aliases: "", bannedTerms: "" },
  "step-07-plot": { mainGoal: "", stages: "", keyConflicts: "", twists: "", climax: "", ending: "" },
  "step-08-chapters": { volumeStructure: "", chapterList: "", chapterGoals: "", characters: "", plotNodes: "" },
};

const BLUEPRINT_FIELD_ALIASES: Record<string, Record<string, string[]>> = {
  "step-01-anchor": {
    coreInspiration: ["inspiration", "core_inspiration", "核心灵感", "灵感来源"],
    coreProposition: ["proposition", "core_proposition", "核心命题", "主题命题"],
    coreEmotion: ["emotion", "core_emotion", "核心情绪", "情绪基调"],
    targetReader: ["reader", "target_reader", "目标读者"],
    sellingPoint: ["selling_point", "商业卖点", "卖点"],
    readerExpectation: ["reader_expectation", "读者期待", "预期"],
  },
  "step-02-genre": {
    mainGenre: ["genre", "main_genre", "主类型", "主题材"],
    subGenre: ["sub_genre", "子类型", "子题材"],
    narrativePov: ["pov", "narrative_pov", "叙事视角", "视角"],
    styleKeywords: ["style", "style_keywords", "文风关键词", "风格关键词"],
    rhythmType: ["rhythm", "rhythm_type", "节奏类型", "节奏"],
    bannedStyle: ["banned_style", "禁用风格", "避免风格"],
  },
  "step-03-premise": {
    oneLineLogline: ["logline", "one_line_logline", "一句话梗概"],
    threeParagraphSummary: ["summary", "three_paragraph_summary", "三段式梗概"],
    beginning: ["start", "opening", "开端"],
    middle: ["mid", "中段"],
    climax: ["高潮"],
    ending: ["结局", "ending_direction"],
  },
  "step-04-characters": {
    protagonist: ["mainCharacter", "main_character", "主角"],
    antagonist: ["villain", "反派"],
    supportingCharacters: ["supporting_characters", "配角", "关键配角"],
    relationshipSummary: ["relationship_summary", "角色关系", "角色关系摘要"],
    growthArc: ["arc", "growth_arc", "成长弧线", "角色成长"],
  },
  "step-05-world": {
    worldBackground: ["background", "world_background", "世界背景"],
    rules: ["rule", "world_rules", "规则", "规则体系"],
    locations: ["places", "地点"],
    organizations: ["factions", "组织", "势力"],
    inviolableRules: ["hard_rules", "inviolable_rules", "不可违反规则", "铁律"],
  },
  "step-06-glossary": {
    personNames: ["person_names", "characters", "人名"],
    placeNames: ["place_names", "地名"],
    organizationNames: ["organization_names", "组织名"],
    terms: ["术语", "glossary_terms"],
    aliases: ["别名", "alias_map"],
    bannedTerms: ["banned_terms", "禁用名词", "禁词"],
  },
  "step-07-plot": {
    mainGoal: ["goal", "main_goal", "主线目标"],
    stages: ["stage_nodes", "阶段节点"],
    keyConflicts: ["conflicts", "key_conflicts", "关键冲突"],
    twists: ["reversals", "反转"],
    climax: ["高潮"],
    ending: ["结局"],
  },
  "step-08-chapters": {
    volumeStructure: ["volume_structure", "卷结构"],
    chapterList: ["chapters", "chapter_list", "章节列表"],
    chapterGoals: ["chapter_goals", "章节目标"],
    characters: ["cast", "出场人物"],
    plotNodes: ["plot_nodes", "关联主线节点"],
  },
};

function normalizeKey(key: string): string {
  return key.toLowerCase().replace(/[\s_\-]/g, "");
}

function findValueByAlias(
  obj: Record<string, unknown>,
  aliases: string[],
): unknown {
  for (const alias of aliases) {
    if (alias in obj) return obj[alias];
    const normalizedAlias = normalizeKey(alias);
    const matchedKey = Object.keys(obj).find((candidate) => normalizeKey(candidate) === normalizedAlias);
    if (matchedKey) {
      return obj[matchedKey];
    }
  }
  return undefined;
}

function valueToText(value: unknown): string | undefined {
  if (typeof value === "string") {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : undefined;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) {
    const items = value
      .map((item) => valueToText(item))
      .filter((item): item is string => Boolean(item));
    return items.length > 0 ? items.join("；") : undefined;
  }
  return undefined;
}

export function parseBlueprintContent(stepKey: string, content: string): Record<string, string> {
  const defaults = { ...BLUEPRINT_DEFAULTS[stepKey] } as Record<string, string>;
  if (!content.trim()) return defaults;
  try {
    const parsed = JSON.parse(content);
    if (typeof parsed !== "object" || parsed === null) return defaults;

    const sourceObjects: Array<Record<string, unknown>> = [];
    if (!Array.isArray(parsed)) {
      sourceObjects.push(parsed as Record<string, unknown>);
      for (const key of ["blueprintStep", "content", "fields", "data", "payload", "result"]) {
        const nested = (parsed as Record<string, unknown>)[key];
        if (nested && typeof nested === "object" && !Array.isArray(nested)) {
          sourceObjects.push(nested as Record<string, unknown>);
        }
      }
    }

    const aliasMap = BLUEPRINT_FIELD_ALIASES[stepKey] ?? {};
    const merged: Record<string, string> = { ...defaults };
    let filledCount = 0;
    for (const key of Object.keys(defaults)) {
      const aliases = [key, ...(aliasMap[key] ?? [])];
      let assigned: string | undefined;
      for (const candidate of sourceObjects) {
        const rawValue = findValueByAlias(candidate, aliases);
        assigned = valueToText(rawValue);
        if (assigned) break;
      }
      if (assigned) {
        merged[key] = assigned;
        filledCount += 1;
      }
    }

    if (filledCount === 0) {
      const parsedObj = parsed as Record<string, unknown>;
      const suggestion = valueToText(parsedObj.suggestion);
      if (suggestion) {
        const firstKey = Object.keys(defaults)[0];
        merged[firstKey] = suggestion;
      }
    }

    return merged;
  } catch {
    // Not JSON — backward compat: put entire text into the first field
    const firstKey = Object.keys(defaults)[0];
    return { ...defaults, [firstKey]: content };
  }
}

export function serializeBlueprintContent(data: Record<string, string>): string {
  return JSON.stringify(data, null, 2);
}
