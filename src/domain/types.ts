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
