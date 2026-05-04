export const TASK_TYPE_LABELS: Record<string, string> = {
  "chapter.draft": "生成草稿",
  "chapter.continue": "续写",
  "chapter.rewrite": "改写",
  "chapter.plan": "章节计划",
  "prose.naturalize": "去 AI 味",
  "character.create": "创建角色卡",
  "world.create_rule": "创建世界规则",
  "plot.create_node": "创建剧情节点",
  "glossary.create_term": "创建名词",
  "narrative.create_obligation": "创建叙事义务",
  "timeline.review": "时间线审阅",
  "relationship.review": "关系审阅",
  "dashboard.review": "仪表盘审阅",
  "export.review": "导出审阅",
  "consistency.scan": "一致性检查",
  "blueprint.generate_step": "生成蓝图步骤",
  custom: "自定义任务",
};

export const TASK_ROUTE_OPTIONS = [
  "chapter.draft",
  "chapter.continue",
  "chapter.rewrite",
  "chapter.plan",
  "prose.naturalize",
  "character.create",
  "world.create_rule",
  "consistency.scan",
  "blueprint.generate_step",
  "plot.create_node",
  "glossary.create_term",
  "narrative.create_obligation",
  "timeline.review",
  "relationship.review",
  "dashboard.review",
  "export.review",
  "custom"
].map(value => ({ value, label: TASK_TYPE_LABELS[value] || value }));


export interface EditorAiAction {
  taskType: string;
  label: string;
  category: "writing" | "character" | "world" | "plot" | "review";
}
export type EditorAiCategory = EditorAiAction["category"];

// WP-5: 编辑器固定 9 按钮任务清单（canonical）
export const EDITOR_AI_ACTIONS: EditorAiAction[] = [
  { taskType: "chapter.continue", label: "续写章节", category: "writing" },
  { taskType: "chapter.draft", label: "生成章节草稿", category: "writing" },
  { taskType: "chapter.plan", label: "生成章节计划", category: "writing" },
  { taskType: "chapter.rewrite", label: "改写选区", category: "writing" },
  { taskType: "prose.naturalize", label: "去 AI 味", category: "writing" },
  { taskType: "character.create", label: "创建角色卡", category: "character" },
  { taskType: "world.create_rule", label: "创建世界规则", category: "world" },
  { taskType: "plot.create_node", label: "创建剧情节点", category: "plot" },
  { taskType: "consistency.scan", label: "一致性扫描", category: "review" },
];

export const EDITOR_AI_TASK_TYPES = EDITOR_AI_ACTIONS.map((action) => action.taskType);
export const EDITOR_AI_TASK_SET = new Set(EDITOR_AI_TASK_TYPES);


export const DIFF_TASK_TYPES = new Set(["chapter.rewrite", "prose.naturalize"]);

const TASK_REQUIREMENT_MAP: Record<string, {
  requiresChapterId: boolean;
  requiresSelectedText: boolean;
  requiresUserInstruction: boolean;
  requiresChapterContent: boolean;
}> = {
  "chapter.draft": {
    requiresChapterId: true,
    requiresSelectedText: false,
    requiresUserInstruction: false,
    requiresChapterContent: false
  },
  "chapter.continue": {
    requiresChapterId: true,
    requiresSelectedText: false,
    requiresUserInstruction: false,
    requiresChapterContent: false
  },
  "chapter.plan": {
    requiresChapterId: false,
    requiresSelectedText: false,
    requiresUserInstruction: false,
    requiresChapterContent: false
  },
  "chapter.rewrite": {
    requiresChapterId: true,
    requiresSelectedText: true,
    requiresUserInstruction: false,
    requiresChapterContent: false
  },
  "prose.naturalize": {
    requiresChapterId: true,
    requiresSelectedText: true,
    requiresUserInstruction: false,
    requiresChapterContent: false
  },
  "character.create": {
    requiresChapterId: false,
    requiresSelectedText: false,
    requiresUserInstruction: true,
    requiresChapterContent: false
  },
  "world.create_rule": {
    requiresChapterId: false,
    requiresSelectedText: false,
    requiresUserInstruction: true,
    requiresChapterContent: false
  },
  "plot.create_node": {
    requiresChapterId: false,
    requiresSelectedText: false,
    requiresUserInstruction: true,
    requiresChapterContent: false
  },
  "consistency.scan": {
    requiresChapterId: true,
    requiresSelectedText: false,
    requiresUserInstruction: false,
    requiresChapterContent: true
  },
  "glossary.create_term": {
    requiresChapterId: false,
    requiresSelectedText: false,
    requiresUserInstruction: true,
    requiresChapterContent: false
  },
  "narrative.create_obligation": {
    requiresChapterId: false,
    requiresSelectedText: false,
    requiresUserInstruction: true,
    requiresChapterContent: false
  },
};

export function getTaskRequirements(taskType: string) {
  const canonical = taskType.trim();
  return (
    TASK_REQUIREMENT_MAP[canonical] || {
      requiresChapterId: false,
      requiresSelectedText: false,
      requiresUserInstruction: false,
      requiresChapterContent: false
    }
  );
}

export function isEditorAiTask(taskType: string): boolean {
  return EDITOR_AI_TASK_SET.has(taskType.trim());
}
