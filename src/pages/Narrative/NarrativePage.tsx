import { useCallback, useEffect, useMemo, useState } from "react";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Select } from "../../components/forms/Select.js";
import { Textarea } from "../../components/forms/Textarea.js";
import {
  createNarrativeObligation,
  deleteNarrativeObligation,
  listNarrativeObligations,
  updateObligationStatus,
  type NarrativeObligation,
} from "../../api/narrativeApi.js";
import { listChapters, type ChapterRecord } from "../../api/chapterApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

const OBLIGATION_TYPES = [
  { value: "foreshadowing", label: "伏笔" },
  { value: "promise", label: "承诺" },
  { value: "mystery", label: "谜团" },
  { value: "relationship", label: "关系线" },
  { value: "setup", label: "设定铺垫" },
];

const STATUS_OPTIONS = [
  { value: "open", label: "未兑现" },
  { value: "in_progress", label: "处理中" },
  { value: "paid_off", label: "已兑现" },
  { value: "dropped", label: "放弃" },
];

const SEVERITY_OPTIONS = [
  { value: "low", label: "低" },
  { value: "medium", label: "中" },
  { value: "high", label: "高" },
];

interface ObligationFormState {
  obligationType: string;
  description: string;
  plantedChapterId: string;
  expectedPayoffChapterId: string;
  actualPayoffChapterId: string;
  payoffStatus: string;
  severity: string;
  relatedEntities: string;
}

const INITIAL_FORM: ObligationFormState = {
  obligationType: "foreshadowing",
  description: "",
  plantedChapterId: "",
  expectedPayoffChapterId: "",
  actualPayoffChapterId: "",
  payoffStatus: "open",
  severity: "medium",
  relatedEntities: "",
};

function chapterNameById(chapters: ChapterRecord[]): Record<string, string> {
  const map: Record<string, string> = {};
  for (const chapter of chapters) {
    map[chapter.id] = `#${chapter.chapterIndex} ${chapter.title}`;
  }
  return map;
}

export function NarrativePage() {
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const [chapters, setChapters] = useState<ChapterRecord[]>([]);
  const [obligations, setObligations] = useState<NarrativeObligation[]>([]);
  const [form, setForm] = useState<ObligationFormState>(INITIAL_FORM);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [statusUpdatingId, setStatusUpdatingId] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const chapterLabelMap = useMemo(() => chapterNameById(chapters), [chapters]);
  const chapterOptions = useMemo(
    () => [
      { value: "", label: "不关联章节" },
      ...chapters.map((ch) => ({
        value: ch.id,
        label: `#${ch.chapterIndex} ${ch.title}`,
      })),
    ],
    [chapters],
  );

  const loadData = useCallback(async () => {
    if (!projectRoot) {
      setChapters([]);
      setObligations([]);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const [chapterRows, obligationRows] = await Promise.all([
        listChapters(projectRoot),
        listNarrativeObligations(projectRoot),
      ]);
      setChapters(chapterRows);
      setObligations(obligationRows);
    } catch (err) {
      setError(err instanceof Error ? err.message : "叙事义务加载失败");
    } finally {
      setLoading(false);
    }
  }, [projectRoot]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  async function handleCreate() {
    if (!projectRoot) return;
    if (!form.description.trim()) {
      setError("义务描述不能为空");
      return;
    }

    setCreating(true);
    setError(null);
    setMessage(null);
    try {
      await createNarrativeObligation(projectRoot, {
        obligationType: form.obligationType,
        description: form.description.trim(),
        plantedChapterId: form.plantedChapterId || undefined,
        expectedPayoffChapterId: form.expectedPayoffChapterId || undefined,
        actualPayoffChapterId: form.actualPayoffChapterId || undefined,
        payoffStatus: form.payoffStatus,
        severity: form.severity,
        relatedEntities: form.relatedEntities.trim() || undefined,
      });
      setForm(INITIAL_FORM);
      await loadData();
      setMessage("叙事义务已创建");
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建叙事义务失败");
    } finally {
      setCreating(false);
    }
  }

  async function handleStatusChange(item: NarrativeObligation, status: string) {
    if (!projectRoot || status === item.payoffStatus) return;
    setStatusUpdatingId(item.id);
    setError(null);
    setMessage(null);
    try {
      await updateObligationStatus(projectRoot, item.id, status);
      await loadData();
      setMessage("状态已更新");
    } catch (err) {
      setError(err instanceof Error ? err.message : "状态更新失败");
    } finally {
      setStatusUpdatingId(null);
    }
  }

  async function handleDelete(item: NarrativeObligation) {
    if (!projectRoot) return;
    if (!window.confirm(`确定删除义务「${item.description.slice(0, 24)}」吗？`)) {
      return;
    }

    setDeletingId(item.id);
    setError(null);
    setMessage(null);
    try {
      await deleteNarrativeObligation(projectRoot, item.id);
      await loadData();
      setMessage("叙事义务已删除");
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除叙事义务失败");
    } finally {
      setDeletingId(null);
    }
  }

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-surface-100">叙事义务</h1>
        <Button variant="secondary" size="sm" onClick={() => void loadData()} loading={loading}>
          刷新
        </Button>
      </div>

      <Card padding="lg" className="space-y-4">
        <h2 className="text-base font-semibold text-surface-100">新建义务</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Select
            label="义务类型"
            value={form.obligationType}
            onChange={(e) => setForm((prev) => ({ ...prev, obligationType: e.target.value }))}
            options={OBLIGATION_TYPES}
          />
          <Select
            label="严重级别"
            value={form.severity}
            onChange={(e) => setForm((prev) => ({ ...prev, severity: e.target.value }))}
            options={SEVERITY_OPTIONS}
          />
          <Select
            label="埋点章节"
            value={form.plantedChapterId}
            onChange={(e) => setForm((prev) => ({ ...prev, plantedChapterId: e.target.value }))}
            options={chapterOptions}
          />
          <Select
            label="预期兑现章节"
            value={form.expectedPayoffChapterId}
            onChange={(e) => setForm((prev) => ({ ...prev, expectedPayoffChapterId: e.target.value }))}
            options={chapterOptions}
          />
          <Select
            label="实际兑现章节"
            value={form.actualPayoffChapterId}
            onChange={(e) => setForm((prev) => ({ ...prev, actualPayoffChapterId: e.target.value }))}
            options={chapterOptions}
          />
          <Select
            label="当前状态"
            value={form.payoffStatus}
            onChange={(e) => setForm((prev) => ({ ...prev, payoffStatus: e.target.value }))}
            options={STATUS_OPTIONS}
          />
        </div>
        <Textarea
          label="义务描述 *"
          value={form.description}
          onChange={(e) => setForm((prev) => ({ ...prev, description: e.target.value }))}
          placeholder="例如：第一章出现的银钥匙必须在第十章揭示其来历"
        />
        <Input
          label="关联实体（可选）"
          value={form.relatedEntities}
          onChange={(e) => setForm((prev) => ({ ...prev, relatedEntities: e.target.value }))}
          placeholder='例如 ["主角","银钥匙"]'
        />
        <div className="pt-2 border-t border-surface-700 flex justify-end">
          <Button variant="primary" onClick={() => void handleCreate()} loading={creating} disabled={!projectRoot}>
            创建义务
          </Button>
        </div>
      </Card>

      {(message || error) && (
        <Card
          padding="sm"
          className={error ? "border border-error/30 bg-error/10 text-error" : "border border-success/30 bg-success/10 text-success"}
        >
          <p className="text-sm">{error ?? message}</p>
        </Card>
      )}

      <Card padding="lg">
        <h2 className="text-base font-semibold text-surface-100 mb-4">义务列表</h2>
        {loading ? (
          <p className="text-sm text-surface-400">加载中...</p>
        ) : obligations.length === 0 ? (
          <p className="text-sm text-surface-400">暂无叙事义务，先创建一条用于跟踪伏笔与兑现。</p>
        ) : (
          <div className="space-y-3">
            {obligations.map((item) => (
              <div key={item.id} className="rounded-lg border border-surface-700 bg-surface-800/50 p-4">
                <div className="flex items-start justify-between gap-4">
                  <div className="space-y-1">
                    <div className="text-sm text-surface-200">{item.description}</div>
                    <div className="text-xs text-surface-500">
                      类型: {item.obligationType} · 严重级别: {item.severity}
                    </div>
                    <div className="text-xs text-surface-500">
                      埋点: {item.plantedChapterId ? chapterLabelMap[item.plantedChapterId] ?? item.plantedChapterId : "未关联"}
                      {" · "}
                      预期兑现: {item.expectedPayoffChapterId ? chapterLabelMap[item.expectedPayoffChapterId] ?? item.expectedPayoffChapterId : "未关联"}
                      {" · "}
                      实际兑现: {item.actualPayoffChapterId ? chapterLabelMap[item.actualPayoffChapterId] ?? item.actualPayoffChapterId : "未关联"}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Select
                      value={item.payoffStatus}
                      onChange={(e) => void handleStatusChange(item, e.target.value)}
                      options={STATUS_OPTIONS}
                      className="min-w-[130px]"
                      disabled={statusUpdatingId === item.id}
                    />
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => void handleDelete(item)}
                      loading={deletingId === item.id}
                    >
                      删除
                    </Button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
