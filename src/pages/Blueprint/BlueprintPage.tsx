import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Badge } from "../../components/ui/Badge.js";
import { Button } from "../../components/ui/Button.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { listBlueprintSteps, saveBlueprintStep, markBlueprintCompleted, resetBlueprintStep, generateBlueprintSuggestion, type BlueprintStepRow } from "../../api/blueprintApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

const STEPS = [
  { key: "step-01-anchor" as const, label: "灵感定锚", desc: "核心灵感、命题、情绪、目标读者、卖点、读者期待" },
  { key: "step-02-genre" as const, label: "类型策略", desc: "主类型、子类型、叙事视角、文风、节奏、禁用风格" },
  { key: "step-03-premise" as const, label: "故事母题", desc: "一句话梗概、三段式梗概、开端、中段、高潮、结局方向" },
  { key: "step-04-characters" as const, label: "角色工坊", desc: "主角、反派、关键配角、角色关系、成长弧线" },
  { key: "step-05-world" as const, label: "世界规则", desc: "世界背景、规则、地点、组织、不可违反规则" },
  { key: "step-06-glossary" as const, label: "名词锁定", desc: "人名、地名、组织名、术语、别名、禁用名词" },
  { key: "step-07-plot" as const, label: "剧情骨架", desc: "主线目标、阶段节点、关键冲突、反转、高潮、结局" },
  { key: "step-08-chapters" as const, label: "章节路线", desc: "卷结构、章节列表、目标、出场人物、关联主线节点" }
];

type StepStatus = "not_started" | "in_progress" | "completed";

export function BlueprintPage() {
  const [steps, setSteps] = useState<BlueprintStepRow[]>([]);
  const [activeIdx, setActiveIdx] = useState(0);
  const [content, setContent] = useState("");
  const [saving, setSaving] = useState(false);
  const [aiLoading, setAiLoading] = useState(false);
  const [aiResult, setAiResult] = useState<string | null>(null);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setSteps([]);
      return;
    }
    const data = await listBlueprintSteps(projectRoot);
    setSteps(data);
  }, [projectRoot]);

  useEffect(() => { void load(); }, [load]);

  useEffect(() => {
    setContent(steps[activeIdx]?.content ?? "");
    setAiResult(null);
  }, [steps, activeIdx]);

  const cur = STEPS[activeIdx];
  const status: StepStatus = (steps[activeIdx]?.status as StepStatus) ?? "not_started";

  function statusDot(s: StepStatus) {
    return s === "completed" ? "bg-success" : s === "in_progress" ? "bg-info" : "bg-surface-600";
  }
  function statusLabel(s: StepStatus) {
    return s === "completed" ? "已完成" : s === "in_progress" ? "进行中" : "未开始";
  }

  async function handleSave() {
    if (!projectRoot) return;
    setSaving(true);
    try {
      await saveBlueprintStep(cur.key, content, false, projectRoot);
      await load();
    } finally {
      setSaving(false);
    }
  }

  async function handleComplete() {
    if (!projectRoot) return;
    if (content.trim()) await saveBlueprintStep(cur.key, content, false, projectRoot);
    await markBlueprintCompleted(cur.key, projectRoot);
    await load();
  }

  async function handleReset() {
    if (!projectRoot) return;
    await resetBlueprintStep(cur.key, projectRoot);
    setContent("");
    setAiResult(null);
    await load();
  }

  async function handleAiSuggest() {
    if (!projectRoot) return;
    setAiLoading(true);
    setAiResult(null);
    try {
      const suggestion = await generateBlueprintSuggestion({
        projectRoot,
        stepKey: cur.key,
        stepTitle: cur.label,
        userInstruction: content.trim() || ""
      });
      setAiResult(suggestion || "未能生成建议。请检查 AI 供应商配置和任务路由。");
    } catch {
      setAiResult("AI 建议生成失败。请检查 AI 供应商配置。");
    } finally {
      setAiLoading(false);
    }
  }

  function handleApplyAiResult() {
    if (aiResult) {
      setContent(aiResult);
      setAiResult(null);
    }
  }

  return (
    <div className="max-w-6xl mx-auto">
      <h1 className="text-2xl font-bold text-surface-100 mb-6">创作蓝图</h1>
      <div className="flex gap-6">
        <div className="w-56 shrink-0">
          <nav className="space-y-1">
            {STEPS.map((s, i) => {
              const st = (steps[i]?.status as StepStatus) ?? "not_started";
              return (
                <button
                  key={s.key}
                  onClick={() => { if (content.trim()) void handleSave(); setActiveIdx(i); }}
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
                  <span className="text-xs text-surface-400">· {cur.desc}</span>
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

            <Textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder={`请输入 ${cur.label} 的内容…\n\n建议包含以下方面：\n${cur.desc.split("、").map((f) => `- ${f}`).join("\n")}`}
              className="min-h-[300px] text-sm leading-relaxed"
            />

            {content.trim() && (
              <div className="mt-3 text-xs text-surface-400">
                字数：{content.replace(/\s/g, "").length}
              </div>
            )}

            {aiResult && (
              <div className="mt-4 p-4 bg-primary/5 border border-primary/20 rounded-xl">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs font-medium text-primary">AI 建议</span>
                  <div className="flex gap-2">
                    <Button variant="primary" size="sm" onClick={handleApplyAiResult}>采用</Button>
                    <Button variant="ghost" size="sm" onClick={() => setAiResult(null)}>忽略</Button>
                  </div>
                </div>
                <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed">{aiResult}</pre>
              </div>
            )}
          </Card>
        </div>

        <div className="w-64 shrink-0 hidden lg:block">
          <Card padding="md">
            <h3 className="text-sm font-semibold text-surface-200 mb-3">AI 建议</h3>
            <p className="text-xs text-surface-400 mb-3">
              AI 可基于项目已有资产生成当前步骤的建议内容。
            </p>
            <Button
              variant="secondary"
              size="sm"
              className="w-full justify-center mb-3"
              loading={aiLoading}
              onClick={() => void handleAiSuggest()}
            >
              {aiLoading ? "生成中..." : "生成建议"}
            </Button>
            {!projectRoot && (
              <p className="text-xs text-warning mb-2">请先打开项目</p>
            )}
          </Card>
        </div>
      </div>
    </div>
  );
}
