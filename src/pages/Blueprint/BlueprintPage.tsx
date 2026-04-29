import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Badge } from "../../components/ui/Badge.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { Select } from "../../components/forms/Select.js";
import { listBlueprintSteps, saveBlueprintStep, markBlueprintCompleted, resetBlueprintStep, generateBlueprintSuggestion } from "../../api/blueprintApi.js";
import { streamBookGenerationPipeline } from "../../api/bookPipelineApi.js";
import { useProjectStore } from "../../stores/projectStore.js";
import { parseBlueprintContent, serializeBlueprintContent } from "../../domain/types.js";
import type { BlueprintStepKey } from "../../domain/constants.js";

interface StepDef {
  key: BlueprintStepKey;
  label: string;
  desc: string;
}

const STEPS: StepDef[] = [
  { key: "step-01-anchor", label: "灵感定锚", desc: "捕获作品最初的火花" },
  { key: "step-02-genre", label: "类型策略", desc: "明确作品类型与叙事风格" },
  { key: "step-03-premise", label: "故事母题", desc: "搭建故事的核心骨架" },
  { key: "step-04-characters", label: "角色工坊", desc: "塑造有血有肉的角色" },
  { key: "step-05-world", label: "世界规则", desc: "构建可信的世界体系" },
  { key: "step-06-glossary", label: "名词锁定", desc: "锁定关键名词与禁用词" },
  { key: "step-07-plot", label: "剧情骨架", desc: "规划冲突、转折与高潮" },
  { key: "step-08-chapters", label: "章节路线", desc: "拆分章节、目标与出场角色" }
];

const GENRE_OPTIONS = [
  { value: "玄幻", label: "玄幻" },
  { value: "都市", label: "都市" },
  { value: "科幻", label: "科幻" },
  { value: "悬疑", label: "悬疑" },
  { value: "言情", label: "言情" },
  { value: "历史", label: "历史" },
  { value: "奇幻", label: "奇幻" },
  { value: "轻小说", label: "轻小说" },
  { value: "剧本", label: "剧本" },
  { value: "其他", label: "其他" },
];

const POV_OPTIONS = [
  { value: "first", label: "第一人称" },
  { value: "third_limited", label: "第三人称限制视角" },
  { value: "third_omniscient", label: "第三人称全知视角" },
];

const RHYTHM_OPTIONS = [
  { value: "平稳", label: "平稳" },
  { value: "张弛", label: "张弛有度" },
  { value: "紧凑", label: "紧凑" },
  { value: "极快", label: "极快" },
];

type StepStatus = "not_started" | "in_progress" | "completed";

// ── Form field helpers ──

function TextField({ label, value, onChange, placeholder, helperText }: {
  label: string; value: string; onChange: (v: string) => void; placeholder?: string; helperText?: string;
}) {
  return (
    <Textarea
      label={label}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      helperText={helperText}
    />
  );
}

// ── Step 1: 灵感定锚 ──

function AnchorForm({ data, onChange }: { data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  function set(k: string) { return (v: string) => onChange({ ...data, [k]: v }); }
  return (
    <div className="grid grid-cols-2 gap-4">
      <div className="col-span-2">
        <TextField label="核心灵感" value={data.coreInspiration} onChange={set("coreInspiration")}
          placeholder="用一个句子描述作品最核心的灵感来源" helperText="你最初被什么打动？是什么让你想写这个故事？" />
      </div>
      <TextField label="核心命题" value={data.coreProposition} onChange={set("coreProposition")}
        placeholder="故事想探讨的核心主题是什么？" helperText="例如：自由与责任的冲突、正义的代价" />
      <TextField label="核心情绪" value={data.coreEmotion} onChange={set("coreEmotion")}
        placeholder="作品整体情绪基调" helperText="例如：压抑中带希望、热血激昂、冷峻克制" />
      <div className="col-span-2">
        <TextField label="目标读者" value={data.targetReader} onChange={set("targetReader")}
          placeholder="描述目标读者画像" helperText="年龄层、阅读偏好、期待从作品中获得什么" />
      </div>
      <TextField label="商业卖点" value={data.sellingPoint} onChange={set("sellingPoint")}
        placeholder="作品的独特卖点是什么？" />
      <TextField label="读者期待" value={data.readerExpectation} onChange={set("readerExpectation")}
        placeholder="读者阅读前中后的期待管理" />
    </div>
  );
}

// ── Step 2: 类型策略 ──

function GenreForm({ data, onChange }: { data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  function set(k: string) { return (v: string) => onChange({ ...data, [k]: v }); }
  return (
    <div className="grid grid-cols-2 gap-4">
      <Select label="主类型" value={data.mainGenre} onChange={(e) => set("mainGenre")(e.target.value)} options={GENRE_OPTIONS} placeholder="选择主类型" />
      <Input label="子类型" value={data.subGenre} onChange={(e) => set("subGenre")(e.target.value)} placeholder="例如：东方玄幻、赛博朋克" />
      <Select label="叙事视角" value={data.narrativePov} onChange={(e) => set("narrativePov")(e.target.value)} options={POV_OPTIONS} />
      <Select label="节奏类型" value={data.rhythmType} onChange={(e) => set("rhythmType")(e.target.value)} options={RHYTHM_OPTIONS} placeholder="选择节奏" />
      <div className="col-span-2">
        <TextField label="文风关键词" value={data.styleKeywords} onChange={set("styleKeywords")}
          placeholder="用逗号分隔描述文风的关键词" helperText="例如：冷峻、画面感强、对话密集、诗意" />
      </div>
      <div className="col-span-2">
        <TextField label="禁用风格" value={data.bannedStyle} onChange={set("bannedStyle")}
          placeholder="明确需要避免的写作风格" helperText="例如：网络段子腔、过度解释、鸡汤式总结" />
      </div>
    </div>
  );
}

// ── Step 3: 故事母题 ──

function PremiseForm({ data, onChange }: { data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  function set(k: string) { return (v: string) => onChange({ ...data, [k]: v }); }
  return (
    <div className="grid grid-cols-2 gap-4">
      <div className="col-span-2">
        <TextField label="一句话梗概" value={data.oneLineLogline} onChange={set("oneLineLogline")}
          placeholder="用一句话概括整个故事" helperText="这是你的故事 elevator pitch" />
      </div>
      <div className="col-span-2">
        <TextField label="三段式梗概" value={data.threeParagraphSummary} onChange={set("threeParagraphSummary")}
          placeholder="用三段描述故事的起因、经过、结果" />
      </div>
      <TextField label="开端" value={data.beginning} onChange={set("beginning")}
        placeholder="故事如何开始？" />
      <TextField label="中段" value={data.middle} onChange={set("middle")}
        placeholder="故事中段的核心冲突" />
      <TextField label="高潮" value={data.climax} onChange={set("climax")}
        placeholder="高潮场景的设计" />
      <TextField label="结局方向" value={data.ending} onChange={set("ending")}
        placeholder="预期的结局方向" />
    </div>
  );
}

// ── Step 4: 角色工坊 ──

function CharactersForm({ data, onChange }: { data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  function set(k: string) { return (v: string) => onChange({ ...data, [k]: v }); }
  return (
    <div className="grid grid-cols-2 gap-4">
      <TextField label="主角" value={data.protagonist} onChange={set("protagonist")}
        placeholder="姓名、身份、核心动机、性格特质" />
      <TextField label="反派" value={data.antagonist} onChange={set("antagonist")}
        placeholder="姓名、立场、威胁、与主角的关系" />
      <div className="col-span-2">
        <TextField label="关键配角" value={data.supportingCharacters} onChange={set("supportingCharacters")}
          placeholder="列出重要配角及其作用" />
      </div>
      <div className="col-span-2">
        <TextField label="角色关系摘要" value={data.relationshipSummary} onChange={set("relationshipSummary")}
          placeholder="描述核心角色之间的关系网络" />
      </div>
      <div className="col-span-2">
        <TextField label="角色成长弧线" value={data.growthArc} onChange={set("growthArc")}
          placeholder="主角/重要角色在故事中的心理成长轨迹" />
      </div>
    </div>
  );
}

// ── Step 5: 世界规则 ──

function WorldForm({ data, onChange }: { data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  function set(k: string) { return (v: string) => onChange({ ...data, [k]: v }); }
  return (
    <div className="grid grid-cols-2 gap-4">
      <div className="col-span-2">
        <TextField label="世界背景" value={data.worldBackground} onChange={set("worldBackground")}
          placeholder="时代、地理、氛围、文明程度" />
      </div>
      <div className="col-span-2">
        <TextField label="能力 / 技术 / 制度规则" value={data.rules} onChange={set("rules")}
          placeholder="核心规则体系（修炼体系、科技水平、社会制度等）" />
      </div>
      <TextField label="地点" value={data.locations} onChange={set("locations")}
        placeholder="重要地点及其特征" />
      <TextField label="组织" value={data.organizations} onChange={set("organizations")}
        placeholder="重要组织/势力及其立场" />
      <div className="col-span-2">
        <TextField label="不可违反规则" value={data.inviolableRules} onChange={set("inviolableRules")}
          placeholder="这些设定一旦确定不可更改，AI 生成时必须严格遵守" helperText="例如：魔法不能起死回生、超能力需要代价" />
      </div>
    </div>
  );
}

// ── Step 6: 名词锁定 ──

function GlossaryForm({ data, onChange }: { data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  function set(k: string) { return (v: string) => onChange({ ...data, [k]: v }); }
  return (
    <div className="grid grid-cols-2 gap-4">
      <TextField label="人名" value={data.personNames} onChange={set("personNames")}
        placeholder="重要人物名称，逗号分隔" helperText="这些名称将被锁定，AI 生成时不得擅自修改" />
      <TextField label="地名" value={data.placeNames} onChange={set("placeNames")}
        placeholder="重要地点名称，逗号分隔" />
      <TextField label="组织名" value={data.organizationNames} onChange={set("organizationNames")}
        placeholder="组织/势力名称，逗号分隔" />
      <TextField label="术语" value={data.terms} onChange={set("terms")}
        placeholder="专用术语，逗号分隔" />
      <TextField label="别名" value={data.aliases} onChange={set("aliases")}
        placeholder="允许使用的别名映射（原名→别名）" />
      <TextField label="禁用名词" value={data.bannedTerms} onChange={set("bannedTerms")}
        placeholder="禁止在文中出现的词汇" helperText="这些词如果在正文中出现，一致性检查会标记为问题" />
    </div>
  );
}

// ── Step 7: 剧情骨架 ──

function PlotForm({ data, onChange }: { data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  function set(k: string) { return (v: string) => onChange({ ...data, [k]: v }); }
  return (
    <div className="grid grid-cols-2 gap-4">
      <div className="col-span-2">
        <TextField label="主线目标" value={data.mainGoal} onChange={set("mainGoal")}
          placeholder="作品整体的主线目标是什么？" helperText="主角要达成什么？故事的终极驱动力是什么？" />
      </div>
      <div className="col-span-2">
        <TextField label="阶段节点" value={data.stages} onChange={set("stages")}
          placeholder="按顺序列出故事的主要阶段节点" />
      </div>
      <TextField label="关键冲突" value={data.keyConflicts} onChange={set("keyConflicts")}
        placeholder="核心冲突的设计" />
      <TextField label="反转" value={data.twists} onChange={set("twists")}
        placeholder="预期的反转/意外" />
      <TextField label="高潮" value={data.climax} onChange={set("climax")}
        placeholder="全作品最高潮的设计" />
      <TextField label="结局" value={data.ending} onChange={set("ending")}
        placeholder="最终结局设计" />
    </div>
  );
}

// ── Step 8: 章节路线 ──

function ChaptersForm({ data, onChange }: { data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  function set(k: string) { return (v: string) => onChange({ ...data, [k]: v }); }
  return (
    <div className="grid grid-cols-2 gap-4">
      <div className="col-span-2">
        <TextField label="卷结构" value={data.volumeStructure} onChange={set("volumeStructure")}
          placeholder="全书分为几卷？每卷的核心内容是什么？" />
      </div>
      <div className="col-span-2">
        <TextField label="章节列表" value={data.chapterList} onChange={set("chapterList")}
          placeholder="按顺序列出章节标题和章节号" />
      </div>
      <div className="col-span-2">
        <TextField label="章节目标" value={data.chapterGoals} onChange={set("chapterGoals")}
          placeholder="每个章节需要达成的叙事目标" />
      </div>
      <TextField label="出场人物" value={data.characters} onChange={set("characters")}
        placeholder="各章节的主要出场人物" />
      <TextField label="关联主线节点" value={data.plotNodes} onChange={set("plotNodes")}
        placeholder="章节与主线节点的对应关系" />
    </div>
  );
}

// ── Form dispatcher ──

function StepForm({ stepKey, data, onChange }: { stepKey: BlueprintStepKey; data: Record<string, string>; onChange: (d: Record<string, string>) => void }) {
  switch (stepKey) {
    case "step-01-anchor": return <AnchorForm data={data} onChange={onChange} />;
    case "step-02-genre": return <GenreForm data={data} onChange={onChange} />;
    case "step-03-premise": return <PremiseForm data={data} onChange={onChange} />;
    case "step-04-characters": return <CharactersForm data={data} onChange={onChange} />;
    case "step-05-world": return <WorldForm data={data} onChange={onChange} />;
    case "step-06-glossary": return <GlossaryForm data={data} onChange={onChange} />;
    case "step-07-plot": return <PlotForm data={data} onChange={onChange} />;
    case "step-08-chapters": return <ChaptersForm data={data} onChange={onChange} />;
    default: return <p className="text-surface-400 text-sm">未知步骤</p>;
  }
}

// ── Field labels for each step (used in AI suggestion fallback) ──

const FIELD_LABELS: Record<string, Record<string, string>> = {
  "step-01-anchor": { coreInspiration: "核心灵感", coreProposition: "核心命题", coreEmotion: "核心情绪", targetReader: "目标读者", sellingPoint: "商业卖点", readerExpectation: "读者期待" },
  "step-02-genre": { mainGenre: "主类型", subGenre: "子类型", narrativePov: "叙事视角", styleKeywords: "文风关键词", rhythmType: "节奏类型", bannedStyle: "禁用风格" },
  "step-03-premise": { oneLineLogline: "一句话梗概", threeParagraphSummary: "三段式梗概", beginning: "开端", middle: "中段", climax: "高潮", ending: "结局方向" },
  "step-04-characters": { protagonist: "主角", antagonist: "反派", supportingCharacters: "关键配角", relationshipSummary: "角色关系摘要", growthArc: "成长弧线" },
  "step-05-world": { worldBackground: "世界背景", rules: "规则体系", locations: "地点", organizations: "组织", inviolableRules: "不可违反规则" },
  "step-06-glossary": { personNames: "人名", placeNames: "地名", organizationNames: "组织名", terms: "术语", aliases: "别名", bannedTerms: "禁用名词" },
  "step-07-plot": { mainGoal: "主线目标", stages: "阶段节点", keyConflicts: "关键冲突", twists: "反转", climax: "高潮", ending: "结局" },
  "step-08-chapters": { volumeStructure: "卷结构", chapterList: "章节列表", chapterGoals: "章节目标", characters: "出场人物", plotNodes: "关联主线节点" },
};

// ── Main component ──

export function BlueprintPage() {
  const [steps, setSteps] = useState<Array<{ status: string; content: string; aiGenerated: boolean }>>([]);
  const [activeIdx, setActiveIdx] = useState(0);
  const [formData, setFormData] = useState<Record<string, string>>(() =>
    parseBlueprintContent(STEPS[0].key, "")
  );
  const [saving, setSaving] = useState(false);
  const [aiLoading, setAiLoading] = useState(false);
  const [aiResult, setAiResult] = useState<string | null>(null);
  const [bookIdeaPrompt, setBookIdeaPrompt] = useState("");
  const [bookPipelineRunning, setBookPipelineRunning] = useState(false);
  const [bookPipelineLogs, setBookPipelineLogs] = useState<string[]>([]);
  const [bookPipelineStatus, setBookPipelineStatus] = useState<string | null>(null);
  const [bookPipelineAbort, setBookPipelineAbort] = useState<AbortController | null>(null);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const cur = STEPS[activeIdx];
  const status: StepStatus = (steps[activeIdx]?.status as StepStatus) ?? "not_started";

  const load = useCallback(async () => {
    if (!projectRoot) { setSteps([]); return; }
    const data = await listBlueprintSteps(projectRoot);
    setSteps(data);
  }, [projectRoot]);

  useEffect(() => { void load(); }, [load]);

  // Populate formData when active step changes or steps load
  useEffect(() => {
    const content = steps[activeIdx]?.content ?? "";
    setFormData(parseBlueprintContent(cur.key, content));
    setAiResult(null);
  }, [steps, activeIdx, cur.key]);

  function hasContent(): boolean {
    return Object.values(formData).some((v) => v.trim().length > 0);
  }

  function statusDot(s: StepStatus) {
    return s === "completed" ? "bg-success" : s === "in_progress" ? "bg-info" : "bg-surface-600";
  }

  function statusLabel(s: StepStatus) {
    return s === "completed" ? "已完成" : s === "in_progress" ? "进行中" : "未开始";
  }

  function wordCount(): number {
    return Object.values(formData).reduce((sum, v) => sum + v.replace(/\s/g, "").length, 0);
  }

  async function handleSave() {
    if (!projectRoot) return;
    setSaving(true);
    try {
      const json = JSON.stringify(formData);
      await saveBlueprintStep(cur.key, json, false, projectRoot);
      await load();
    } finally { setSaving(false); }
  }

  async function handleComplete() {
    if (!projectRoot) return;
    if (hasContent()) {
      const json = JSON.stringify(formData);
      await saveBlueprintStep(cur.key, json, false, projectRoot);
    }
    await markBlueprintCompleted(cur.key, projectRoot);
    await load();
  }

  async function handleReset() {
    if (!projectRoot) return;
    await resetBlueprintStep(cur.key, projectRoot);
    setFormData(parseBlueprintContent(cur.key, ""));
    setAiResult(null);
    await load();
  }

  async function handleAiSuggest() {
    if (!projectRoot) return;
    setAiLoading(true);
    setAiResult(null);
    try {
      const textSummary = Object.entries(formData)
        .filter(([, v]) => v.trim())
        .map(([k, v]) => `${(FIELD_LABELS[cur.key]?.[k] ?? k)}：${v}`)
        .join("\n");
      const suggestion = await generateBlueprintSuggestion({
        projectRoot,
        stepKey: cur.key,
        stepTitle: cur.label,
        userInstruction: textSummary || ""
      });
      setAiResult(suggestion.trim() ? suggestion : "AI 返回为空内容，请重试或切换模型后再试。");
      await load();
    } catch {
      setAiResult("AI 建议生成失败。请检查 AI 供应商配置。");
    } finally { setAiLoading(false); }
  }

  function handleApplyAiResult() {
    if (!aiResult) return;
    setFormData(parseBlueprintContent(cur.key, aiResult));
    setAiResult(null);
  }

  function handleFormChange(newData: Record<string, string>) {
    setFormData(newData);
  }

  async function handleRunBookPipeline() {
    if (!projectRoot || !bookIdeaPrompt.trim() || bookPipelineRunning) return;
    const abortController = new AbortController();
    setBookPipelineAbort(abortController);
    setBookPipelineRunning(true);
    setBookPipelineStatus(null);
    setBookPipelineLogs([]);
    try {
      for await (const event of streamBookGenerationPipeline(
        {
          projectRoot,
          ideaPrompt: bookIdeaPrompt.trim(),
        },
        abortController.signal,
      )) {
        if (event.type === "stage-start") {
          setBookPipelineLogs((prev) => [...prev, `开始：${event.stageLabel}`]);
          continue;
        }
        if (event.type === "stage-done") {
          setBookPipelineLogs((prev) => [...prev, `完成：${event.stageLabel}`]);
          continue;
        }
        if (event.type === "stage-error") {
          setBookPipelineLogs((prev) => [...prev, `失败：${event.stageLabel} - ${event.message}`]);
          setBookPipelineStatus(event.message);
          return;
        }
      }
      setBookPipelineStatus("全书生成编排执行完成");
      await load();
    } catch (error) {
      setBookPipelineStatus(error instanceof Error ? error.message : "全书生成编排执行失败");
    } finally {
      setBookPipelineRunning(false);
      setBookPipelineAbort(null);
    }
  }

  function handleCancelBookPipeline() {
    if (!bookPipelineAbort) return;
    bookPipelineAbort.abort();
    setBookPipelineStatus("已取消全书生成编排");
  }

  return (
    <div className="max-w-6xl mx-auto">
      <h1 className="text-2xl font-bold text-surface-100 mb-6">创作蓝图</h1>
      <div className="flex gap-6">
        {/* ── Sidebar nav ── */}
        <div className="w-56 shrink-0">
          <nav className="space-y-1">
            {STEPS.map((s, i) => {
              const st: StepStatus = (steps[i]?.status as StepStatus) ?? "not_started";
              return (
                <button
                  key={s.key}
                  onClick={() => { if (hasContent()) void handleSave(); setActiveIdx(i); }}
                  className={`w-full flex items-center gap-3 px-3 py-2.5 text-sm rounded-lg transition-colors ${
                    i === activeIdx ? "bg-primary/10 text-primary" : "text-surface-300 hover:bg-surface-700 hover:text-surface-100"
                  }`}
                >
                  <span className={`w-2 h-2 rounded-full shrink-0 ${statusDot(st)}`} />
                  <span className="truncate">{s.label}</span>
                  {st === "completed" && <span className="text-success ml-auto">✓</span>}
                </button>
              );
            })}
          </nav>
        </div>

        {/* ── Main form ── */}
        <div className="flex-1 min-w-0">
          <Card padding="lg">
            <div className="flex items-center justify-between mb-4">
              <div>
                <h2 className="text-lg font-semibold text-surface-100">{cur.label}</h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={status === "completed" ? "success" : status === "in_progress" ? "info" : "default"}>
                    {statusLabel(status)}
                  </Badge>
                  {steps[activeIdx]?.aiGenerated && <span className="text-xs text-primary">AI 生成</span>}
                  <span className="text-xs text-surface-400">{cur.desc}</span>
                </div>
              </div>
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" onClick={handleReset}>重置</Button>
                <Button variant="secondary" size="sm" loading={saving} onClick={() => void handleSave()}>保存</Button>
                <Button variant="primary" size="sm" onClick={() => void handleComplete()}>
                  {status === "completed" ? "已完成 ✓" : "标记完成"}
                </Button>
              </div>
            </div>

            <StepForm stepKey={cur.key} data={formData} onChange={handleFormChange} />

            {hasContent() && (
              <div className="mt-3 text-xs text-surface-400">
                总字数：{wordCount()}
              </div>
            )}

            {/* ── AI result panel ── */}
            {aiResult && (
              <div className="mt-4 p-4 bg-primary/5 border border-primary/20 rounded-xl">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs font-medium text-primary">AI 建议（已自动写入当前步骤，可选同步到表单）</span>
                  <div className="flex gap-2">
                    <Button variant="primary" size="sm" onClick={handleApplyAiResult}>填充到表单</Button>
                    <Button variant="ghost" size="sm" onClick={() => setAiResult(null)}>忽略</Button>
                  </div>
                </div>
                <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed max-h-64 overflow-y-auto">{aiResult}</pre>
              </div>
            )}
          </Card>
        </div>

        {/* ── AI sidebar ── */}
        <div className="w-64 shrink-0 hidden lg:block">
          <Card padding="md">
            <h3 className="text-sm font-semibold text-surface-200 mb-3">AI 建议</h3>
            <p className="text-xs text-surface-400 mb-3">
              AI 可基于已有内容生成当前步骤的完善建议。
            </p>
            <Button
              variant="secondary"
              size="sm"
              className="w-full justify-center mb-3"
              loading={aiLoading}
              onClick={() => void handleAiSuggest()}
            >
              {aiLoading ? "生成中..." : "生成并写入"}
            </Button>
            {!projectRoot && <p className="text-xs text-warning mb-2">请先打开项目</p>}
          </Card>
          <Card padding="md" className="mt-3">
            <h3 className="text-sm font-semibold text-surface-200 mb-3">一键全书生成</h3>
            <Textarea
              label="创意提示词"
              value={bookIdeaPrompt}
              onChange={(e) => setBookIdeaPrompt(e.target.value)}
              placeholder="输入核心创意，按阶段自动生成蓝图/角色/设定/剧情"
            />
            <div className="mt-3 flex gap-2">
              <Button
                variant="primary"
                size="sm"
                className="flex-1 justify-center"
                loading={bookPipelineRunning}
                onClick={() => void handleRunBookPipeline()}
                disabled={!bookIdeaPrompt.trim()}
              >
                {bookPipelineRunning ? "编排中..." : "开始编排"}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="justify-center"
                onClick={handleCancelBookPipeline}
                disabled={!bookPipelineRunning}
              >
                取消
              </Button>
            </div>
            {bookPipelineStatus && (
              <p className="mt-3 text-xs text-surface-300">{bookPipelineStatus}</p>
            )}
            {bookPipelineLogs.length > 0 && (
              <div className="mt-3 max-h-28 overflow-y-auto rounded-lg border border-surface-700 bg-surface-800/80 p-2">
                {bookPipelineLogs.map((log, idx) => (
                  <p key={`${log}-${idx}`} className="text-[11px] text-surface-300">
                    {log}
                  </p>
                ))}
              </div>
            )}
          </Card>
        </div>
      </div>
    </div>
  );
}
