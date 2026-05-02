import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Select } from "../../components/forms/Select.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { Badge } from "../../components/ui/Badge.js";
import type { GlossaryTermInput } from "../../domain/types.js";
import { listGlossaryTerms, createGlossaryTerm, type GlossaryRow } from "../../api/glossaryApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

const TERM_TYPES = [
  { value: "人名", label: "人名" },
  { value: "地名", label: "地名" },
  { value: "组织名", label: "组织名" },
  { value: "术语", label: "术语" },
  { value: "别名", label: "别名" },
  { value: "禁用词", label: "禁用词" }
];

const termTypeColors: Record<string, BadgeVariant> = {
  "人名": "info", "地名": "default", "组织名": "warning", "术语": "default", "别名": "info", "禁用词": "error"
};

type BadgeVariant = "default" | "success" | "warning" | "error" | "info";

export function GlossaryPage() {
  const [terms, setTerms] = useState<GlossaryRow[]>([]);
  const [showNew, setShowNew] = useState(false);
  const [form, setForm] = useState({ term: "", termType: "术语" as string, description: "", locked: false, banned: false });
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setTerms([]);
      return;
    }
    const data = await listGlossaryTerms(projectRoot);
    setTerms(data);
  }, [projectRoot]);

  useEffect(() => { void load(); }, [load]);

  async function handleCreate() {
    if (!form.term.trim() || !projectRoot) return;
    await createGlossaryTerm({
      term: form.term.trim(),
      termType: form.termType as GlossaryTermInput["termType"],
      description: form.description || undefined,
      locked: form.locked,
      banned: form.banned
    }, projectRoot);
    setForm({ term: "", termType: "术语", description: "", locked: false, banned: false });
    setShowNew(false);
    await load();
  }

  return (
    <div className="max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-surface-100">名词库</h1>
        <div className="flex gap-2">
          <Button variant="primary" size="sm" onClick={() => setShowNew(true)}>新增名词</Button>
        </div>
      </div>

      <div className="mb-4 rounded-lg border border-surface-700 bg-surface-800/70 px-3 py-2">
        <p className="text-xs text-surface-400">
          本页是正式名词的深度整理入口。主生产链中的抽取、锁定和审查已整合到全书指挥台。
        </p>
      </div>

      <Card padding="none">
        {terms.length === 0 ? (
          <div className="text-center py-12 text-surface-400 text-sm">还没有名词，点击"新增名词"添加</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-surface-700 text-surface-400 text-xs uppercase">
                  <th className="text-left px-4 py-3 font-medium">名词</th>
                  <th className="text-left px-4 py-3 font-medium">类型</th>
                  <th className="text-left px-4 py-3 font-medium">别名</th>
                  <th className="text-left px-4 py-3 font-medium">描述</th>
                  <th className="text-center px-4 py-3 font-medium w-20">锁定</th>
                  <th className="text-center px-4 py-3 font-medium w-20">禁用</th>
                </tr>
              </thead>
              <tbody>
                {terms.map((t) => (
                  <tr key={t.id} className="border-b border-surface-700/50 hover:bg-surface-800/50">
                    <td className="px-4 py-3 text-surface-100 font-medium">{t.term}</td>
                    <td className="px-4 py-3">
                      <Badge variant={termTypeColors[t.term_type] ?? "default"}>{t.term_type}</Badge>
                    </td>
                    <td className="px-4 py-3 text-surface-300">
                      {Array.isArray(t.aliases) ? t.aliases.join(", ") : ""}
                    </td>
                    <td className="px-4 py-3 text-surface-400 max-w-[200px] truncate">{t.description ?? ""}</td>
                    <td className="px-4 py-3 text-center">
                      {t.locked ? <span className="text-success">✓</span> : <span className="text-surface-500">-</span>}
                    </td>
                    <td className="px-4 py-3 text-center">
                      {t.banned ? <span className="text-error">✗</span> : <span className="text-surface-500">-</span>}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      <Modal open={showNew} onClose={() => setShowNew(false)} title="新增名词" width="sm">
        <div className="space-y-4">
          <Input label="名词 *" value={form.term} onChange={(e) => setForm({ ...form, term: e.target.value })} placeholder="输入名词" />
          <Select label="类型" value={form.termType} onChange={(e) => setForm({ ...form, termType: e.target.value })} options={TERM_TYPES} />
          <Input label="描述" value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} placeholder="可选" />
          <div className="flex gap-6">
            <label className="flex items-center gap-2 text-sm text-surface-300 cursor-pointer">
              <input type="checkbox" checked={form.locked} onChange={(e) => setForm({ ...form, locked: e.target.checked })} className="accent-primary" />
              锁定（不可改动）
            </label>
            <label className="flex items-center gap-2 text-sm text-surface-300 cursor-pointer">
              <input type="checkbox" checked={form.banned} onChange={(e) => setForm({ ...form, banned: e.target.checked })} className="accent-primary" />
              禁用（禁止出现）
            </label>
          </div>
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleCreate()} disabled={!form.term.trim()}>创建</Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
