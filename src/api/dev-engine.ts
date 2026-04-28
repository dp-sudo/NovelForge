import { BLUEPRINT_STEP_KEYS } from "../domain/constants.js";
import type { BlueprintStepKey, BlueprintStepStatus, ChapterStatus } from "../domain/constants.js";
import type {
  BlueprintStep,
  ChapterInput,
  ChapterRecord,
  CharacterInput,
  CreateProjectInput,
  GlossaryTermInput,
  PlotNodeInput,
  ProjectJson,
  WorldRuleInput
} from "../domain/types.js";

const PROJECT_STORAGE_KEY = "nf_dev_project";
const DATA_PREFIX = "nf_dev_data_";

function projectKey(id: string): string {
  return `${DATA_PREFIX}${id}`;
}

function generateId(): string {
  return crypto.randomUUID();
}

function now(): string {
  return new Date().toISOString();
}

function loadProject(): ProjectJson | null {
  try {
    const raw = localStorage.getItem(PROJECT_STORAGE_KEY);
    return raw ? (JSON.parse(raw) as ProjectJson) : null;
  } catch {
    return null;
  }
}

function saveProject(p: ProjectJson): void {
  localStorage.setItem(PROJECT_STORAGE_KEY, JSON.stringify(p));
}

function clearProject(): void {
  const p = loadProject();
  if (p) localStorage.removeItem(projectKey(p.projectId));
  localStorage.removeItem(PROJECT_STORAGE_KEY);
}

function loadData<T>(projectId: string, collection: string): T[] {
  try {
    const raw = localStorage.getItem(`${projectKey(projectId)}_${collection}`);
    return raw ? (JSON.parse(raw) as T[]) : [];
  } catch {
    return [];
  }
}

function saveData<T>(projectId: string, collection: string, data: T[]): void {
  localStorage.setItem(`${projectKey(projectId)}_${collection}`, JSON.stringify(data));
}

const stepTitles: Record<string, string> = {
  "step-01-anchor": "灵感定锚",
  "step-02-genre": "类型策略",
  "step-03-premise": "故事母题",
  "step-04-characters": "角色工坊",
  "step-05-world": "世界规则",
  "step-06-glossary": "名词锁定",
  "step-07-plot": "剧情骨架",
  "step-08-chapters": "章节路线"
};

// ─── Project ───────────────────────────────────────────

export const DevProject = {
  async create(input: CreateProjectInput) {
    const id = generateId();
    const dirName = input.name.replace(/[<>:"/\\|?*]/g, "_").slice(0, 60);
    const project: ProjectJson = {
      schemaVersion: "1.0.0",
      appMinVersion: "0.1.0",
      projectId: id,
      name: input.name.trim(),
      author: input.author ?? "",
      genre: input.genre,
      targetWords: input.targetWords ?? 300000,
      createdAt: now(),
      updatedAt: now(),
      database: "database/project.sqlite",
      manuscriptRoot: "manuscript/chapters",
      settings: {
        defaultNarrativePov: "third_limited",
        language: "zh-CN",
        autosaveIntervalMs: 5000
      }
    };
    saveProject(project);
    return { projectRoot: `/dev/${dirName}`, project };
  },

  load(): ProjectJson | null {
    return loadProject();
  },

  clear(): void {
    clearProject();
  }
};

// ─── Blueprint ─────────────────────────────────────────

export const DevBlueprint = {
  listSteps(): BlueprintStep[] {
    const p = loadProject();
    if (!p) return [];
    const steps = loadData<BlueprintStep>(p.projectId, "blueprint");
    return BLUEPRINT_STEP_KEYS.map((key) => {
      const existing = steps.find((s) => s.stepKey === key);
      return (
        existing ?? {
          id: "",
          projectId: p.projectId,
          stepKey: key,
          title: stepTitles[key] ?? key,
          content: "",
          contentPath: `blueprint/${key}.md`,
          status: "not_started",
          aiGenerated: false,
          createdAt: "",
          updatedAt: ""
        }
      );
    });
  },

  saveStep(stepKey: BlueprintStepKey, content: string, aiGenerated = false): void {
    const p = loadProject();
    if (!p) return;
    const steps = loadData<BlueprintStep>(p.projectId, "blueprint");
    const idx = steps.findIndex((s) => s.stepKey === stepKey);
    const entry: BlueprintStep = {
      id: generateId(),
      projectId: p.projectId,
      stepKey,
      title: stepTitles[stepKey] ?? stepKey,
      content,
      contentPath: `blueprint/${stepKey}.md`,
      status: content.trim().length > 0 ? "in_progress" : "not_started",
      aiGenerated,
      createdAt: now(),
      updatedAt: now()
    };
    if (idx >= 0) steps[idx] = entry;
    else steps.push(entry);
    saveData(p.projectId, "blueprint", steps);
  },

  markCompleted(stepKey: BlueprintStepKey): void {
    const p = loadProject();
    if (!p) return;
    const steps = loadData<BlueprintStep>(p.projectId, "blueprint");
    const idx = steps.findIndex((s) => s.stepKey === stepKey);
    if (idx >= 0) {
      steps[idx].status = "completed";
      steps[idx].completedAt = now();
      steps[idx].updatedAt = now();
    }
    saveData(p.projectId, "blueprint", steps);
  },

  resetStep(stepKey: BlueprintStepKey): void {
    const p = loadProject();
    if (!p) return;
    const steps = loadData<BlueprintStep>(p.projectId, "blueprint");
    const idx = steps.findIndex((s) => s.stepKey === stepKey);
    if (idx >= 0) {
      steps[idx].content = "";
      steps[idx].status = "not_started" as const;
      steps[idx].aiGenerated = false;
      steps[idx].completedAt = undefined;
      steps[idx].updatedAt = now();
    }
    saveData(p.projectId, "blueprint", steps);
  }
};

// ─── Characters ────────────────────────────────────────

interface CharRow {
  id: string;
  projectId: string;
  name: string;
  aliases: string;
  roleType: string;
  age: string | null;
  gender: string | null;
  identityText: string | null;
  appearance: string | null;
  motivation: string | null;
  desire: string | null;
  fear: string | null;
  flaw: string | null;
  arcStage: string | null;
  lockedFields: string;
  notes: string | null;
  isDeleted: number;
  createdAt: string;
  updatedAt: string;
}

export const DevCharacter = {
  list() {
    const p = loadProject();
    if (!p) return [];
    const rows = loadData<CharRow>(p.projectId, "characters");
    return rows
      .filter((r) => !r.isDeleted)
      .map((r) => ({
        ...r,
        aliases: JSON.parse(r.aliases),
        locked_fields: JSON.parse(r.lockedFields)
      }));
  },

  create(input: CharacterInput): string {
    const p = loadProject();
    if (!p) throw new Error("No project open");
    const rows = loadData<CharRow>(p.projectId, "characters");
    const id = generateId();
    const row: CharRow = {
      id,
      projectId: p.projectId,
      name: input.name,
      aliases: JSON.stringify(input.aliases ?? []),
      roleType: input.roleType,
      age: input.age ?? null,
      gender: input.gender ?? null,
      identityText: input.identityText ?? null,
      appearance: input.appearance ?? null,
      motivation: input.motivation ?? null,
      desire: input.desire ?? null,
      fear: input.fear ?? null,
      flaw: input.flaw ?? null,
      arcStage: input.arcStage ?? null,
      lockedFields: JSON.stringify(input.lockedFields ?? []),
      notes: input.notes ?? null,
      isDeleted: 0,
      createdAt: now(),
      updatedAt: now()
    };
    rows.push(row);
    saveData(p.projectId, "characters", rows);
    return id;
  },

  update(id: string, input: Partial<CharacterInput>): void {
    const p = loadProject();
    if (!p) return;
    const rows = loadData<CharRow>(p.projectId, "characters");
    const idx = rows.findIndex((r) => r.id === id);
    if (idx < 0) return;
    const cur = rows[idx];
    rows[idx] = {
      ...cur,
      name: input.name ?? cur.name,
      aliases: JSON.stringify(input.aliases ?? JSON.parse(cur.aliases)),
      roleType: input.roleType ?? cur.roleType,
      age: input.age ?? cur.age,
      gender: input.gender ?? cur.gender,
      identityText: input.identityText ?? cur.identityText,
      appearance: input.appearance ?? cur.appearance,
      motivation: input.motivation ?? cur.motivation,
      desire: input.desire ?? cur.desire,
      fear: input.fear ?? cur.fear,
      flaw: input.flaw ?? cur.flaw,
      arcStage: input.arcStage ?? cur.arcStage,
      lockedFields: JSON.stringify(input.lockedFields ?? JSON.parse(cur.lockedFields)),
      notes: input.notes ?? cur.notes,
      updatedAt: now()
    };
    saveData(p.projectId, "characters", rows);
  },

  softDelete(id: string): void {
    const p = loadProject();
    if (!p) return;
    const rows = loadData<CharRow>(p.projectId, "characters");
    const idx = rows.findIndex((r) => r.id === id);
    if (idx >= 0) {
      rows[idx].isDeleted = 1;
      rows[idx].updatedAt = now();
    }
    saveData(p.projectId, "characters", rows);
  }
};

// ─── World ─────────────────────────────────────────────

interface WorldRow {
  id: string;
  projectId: string;
  title: string;
  category: string;
  description: string;
  constraintLevel: string;
  relatedEntities: string;
  examples: string | null;
  contradictionPolicy: string | null;
  isDeleted: number;
  createdAt: string;
  updatedAt: string;
}

export const DevWorld = {
  list() {
    const p = loadProject();
    if (!p) return [];
    const rows = loadData<WorldRow>(p.projectId, "world");
    return rows
      .filter((r) => !r.isDeleted)
      .map((r) => ({
        ...r,
        related_entities: JSON.parse(r.relatedEntities)
      }));
  },

  create(input: WorldRuleInput): string {
    const p = loadProject();
    if (!p) throw new Error("No project");
    const rows = loadData<WorldRow>(p.projectId, "world");
    const id = generateId();
    const row: WorldRow = {
      id,
      projectId: p.projectId,
      title: input.title,
      category: input.category,
      description: input.description,
      constraintLevel: input.constraintLevel,
      relatedEntities: JSON.stringify(input.relatedEntities ?? []),
      examples: input.examples ?? null,
      contradictionPolicy: input.contradictionPolicy ?? null,
      isDeleted: 0,
      createdAt: now(),
      updatedAt: now()
    };
    rows.push(row);
    saveData(p.projectId, "world", rows);
    return id;
  },

  softDelete(id: string): void {
    const p = loadProject();
    if (!p) return;
    const rows = loadData<WorldRow>(p.projectId, "world");
    const idx = rows.findIndex((r) => r.id === id);
    if (idx >= 0) {
      rows[idx].isDeleted = 1;
      rows[idx].updatedAt = now();
    }
    saveData(p.projectId, "world", rows);
  }
};

// ─── Glossary ──────────────────────────────────────────

interface GlossaryRow {
  id: string;
  projectId: string;
  term: string;
  termType: string;
  aliases: string;
  description: string | null;
  locked: number;
  banned: number;
  preferredUsage: string | null;
  createdAt: string;
  updatedAt: string;
}

export const DevGlossary = {
  list() {
    const p = loadProject();
    if (!p) return [];
    const rows = loadData<GlossaryRow>(p.projectId, "glossary");
    return rows.map((r) => ({
      id: r.id,
      project_id: r.projectId,
      term: r.term,
      term_type: r.termType,
      aliases: JSON.parse(r.aliases),
      description: r.description,
      locked: r.locked === 1,
      banned: r.banned === 1,
      preferred_usage: r.preferredUsage,
      created_at: r.createdAt,
      updated_at: r.updatedAt
    }));
  },

  create(input: GlossaryTermInput): string {
    const p = loadProject();
    if (!p) throw new Error("No project");
    const rows = loadData<GlossaryRow>(p.projectId, "glossary");
    const id = generateId();
    const row: GlossaryRow = {
      id,
      projectId: p.projectId,
      term: input.term,
      termType: input.termType,
      aliases: JSON.stringify(input.aliases ?? []),
      description: input.description ?? null,
      locked: input.locked ? 1 : 0,
      banned: input.banned ? 1 : 0,
      preferredUsage: input.preferredUsage ?? null,
      createdAt: now(),
      updatedAt: now()
    };
    rows.push(row);
    saveData(p.projectId, "glossary", rows);
    return id;
  }
};

// ─── Plot ──────────────────────────────────────────────

interface PlotRow {
  id: string;
  projectId: string;
  title: string;
  nodeType: string;
  sortOrder: number;
  goal: string | null;
  conflict: string | null;
  emotionalCurve: string | null;
  status: string;
  relatedCharacters: string;
  createdAt: string;
  updatedAt: string;
}

export const DevPlot = {
  list() {
    const p = loadProject();
    if (!p) return [];
    const rows = loadData<PlotRow>(p.projectId, "plot");
    return rows
      .sort((a, b) => a.sortOrder - b.sortOrder)
      .map((r) => ({
        id: r.id,
        project_id: r.projectId,
        title: r.title,
        node_type: r.nodeType,
        sort_order: r.sortOrder,
        goal: r.goal,
        conflict: r.conflict,
        emotional_curve: r.emotionalCurve,
        status: r.status,
        related_characters: JSON.parse(r.relatedCharacters),
        created_at: r.createdAt,
        updated_at: r.updatedAt
      }));
  },

  create(input: PlotNodeInput): string {
    const p = loadProject();
    if (!p) throw new Error("No project");
    const rows = loadData<PlotRow>(p.projectId, "plot");
    const id = generateId();
    const row: PlotRow = {
      id,
      projectId: p.projectId,
      title: input.title,
      nodeType: input.nodeType,
      sortOrder: input.sortOrder,
      goal: input.goal ?? null,
      conflict: input.conflict ?? null,
      emotionalCurve: input.emotionalCurve ?? null,
      status: input.status ?? "planning",
      relatedCharacters: JSON.stringify(input.relatedCharacters ?? []),
      createdAt: now(),
      updatedAt: now()
    };
    rows.push(row);
    saveData(p.projectId, "plot", rows);
    return id;
  },

  reorder(orderedIds: string[]): void {
    const p = loadProject();
    if (!p) return;
    const rows = loadData<PlotRow>(p.projectId, "plot");
    for (let i = 0; i < orderedIds.length; i++) {
      const idx = rows.findIndex((r) => r.id === orderedIds[i]);
      if (idx >= 0) {
        rows[idx].sortOrder = i + 1;
        rows[idx].updatedAt = now();
      }
    }
    saveData(p.projectId, "plot", rows);
  }
};

// ─── Chapters ──────────────────────────────────────────

interface ChapterRow {
  id: string;
  chapterIndex: number;
  title: string;
  summary: string;
  status: string;
  targetWords: number;
  currentWords: number;
  contentPath: string;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export const DevChapter = {
  list(): ChapterRecord[] {
    const p = loadProject();
    if (!p) return [];
    const rows = loadData<ChapterRow>(p.projectId, "chapters");
    return rows.map((r) => ({
      id: r.id,
      chapterIndex: r.chapterIndex,
      title: r.title,
      summary: r.summary,
      status: r.status as ChapterRecord["status"],
      targetWords: r.targetWords,
      currentWords: r.currentWords,
      contentPath: r.contentPath,
      version: r.version,
      updatedAt: r.updatedAt
    }));
  },

  create(input: ChapterInput): ChapterRecord {
    const p = loadProject();
    if (!p) throw new Error("No project");
    const rows = loadData<ChapterRow>(p.projectId, "chapters");
    const nextIndex = rows.length > 0 ? Math.max(...rows.map((r) => r.chapterIndex)) + 1 : 1;
    const id = generateId();
    const row: ChapterRow = {
      id,
      chapterIndex: nextIndex,
      title: input.title.trim(),
      summary: input.summary ?? "",
      status: input.status ?? "drafting",
      targetWords: input.targetWords ?? 0,
      currentWords: 0,
      contentPath: `manuscript/chapters/ch-${String(nextIndex).padStart(4, "0")}.md`,
      version: 1,
      createdAt: now(),
      updatedAt: now()
    };
    rows.push(row);
    saveData(p.projectId, "chapters", rows);
    return {
      id: row.id,
      chapterIndex: row.chapterIndex,
      title: row.title,
      summary: row.summary,
      status: row.status as ChapterRecord["status"],
      targetWords: row.targetWords,
      currentWords: row.currentWords,
      contentPath: row.contentPath,
      version: row.version,
      updatedAt: row.updatedAt
    };
  },

  delete(id: string): void {
    const p = loadProject();
    if (!p) return;
    const rows = loadData<ChapterRow>(p.projectId, "chapters");
    saveData(
      p.projectId,
      "chapters",
      rows.filter((r) => r.id !== id)
    );
  }
};

// ─── Context Assembly ─────────────────────────────────

export const DevContext = {
  forChapter(chapterId: string) {
    const p = loadProject();
    if (!p) return null;
    const chapters = DevChapter.list();
    const chapter = chapters.find((c) => c.id === chapterId);
    if (!chapter) return null;

    const chars = DevCharacter.list();
    const worlds = DevWorld.list();
    const plots = DevPlot.list();
    const glossary = DevGlossary.list();
    const steps = DevBlueprint.listSteps();

    const prevChapter = chapters
      .filter((c) => c.chapterIndex < chapter.chapterIndex)
      .sort((a, b) => b.chapterIndex - a.chapterIndex)[0];

    return {
      chapter: {
        id: chapter.id,
        title: chapter.title,
        summary: chapter.summary,
        status: chapter.status,
        targetWords: chapter.targetWords,
        currentWords: chapter.currentWords
      },
      characters: chars.map((c) => ({
        id: c.id,
        name: c.name,
        roleType: c.roleType,
        identityText: c.identityText,
        motivation: c.motivation,
        desire: c.desire,
        flaw: c.flaw
      })),
      worldRules: worlds.map((w) => ({
        id: w.id,
        title: w.title,
        category: w.category,
        description: w.description.slice(0, 200),
        constraintLevel: w.constraintLevel
      })),
      plotNodes: plots.map((p) => ({
        id: p.id,
        title: p.title,
        nodeType: p.node_type,
        goal: p.goal,
        sortOrder: p.sort_order
      })),
      glossary: glossary.map((g) => ({
        term: g.term,
        termType: g.term_type,
        locked: g.locked,
        banned: g.banned
      })),
      blueprint: steps.map((s) => ({
        stepKey: s.stepKey,
        content: s.content.slice(0, 500)
      })),
      previousChapterSummary: prevChapter ? prevChapter.summary : null
    };
  }
};

// ─── AI Mock ─────────────────────────────────────────

let aiRequestCounter = 0;

const MOCK_AI_DELAY_MS = 800;
const MOCK_AI_CHUNK_SIZE = 15;

const MOCK_RESPONSES: Record<string, string> = {
  generate_chapter_draft:
    "夜色深沉，窗外只有风声穿过街道。\n\n他坐在桌前已经三个小时，面前的纸上只有寥寥几行字。笔尖悬在纸面上方，墨水在笔尖凝成一滴，却始终落不下去。\n\n“需要一点勇气。”他对自己说。\n\n可勇气这东西，从来不是想要就能有的。它像一只胆小的猫，你招手它反而退得更远。\n\n窗外忽然传来一声响动。他抬起头，看见一道黑影从对面的屋顶掠过。\n\n心跳骤然加速。\n\n那不是猫。猫不会有那样大的轮廓，也不会带着那种刻意压低的呼吸声。\n\n他放下笔，站起身，走到窗边。\n\n夜色里什么都看不清，但他知道有人在看着他。\n\n或者说——有东西在看着他。",
  continue_chapter:
    "\n\n他拉上窗帘，后退两步，背抵着墙壁。\n\n呼吸在安静的房间里显得格外清晰。\n\n手机亮了，是一条未知号码的短信：\n\n“别回头。”\n\n他僵住了。不是因为短信的内容，而是因为他感觉到——身后有人。\n\n温热的气息拂过他的后颈。\n\n“我告诉过你，”一个声音在耳边响起，轻得像叹息，“别回头。”",
  rewrite_selection:
    "雨水顺着他的领口滑进脊背，冰凉的触感让他打了个寒颤。他没有停下脚步。街道尽头是一盏坏掉的路灯，明灭不定地闪烁着，把整条巷子切割成无数个破碎的瞬间。",
  deai_text:
    "天亮了。阳光从窗帘的缝隙里漏进来，在地板上画出一道狭长的光带。他坐在床沿，盯着自己的手看了很久。昨天的一切不是梦。桌上那张纸条还在，字迹没有被夜色抹去。",
  scan_consistency:
    "未发现明显的一致性问题。\n\n检查摘要：\n- 锁定名词使用正确 ✓\n- 禁用词未出现 ✓\n- 角色设定与角色卡一致 ✓\n- 世界规则未违反 ✓\n- 未发现明显 AI 腔 ✓"
};

export const DevAi = {
  async generatePreview(taskType: string, userInstruction: string) {
    aiRequestCounter++;
    const requestId = `ai-${Date.now()}-${aiRequestCounter}`;
    const mockResponse = MOCK_RESPONSES[taskType] ?? MOCK_RESPONSES.generate_chapter_draft;

    await new Promise((r) => setTimeout(r, MOCK_AI_DELAY_MS));

    return {
      requestId,
      preview: mockResponse,
      usedContext: ["project.json", "当前章节信息", "关联角色", "关联主线"],
      risks: ["请检查生成内容是否符合预期", "建议手动调整细节"]
    };
  },

  async *streamPreview(taskType: string, _userInstruction: string) {
    aiRequestCounter++;
    const requestId = `ai-${Date.now()}-${aiRequestCounter}`;
    const mockResponse = MOCK_RESPONSES[taskType] ?? MOCK_RESPONSES.generate_chapter_draft;

    yield { requestId, type: "start" as const };

    let pos = 0;
    while (pos < mockResponse.length) {
      await new Promise((r) => setTimeout(r, 50));
      const chunk = mockResponse.slice(pos, pos + MOCK_AI_CHUNK_SIZE);
      pos += MOCK_AI_CHUNK_SIZE;
      yield { requestId, type: "delta" as const, delta: chunk };
    }

    yield { requestId, type: "done" as const };
  }
};

// ─── Consistency Mock ────────────────────────────────

export const DevConsistency = {
  scanChapter(chapterId: string) {
    const p = loadProject();
    if (!p) return [];
    const chapters = DevChapter.list();
    const chapter = chapters.find((c) => c.id === chapterId);
    if (!chapter) return [];

    const glossary = DevGlossary.list();
    const bannedTerms = glossary.filter((g) => g.banned);
    const chars = DevCharacter.list();

    const issues: Array<{
      id: string;
      issueType: string;
      severity: string;
      chapterId: string;
      sourceText: string;
      explanation: string;
      suggestedFix: string;
      status: string;
    }> = [];

    for (const bt of bannedTerms) {
      issues.push({
        id: `issue-${generateId()}`,
        issueType: "glossary",
        severity: "high",
        chapterId: chapter.id,
        sourceText: bt.term,
        explanation: `禁用词 "${bt.term}" 出现在章节中，建议删除或替换`,
        suggestedFix: `删除或替换 "${bt.term}"`,
        status: "open"
      });
    }

    if (chars.length === 0) {
      issues.push({
        id: `issue-${generateId()}`,
        issueType: "character",
        severity: "low",
        chapterId: chapter.id,
        sourceText: "无出场角色",
        explanation: "当前章节未关联任何角色",
        suggestedFix: "在章节设置中添加出场角色",
        status: "open"
      });
    }

    if (issues.length === 0) {
      issues.push({
        id: `issue-${generateId()}`,
        issueType: "prose_style",
        severity: "info",
        chapterId: chapter.id,
        sourceText: "未发现问题",
        explanation: "基础检查未发现明显问题",
        suggestedFix: "",
        status: "open"
      });
    }

    return issues;
  },

  scanAll() {
    const p = loadProject();
    if (!p) return [];
    const chapters = DevChapter.list();
    const allIssues: Array<{
      id: string;
      issueType: string;
      severity: string;
      chapterId: string;
      sourceText: string;
      explanation: string;
      suggestedFix: string;
      status: string;
    }> = [];

    for (const ch of chapters) {
      const issues = this.scanChapter(ch.id);
      allIssues.push(...issues);
    }

    return allIssues;
  },

  updateIssueStatus(_issueId: string, _status: string): void {
    // In-memory for dev mode
  }
};

// ─── Export Mock ─────────────────────────────────────

export const DevExport = {
  async exportChapter(
    _projectRoot: string,
    _chapterId: string,
    format: "txt" | "md" | "docx" | "pdf" | "epub",
    outputPath: string,
    _options?: unknown
  ) {
    const p = loadProject();
    const chapters = DevChapter.list();
    const ch = chapters[0];
    if (!ch) throw new Error("No chapters to export");
    const content = format === "md"
      ? `# ${ch.title}\n\n${ch.summary}`
      : `${ch.title}\n\n${ch.summary}`;
    if (format === "txt" || format === "md") {
      return { outputPath, content };
    }
    return { outputPath, content: `${format.toUpperCase()} 导出已模拟完成` };
  },

  async exportBook(
    _projectRoot: string,
    format: "txt" | "md" | "docx" | "pdf" | "epub",
    outputPath: string,
    _options?: unknown
  ) {
    const chapters = DevChapter.list();
    const parts = chapters.map((ch, i) => {
      const body = format === "md"
        ? `# ${ch.title}\n\n${ch.summary}`
        : `${ch.title}\n\n${ch.summary}`;
      return `\n\n---\n\n${body}`;
    });
    if (format === "txt" || format === "md") {
      return { outputPath, content: parts.join("") };
    }
    return { outputPath, content: `${format.toUpperCase()} 导出已模拟完成` };
  }
};

// ─── Stats ─────────────────────────────────────────────

export const DevStats = {
  get() {
    const p = loadProject();
    if (!p) return null;
    const chars = DevCharacter.list();
    const worlds = DevWorld.list();
    const plots = DevPlot.list();
    const chapters = DevChapter.list();
    const steps = DevBlueprint.listSteps();
    const completedSteps = steps.filter((s) => s.status === "completed").length;
    const issues = DevConsistency.scanAll();
    return {
      totalWords: chapters.reduce((s, c) => s + c.currentWords, 0),
      chapterCount: chapters.length,
      characterCount: chars.length,
      worldRuleCount: worlds.length,
      plotNodeCount: plots.length,
      openIssueCount: issues.filter((i) => i.status === "open").length,
      blueprintProgress: Math.round((completedSteps / steps.length) * 100)
    };
  }
};

// ─── Settings ────────────────────────────────────────

const SETTINGS_KEY = "nf_dev_settings";

interface StoredSettings {
  provider: {
    providerName: string;
    baseUrl: string;
    apiKey: string;
    model: string;
    temperature: number;
    maxTokens: number;
    stream: boolean;
  };
  editor: {
    fontSize: number;
    lineHeight: number;
    autosaveInterval: number;
    narrativePov: string;
  };
}

function defaultSettings(): StoredSettings {
  return {
    provider: {
      providerName: "",
      baseUrl: "",
      apiKey: "",
      model: "",
      temperature: 0.7,
      maxTokens: 4096,
      stream: true
    },
    editor: {
      fontSize: 16,
      lineHeight: 1.75,
      autosaveInterval: 5,
      narrativePov: "third_limited"
    }
  };
}

function loadSettings(): StoredSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<StoredSettings>;
      return { ...defaultSettings(), ...parsed };
    }
  } catch { /* ignore */ }
  return defaultSettings();
}

function saveSettings(s: StoredSettings): void {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
}

export const DevSettings = {
  loadProvider() {
    const s = loadSettings();
    const { apiKey, ...rest } = s.provider;
    return {
      ...rest,
      apiKeyMasked: apiKey ? `${apiKey.slice(0, 4)}••••${apiKey.slice(-4)}` : ""
    };
  },

  saveProvider(input: {
    providerName: string;
    baseUrl: string;
    model: string;
    temperature: number;
    maxTokens: number;
    stream: boolean;
    apiKey?: string;
  }) {
    const s = loadSettings();
    s.provider = {
      ...s.provider,
      providerName: input.providerName,
      baseUrl: input.baseUrl,
      model: input.model,
      temperature: input.temperature,
      maxTokens: input.maxTokens,
      stream: input.stream
    };
    if (input.apiKey) s.provider.apiKey = input.apiKey;
    saveSettings(s);
  },

  testConnection() {
    const s = loadSettings();
    if (!s.provider.baseUrl || !s.provider.model) {
      return { success: false, message: "请先填写 Base URL 和 Model" };
    }
    return { success: true, message: `连接成功！模型 ${s.provider.model} 可用。` };
  },

  loadEditor() {
    return loadSettings().editor;
  },

  saveEditor(input: {
    fontSize: number;
    lineHeight: number;
    autosaveInterval: number;
    narrativePov: string;
  }) {
    const s = loadSettings();
    s.editor = { ...s.editor, ...input };
    saveSettings(s);
  }
};
