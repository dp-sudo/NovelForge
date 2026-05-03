import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Select } from "../../components/forms/Select.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { ConfirmDialog } from "../../components/dialogs/ConfirmDialog.js";
import { listWorldRules, createWorldRule, deleteWorldRule, aiGenerateWorldRule, type WorldRow } from "../../api/worldApi.js";
import { useProjectStore } from "../../stores/projectStore.js";
import type { WorldRuleInput } from "../../domain/types.js";

const CATEGORIES = [
  { value: "世界规则", label: "世界规则" },
  { value: "地点", label: "地点" },
  { value: "组织", label: "组织" },
  { value: "道具", label: "道具" },
  { value: "能力", label: "能力" },
  { value: "历史事件", label: "历史事件" },
  { value: "术语", label: "术语" }
];

const CONSTRAINTS = [
  { value: "weak", label: "弱设定" },
  { value: "normal", label: "普通设定" },
  { value: "strong", label: "强约束" },
  { value: "absolute", label: "绝对不可违反" }
];

const emptyForm = { title: "", category: "世界规则" as const, description: "", constraintLevel: "normal" as const, examples: "" };

function pickText(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function normalizeConstraintLevel(value: unknown): "weak" | "normal" | "strong" | "absolute" {
  const raw = pickText(value)?.toLowerCase();
  if (!raw) return "normal";
  if (["weak", "low", "soft", "弱", "弱设定"].some((x) => raw.includes(x))) return "weak";
  if (["strong", "high", "严格", "强"].some((x) => raw.includes(x))) return "strong";
  if (["absolute", "must", "forbid", "绝对", "不可"].some((x) => raw.includes(x))) return "absolute";
  return "normal";
}

function normalizeWorldCategory(value: unknown): WorldRuleInput["category"] {
  const text = pickText(value);
  if (!text) {
    return "世界规则";
  }
  const matched = CATEGORIES.find((item) => item.value === text);
  if (matched) {
    return matched.value as WorldRuleInput["category"];
  }
  if (text.includes("地点")) return "地点";
  if (text.includes("组织")) return "组织";
  if (text.includes("道具")) return "道具";
  if (text.includes("能力")) return "能力";
  if (text.includes("历史")) return "历史事件";
  if (text.includes("术语")) return "术语";
  return "世界规则";
}

export function WorldPage() {
  const [rules, setRules] = useState<WorldRow[]>([]);
  const [filter, setFilter] = useState("全部");
  const [selected, setSelected] = useState<WorldRow | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [showDelete, setShowDelete] = useState(false);
  const [form, setForm] = useState(emptyForm);
  const [aiDescription, setAiDescription] = useState("");
  const [aiResult, setAiResult] = useState<string | null>(null);
  const [aiLoading, setAiLoading] = useState(false);
  const [showAiCreate, setShowAiCreate] = useState(false);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setRules([]);
      return;
    }
    const data = await listWorldRules(projectRoot);
    setRules(data);
  }, [projectRoot]);

  useEffect(() => { void load(); }, [load]);

  const categories = ["全部", ...new Set(rules.map((r) => r.category))];
  const filtered = filter === "全部" ? rules : rules.filter((r) => r.category === filter);

  const constraintColors: Record<string, string> = {
    weak: "text-surface-400", normal: "text-info", strong: "text-warning", absolute: "text-error"
  };

  async function handleCreate() {
    if (!form.title.trim() || !projectRoot) return;
    await createWorldRule({
      title: form.title.trim(),
      category: form.category,
      description: form.description,
      constraintLevel: form.constraintLevel,
      examples: form.examples || undefined
    }, projectRoot);
    setForm(emptyForm);
    setShowNew(false);
    await load();
  }

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-surface-100">世界设定库</h1>
        <div className="flex gap-2">
          <Button variant="ghost" size="sm" onClick={() => { setAiDescription(""); setAiResult(null); setShowAiCreate(true); }}>AI 生成</Button>
          <Button variant="primary" size="sm" onClick={() => { setForm(emptyForm); setShowNew(true); }}>新建设定</Button>
        </div>
      </div>

      <div className="flex gap-6">
        <div className="w-48 shrink-0">
          <div className="space-y-1">
            {categories.map((cat) => (
              <button
                key={cat}
                onClick={() => setFilter(cat)}
                className={`w-full text-left px-3 py-2 text-sm rounded-lg transition-colors ${
                  filter === cat ? "bg-primary/10 text-primary" : "text-surface-300 hover:bg-surface-700"
                }`}
              >
                {cat} {cat !== "全部" && `(${rules.filter((r) => r.category === cat).length})`}
              </button>
            ))}
          </div>
        </div>

        <div className="w-64 shrink-0 space-y-2">
          {filtered.length === 0 ? (
            <Card padding="md" className="text-center"><p className="text-sm text-surface-400">暂无设定</p></Card>
          ) : (
            filtered.map((r) => (
              <button
                key={r.id}
                onClick={() => setSelected(r)}
                className={`w-full text-left p-3 rounded-lg transition-colors border ${
                  selected?.id === r.id ? "bg-primary/10 border-primary/30" : "bg-surface-800 border-surface-700 hover:border-surface-500"
                }`}
              >
                <div className="text-sm font-medium text-surface-100">{r.title}</div>
                <div className="flex items-center gap-2 mt-1">
                  <span className="text-xs text-surface-400">{r.category}</span>
                  <span className={`text-xs ${constraintColors[r.constraint_level] ?? ""}`}>
                    {CONSTRAINTS.find((c) => c.value === r.constraint_level)?.label ?? r.constraint_level}
                  </span>
                </div>
              </button>
            ))
          )}
        </div>

        <div className="flex-1 min-w-0">
          {!selected ? (
            <Card padding="lg" className="text-center"><p className="text-surface-400 text-sm">选择一个设定查看详情</p></Card>
          ) : (
            <Card padding="lg" className="space-y-4">
              <div className="flex items-center justify-between">
                <h2 className="text-lg font-semibold text-surface-100">{selected.title}</h2>
                <Button variant="danger" size="sm" onClick={() => setShowDelete(true)}>删除</Button>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <Input label="类型" value={selected.category} readOnly />
                <Input
                  label="约束等级"
                  value={CONSTRAINTS.find((c) => c.value === selected.constraint_level)?.label ?? selected.constraint_level}
                  readOnly
                />
              </div>
              <Textarea label="描述" value={selected.description} readOnly className="min-h-[80px]" />
              {selected.examples && <Textarea label="示例" value={selected.examples} readOnly />}
            </Card>
          )}
        </div>
      </div>

      <Modal open={showNew} onClose={() => setShowNew(false)} title="新建设定" width="lg">
        <div className="space-y-4">
          <Input label="标题 *" value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />
          <div className="grid grid-cols-2 gap-4">
            <Select label="类型" value={form.category} onChange={(e) => setForm({ ...form, category: e.target.value as typeof form.category })} options={CATEGORIES} />
            <Select label="约束等级" value={form.constraintLevel} onChange={(e) => setForm({ ...form, constraintLevel: e.target.value as typeof form.constraintLevel })} options={CONSTRAINTS} />
          </div>
          <Textarea label="描述" value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} className="min-h-[100px]" />
          <Textarea label="示例（可选）" value={form.examples} onChange={(e) => setForm({ ...form, examples: e.target.value })} />
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleCreate()} disabled={!form.title.trim()}>创建</Button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        open={showDelete}
        title="删除设定"
        message={`确定删除「${selected?.title}」吗？`}
        variant="danger"
        confirmLabel="删除"
        onConfirm={() => { if (selected && projectRoot) void deleteWorldRule(selected.id, projectRoot).then(() => load()); setShowDelete(false); setSelected(null); }}
        onCancel={() => setShowDelete(false)}
      />

      <Modal open={showAiCreate} onClose={() => setShowAiCreate(false)} title="AI 生成世界设定" width="lg">
        <div className="space-y-4">
          <Textarea
            label="描述你想要的设定"
            value={aiDescription}
            onChange={(e) => setAiDescription(e.target.value)}
            placeholder="例如：一个以契约魔法为核心的修炼体系，魔法师通过签订契约获得力量"
            className="min-h-[100px]"
          />
          <Button
            variant="primary"
            loading={aiLoading}
            onClick={async () => {
              if (!projectRoot) return;
              setAiLoading(true);
              try {
                setAiResult(await aiGenerateWorldRule(projectRoot, aiDescription));
                await load();
              }
              catch { setAiResult("AI 生成失败。请检查 AI 供应商配置。"); }
              finally { setAiLoading(false); }
            }}
            disabled={!aiDescription.trim()}
          >
            {aiLoading ? "生成中..." : "生成并入库"}
          </Button>
          {aiResult && (
            <div className="p-4 bg-primary/5 border border-primary/20 rounded-xl">
              <p className="text-xs text-success mb-2">AI 结果已自动写入世界设定库。</p>
              <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed max-h-64 overflow-y-auto">{aiResult}</pre>
              <div className="flex gap-2 mt-3">
                <Button variant="ghost" size="sm" onClick={() => setAiResult(null)}>重新生成</Button>
                <Button variant="primary" size="sm" onClick={() => { setAiResult(null); setShowAiCreate(false); setAiDescription(""); }}>关闭</Button>
              </div>
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
}
