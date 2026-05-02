import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Select } from "../../components/forms/Select.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { Badge } from "../../components/ui/Badge.js";
import type { PlotNodeInput } from "../../domain/types.js";
import { listPlotNodes, createPlotNode, reorderPlotNodes, type PlotRow } from "../../api/plotApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

const NODE_TYPES = [
  "开端", "转折", "冲突", "失败", "胜利", "高潮", "结局", "支线"
];
const NODE_STATUSES = [
  { value: "未使用", label: "未使用" },
  { value: "规划中", label: "规划中" },
  { value: "已写入", label: "已写入" },
  { value: "需调整", label: "需调整" }
];
const nodeTypeColors: Record<string, BadgeVariant> = {
  "开端": "info", "转折": "warning", "冲突": "error", "失败": "error",
  "胜利": "success", "高潮": "warning", "结局": "info", "支线": "default"
};

type BadgeVariant = "default" | "success" | "warning" | "error" | "info";

function pickText(value: unknown): string | undefined {
  // 问题7修复: 清理未使用解析器，保留当前仍被调用的最小规范化工具函数。
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function normalizeNodeType(value: unknown): PlotNodeInput["nodeType"] {
  const raw = pickText(value);
  if (!raw) return "开端";
  if ((NODE_TYPES as readonly string[]).includes(raw)) {
    return raw as PlotNodeInput["nodeType"];
  }
  if (raw.includes("高潮")) return "高潮";
  if (raw.includes("结局")) return "结局";
  if (raw.includes("冲突")) return "冲突";
  if (raw.includes("转折")) return "转折";
  if (raw.includes("胜")) return "胜利";
  if (raw.includes("败")) return "失败";
  if (raw.includes("支线")) return "支线";
  return "开端";
}

function normalizeStatus(value: unknown): PlotNodeInput["status"] {
  const raw = pickText(value);
  if (!raw) return "规划中";
  if (raw === "未使用" || raw === "规划中" || raw === "已写入" || raw === "需调整") {
    return raw;
  }
  return "规划中";
}

export function PlotPage() {
  const [nodes, setNodes] = useState<PlotRow[]>([]);
  const [selected, setSelected] = useState<PlotRow | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [dragIdx, setDragIdx] = useState<number | null>(null);
  const [form, setForm] = useState({ title: "", nodeType: "开端", sortOrder: 1, goal: "", conflict: "", status: "规划中" });
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setNodes([]);
      return;
    }
    const data = await listPlotNodes(projectRoot);
    setNodes(data);
  }, [projectRoot]);

  useEffect(() => { void load(); }, [load]);

  async function handleCreate() {
    if (!form.title.trim() || !projectRoot) return;
    const nextOrder = nodes.length > 0 ? Math.max(...nodes.map((n) => n.sort_order)) + 1 : 1;
    await createPlotNode({
      title: form.title.trim(),
      nodeType: form.nodeType as "开端" | "转折" | "冲突" | "失败" | "胜利" | "高潮" | "结局" | "支线",
      sortOrder: nextOrder,
      goal: form.goal || undefined,
      conflict: form.conflict || undefined,
      status: form.status as "未使用" | "规划中" | "已写入" | "需调整"
    }, projectRoot);
    setForm({ title: "", nodeType: "开端", sortOrder: 1, goal: "", conflict: "", status: "规划中" });
    setShowNew(false);
    await load();
  }

  async function handleMoveUp(index: number) {
    if (index === 0 || !projectRoot) return;
    const ids = nodes.map((n) => n.id);
    [ids[index - 1], ids[index]] = [ids[index], ids[index - 1]];
    await reorderPlotNodes(ids, projectRoot);
    await load();
  }

  async function handleMoveDown(index: number) {
    if (index === nodes.length - 1 || !projectRoot) return;
    const ids = nodes.map((n) => n.id);
    [ids[index], ids[index + 1]] = [ids[index + 1], ids[index]];
    await reorderPlotNodes(ids, projectRoot);
    await load();
  }

  return (
    <div className="max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-surface-100">剧情骨架</h1>
        <div className="flex gap-2">
          <Button variant="primary" size="sm" onClick={() => setShowNew(true)}>新增节点</Button>
        </div>
      </div>

      <div className="mb-4 rounded-lg border border-surface-700 bg-surface-800/70 px-3 py-2">
        <p className="text-xs text-surface-400">
          本页用于正式剧情节点的深度整理。章节推进中的主链编排和使用上下文，已统一收束到全书指挥台。
        </p>
      </div>

      <div className="flex gap-6">
        <div className="flex-1 min-w-0">
          {nodes.length === 0 ? (
            <Card padding="lg" className="text-center">
              <p className="text-sm text-surface-400 mb-3">还没有剧情节点</p>
              <Button variant="primary" size="sm" onClick={() => setShowNew(true)}>新增节点</Button>
            </Card>
          ) : (
            <div className="space-y-2">
              {nodes.map((node, i) => (
                <button
                  key={node.id}
                  onClick={() => setSelected(node)}
                  className={`w-full text-left p-4 rounded-lg transition-colors border ${
                    selected?.id === node.id
                      ? "bg-primary/10 border-primary/30"
                      : "bg-surface-800 border-surface-700 hover:border-surface-500"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <span className="text-xs text-surface-500 w-6 shrink-0">#{node.sort_order}</span>
                    <Badge variant={nodeTypeColors[node.node_type] ?? "default"}>{node.node_type}</Badge>
                    <span className="text-sm font-medium text-surface-100">{node.title}</span>
                    {node.status !== "规划中" && (
                      <span className="text-xs text-surface-400">({node.status})</span>
                    )}
                    <div className="ml-auto flex gap-1">
                      <button
                        onClick={(e) => { e.stopPropagation(); handleMoveUp(i); }}
                        disabled={i === 0}
                        className="px-1.5 py-0.5 text-xs text-surface-400 hover:text-surface-200 disabled:opacity-30"
                      >
                        ▲
                      </button>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleMoveDown(i); }}
                        disabled={i === nodes.length - 1}
                        className="px-1.5 py-0.5 text-xs text-surface-400 hover:text-surface-200 disabled:opacity-30"
                      >
                        ▼
                      </button>
                    </div>
                  </div>
                  {node.goal && <div className="mt-1 text-xs text-surface-400 ml-9">{node.goal.slice(0, 60)}</div>}
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="w-80 shrink-0">
          {selected && (
            <Card padding="md" className="space-y-3">
              <h3 className="text-sm font-semibold text-surface-100">{selected.title}</h3>
              <Badge variant={nodeTypeColors[selected.node_type] ?? "default"}>{selected.node_type}</Badge>
              <span className="text-xs text-surface-400 ml-2">#{selected.sort_order}</span>
              {selected.goal && (
                <div>
                  <label className="text-xs text-surface-400">目标</label>
                  <p className="text-sm text-surface-200">{selected.goal}</p>
                </div>
              )}
              {selected.conflict && (
                <div>
                  <label className="text-xs text-surface-400">冲突</label>
                  <p className="text-sm text-surface-200">{selected.conflict}</p>
                </div>
              )}
              {selected.emotional_curve && (
                <div>
                  <label className="text-xs text-surface-400">情绪曲线</label>
                  <p className="text-sm text-surface-200">{selected.emotional_curve}</p>
                </div>
              )}
              <div>
                <label className="text-xs text-surface-400">状态</label>
                <p className="text-sm text-surface-200">{selected.status}</p>
              </div>
            </Card>
          )}
        </div>
      </div>
      <Modal open={showNew} onClose={() => setShowNew(false)} title="新增剧情节点" width="sm">
        <div className="space-y-4">
          <Input label="标题 *" value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />
          <Select label="节点类型" value={form.nodeType} onChange={(e) => setForm({ ...form, nodeType: e.target.value })} options={NODE_TYPES.map((t) => ({ value: t, label: t }))} />
          <Select label="状态" value={form.status} onChange={(e) => setForm({ ...form, status: e.target.value })} options={NODE_STATUSES} />
          <Input label="目标" value={form.goal} onChange={(e) => setForm({ ...form, goal: e.target.value })} />
          <Input label="冲突" value={form.conflict} onChange={(e) => setForm({ ...form, conflict: e.target.value })} />
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleCreate()} disabled={!form.title.trim()}>创建</Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
