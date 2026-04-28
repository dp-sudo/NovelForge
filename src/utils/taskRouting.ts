export const TASK_ROUTE_OPTIONS = [
  { value: "chapter.draft", label: "章节草稿" },
  { value: "chapter.continue", label: "章节续写" },
  { value: "chapter.rewrite", label: "局部改写" },
  { value: "chapter.plan", label: "章节计划" },
  { value: "prose.naturalize", label: "去 AI 味" },
  { value: "character.create", label: "角色生成" },
  { value: "world.create_rule", label: "世界观生成" },
  { value: "consistency.scan", label: "一致性检查" },
  { value: "blueprint.generate_step", label: "蓝图生成" },
  { value: "plot.create_node", label: "剧情生成" },
  { value: "custom", label: "自定义任务（兜底）" },
] as const;

const TASK_TYPE_ALIAS_MAP: Record<string, string> = {
  chapter_draft: "chapter.draft",
  generate_chapter_draft: "chapter.draft",
  draft: "chapter.draft",
  chapter_continue: "chapter.continue",
  continue_chapter: "chapter.continue",
  continue_draft: "chapter.continue",
  chapter_rewrite: "chapter.rewrite",
  rewrite_selection: "chapter.rewrite",
  chapter_plan: "chapter.plan",
  plan_chapter: "chapter.plan",
  prose_naturalize: "prose.naturalize",
  deai_text: "prose.naturalize",
  character_create: "character.create",
  "world.generate": "world.create_rule",
  world_create_rule: "world.create_rule",
  "plot.generate": "plot.create_node",
  plot_create_node: "plot.create_node",
  scan_consistency: "consistency.scan",
  consistency_scan: "consistency.scan",
  generate_blueprint_step: "blueprint.generate_step",
  blueprint_generate: "blueprint.generate_step",
};

export const TASK_TYPE_LABELS: Record<string, string> = {
  "chapter.draft": "生成草稿",
  "chapter.continue": "续写",
  "chapter.rewrite": "改写",
  "chapter.plan": "章节计划",
  "prose.naturalize": "去 AI 味",
  "character.create": "创建角色卡",
  "world.create_rule": "创建世界规则",
  "plot.create_node": "创建剧情节点",
  "consistency.scan": "一致性检查",
  "blueprint.generate_step": "生成蓝图步骤",
  custom: "自定义",
};

export const DIFF_TASK_TYPES = new Set(["chapter.rewrite", "prose.naturalize"]);

export function canonicalTaskType(taskType: string): string {
  const normalized = taskType.trim();
  return TASK_TYPE_ALIAS_MAP[normalized] || normalized;
}
