import { useEffect, useMemo, useState } from "react";
import { Card } from "../../components/cards/Card";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/dialogs/Modal";
import { Textarea } from "../../components/forms/Textarea";
import { listTimelineEntries, type TimelineEntry } from "../../api/timelineApi";
import { runModuleAiTaskWithMeta, type ModuleReviewWorkItem } from "../../api/moduleAiApi";
import { listReviewWorkItems, updateReviewQueueItemStatus } from "../../api/contextApi";
import { useProjectStore } from "../../stores/projectStore";

const STATUS_TEXT: Record<string, string> = {
  drafting: "写作中",
  completed: "已完成",
  revising: "待修订"
};

export function TimelinePage() {
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const [entries, setEntries] = useState<TimelineEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [descending, setDescending] = useState(false);
  const [showAiReview, setShowAiReview] = useState(false);
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiResult, setAiResult] = useState<string | null>(null);
  const [aiError, setAiError] = useState<string | null>(null);
  const [aiLoading, setAiLoading] = useState(false);
  const [reviewPendingCount, setReviewPendingCount] = useState(0);
  const [reviewChecklistHints, setReviewChecklistHints] = useState<string[]>([]);
  const [reviewWorkItems, setReviewWorkItems] = useState<ModuleReviewWorkItem[]>([]);
  const [reviewItemUpdating, setReviewItemUpdating] = useState<Record<string, boolean>>({});
  const [taskContractHint, setTaskContractHint] = useState<string | null>(null);

  useEffect(() => {
    if (!projectRoot) {
      setEntries([]);
      return;
    }

    setLoading(true);
    setError(null);
    listTimelineEntries(projectRoot)
      .then((rows) => setEntries(rows))
      .catch((err) => {
        setError(err instanceof Error ? err.message : "加载时间线失败");
        setEntries([]);
      })
      .finally(() => setLoading(false));
  }, [projectRoot]);

  useEffect(() => {
    if (!projectRoot) {
      setReviewPendingCount(0);
      return;
    }
    void listReviewWorkItems(projectRoot, {
      taskType: "timeline.review",
      status: "pending",
      limit: 200,
    }).then((items) => setReviewPendingCount(items.length)).catch(() => setReviewPendingCount(0));
  }, [projectRoot, aiResult]);

  const sortedEntries = useMemo(() => {
    const rows = [...entries];
    rows.sort((a, b) =>
      descending ? b.chapterIndex - a.chapterIndex : a.chapterIndex - b.chapterIndex
    );
    return rows;
  }, [entries, descending]);

  return (
    <div className="max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-surface-100">时间线</h1>
          <p className="text-sm text-surface-400 mt-1">按章节顺序浏览全书推进脉络</p>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-surface-500">待办审查 {reviewPendingCount}</span>
          <Button variant="ghost" size="sm" onClick={() => { setShowAiReview(true); setAiError(null); setAiResult(null); }}>
            AI 审阅
          </Button>
          <Button variant="secondary" size="sm" onClick={() => setDescending((v) => !v)}>
            {descending ? "切换正序" : "切换倒序"}
          </Button>
        </div>
      </div>

      {loading ? (
        <Card padding="lg">
          <p className="text-sm text-surface-400">加载中...</p>
        </Card>
      ) : error ? (
        <Card padding="lg" className="border-error/30 bg-error/5">
          <p className="text-sm text-error">{error}</p>
        </Card>
      ) : sortedEntries.length === 0 ? (
        <Card padding="lg">
          <p className="text-sm text-surface-400">暂无章节时间线数据</p>
        </Card>
      ) : (
        <div className="relative pl-8">
          <div className="absolute left-3.5 top-1 bottom-1 w-px bg-surface-700" />
          <div className="space-y-4">
            {sortedEntries.map((entry) => (
              <div key={entry.chapterId} className="relative">
                <span className="absolute -left-[1.45rem] top-6 w-3 h-3 rounded-full bg-primary border border-primary/40" />
                <Card padding="md" className="space-y-2">
                  <div className="flex items-center justify-between gap-3">
                    <h2 className="text-sm font-semibold text-surface-100">
                      #{entry.chapterIndex} {entry.title}
                    </h2>
                    <span className="text-xs text-surface-400">
                      {entry.volumeTitle ? `卷：${entry.volumeTitle}` : "未分卷"}
                    </span>
                  </div>
                  <p className="text-sm text-surface-300 whitespace-pre-wrap break-words">
                    {entry.summary || "暂无章节摘要"}
                  </p>
                  <div className="flex items-center justify-between text-xs text-surface-500">
                    <span>{STATUS_TEXT[entry.status] ?? entry.status}</span>
                    <span>更新时间：{new Date(entry.updatedAt).toLocaleString("zh-CN")}</span>
                  </div>
                </Card>
              </div>
            ))}
          </div>
        </div>
      )}

      <Modal open={showAiReview} onClose={() => setShowAiReview(false)} title="AI 时间线审阅" width="lg">
        <div className="space-y-4">
          <Textarea
            label="附加要求（可选）"
            value={aiPrompt}
            onChange={(e) => setAiPrompt(e.target.value)}
            placeholder="例如：重点检查人物年龄线和事件先后关系"
            className="min-h-[90px]"
          />
          <Button
            variant="primary"
            loading={aiLoading}
            onClick={async () => {
              if (!projectRoot) return;
              setAiLoading(true);
              setAiError(null);
              setAiResult(null);
              try {
                const result = await runModuleAiTaskWithMeta({
                  projectRoot,
                  taskType: "timeline.review",
                  uiAction: "timeline.ai.review",
                  userInstruction: aiPrompt,
                });
                setAiResult(result.output || "AI 未返回内容。");
                const hints = result.reviewChecklist
                  .filter((item) => item.status === "attention")
                  .map((item) => `${item.title}: ${item.message}`);
                setReviewChecklistHints(hints);
                setReviewWorkItems(result.reviewWorkItems);
                const contract = result.taskContract;
                if (contract) {
                  const authorityLayer = typeof contract.authorityLayer === "string" ? contract.authorityLayer : "n/a";
                  const stateLayer = typeof contract.stateLayer === "string" ? contract.stateLayer : "n/a";
                  const capabilityPack = typeof contract.capabilityPack === "string" ? contract.capabilityPack : "n/a";
                  const reviewGate = typeof contract.reviewGate === "string" ? contract.reviewGate : "n/a";
                  setTaskContractHint(`权威层: ${authorityLayer} | 状态层: ${stateLayer} | 能力包: ${capabilityPack} | 审查门: ${reviewGate}`);
                } else {
                  setTaskContractHint(null);
                }
                const pending = await listReviewWorkItems(projectRoot, {
                  taskType: "timeline.review",
                  status: "pending",
                  limit: 200,
                });
                setReviewPendingCount(pending.length);
              } catch (err) {
                setAiError(err instanceof Error ? err.message : "AI 审阅失败");
              } finally {
                setAiLoading(false);
              }
            }}
            disabled={!projectRoot}
          >
            {aiLoading ? "审阅中..." : "生成审阅报告"}
          </Button>
          {aiError && (
            <div className="p-3 rounded-lg bg-error/10 border border-error/30 text-sm text-error">
              {aiError}
            </div>
          )}
          {aiResult && (
            <div className="p-4 rounded-xl bg-primary/5 border border-primary/20">
              <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed max-h-80 overflow-y-auto">{aiResult}</pre>
              {taskContractHint && <div className="mt-3 text-xs text-surface-400">{taskContractHint}</div>}
              {reviewChecklistHints.length > 0 && (
                <div className="mt-3 space-y-1">
                  {reviewChecklistHints.map((hint, idx) => (
                    <div key={`${hint}-${idx}`} className="text-xs text-warning">{hint}</div>
                  ))}
                </div>
              )}
              {reviewWorkItems.length > 0 && (
                <div className="mt-3 space-y-2">
                  <div className="text-xs text-surface-400">审查工单</div>
                  {reviewWorkItems.map((item) => (
                    <div key={item.id} className="p-2 rounded-lg bg-surface-800 border border-surface-700">
                      <div className="text-xs text-surface-200">{item.title}</div>
                      <div className="text-[11px] text-surface-400 mt-1">{item.message}</div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        <button
                          onClick={async () => {
                            if (!projectRoot) return;
                            setReviewItemUpdating((prev) => ({ ...prev, [item.id]: true }));
                            try {
                              await updateReviewQueueItemStatus(projectRoot, item.id, "resolved");
                              setReviewWorkItems((prev) => prev.map((row) => row.id === item.id ? { ...row, status: "resolved" } : row));
                              const pending = await listReviewWorkItems(projectRoot, {
                                taskType: "timeline.review",
                                status: "pending",
                                limit: 200,
                              });
                              setReviewPendingCount(pending.length);
                            } finally {
                              setReviewItemUpdating((prev) => {
                                const next = { ...prev };
                                delete next[item.id];
                                return next;
                              });
                            }
                          }}
                          disabled={Boolean(reviewItemUpdating[item.id])}
                          className="px-2 py-1 text-[11px] bg-surface-700 text-surface-100 rounded border border-surface-600 disabled:opacity-40"
                        >
                          已处理
                        </button>
                        <button
                          onClick={async () => {
                            if (!projectRoot) return;
                            setReviewItemUpdating((prev) => ({ ...prev, [item.id]: true }));
                            try {
                              await updateReviewQueueItemStatus(projectRoot, item.id, "rejected");
                              setReviewWorkItems((prev) => prev.map((row) => row.id === item.id ? { ...row, status: "rejected" } : row));
                              const pending = await listReviewWorkItems(projectRoot, {
                                taskType: "timeline.review",
                                status: "pending",
                                limit: 200,
                              });
                              setReviewPendingCount(pending.length);
                            } finally {
                              setReviewItemUpdating((prev) => {
                                const next = { ...prev };
                                delete next[item.id];
                                return next;
                              });
                            }
                          }}
                          disabled={Boolean(reviewItemUpdating[item.id])}
                          className="px-2 py-1 text-[11px] bg-surface-700 text-warning rounded border border-surface-600 disabled:opacity-40"
                        >
                          驳回
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
}
