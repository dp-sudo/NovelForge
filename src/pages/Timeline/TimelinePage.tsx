import { useEffect, useMemo, useState } from "react";
import { Card } from "../../components/cards/Card";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/dialogs/Modal";
import { Textarea } from "../../components/forms/Textarea";
import { useAiTask } from "../../hooks/useAiTask.js";
import { listTimelineEntries, type TimelineEntry } from "../../api/timelineApi";
import { runModuleAiTaskWithMeta } from "../../api/moduleAiApi";
import { listReviewWorkItems } from "../../api/contextApi";
import { useProjectStore } from "../../stores/projectStore";
import { useReviewChecklist } from "../../hooks/useReviewChecklist.js";

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
  const ai = useAiTask();
  const [reviewPendingCount, setReviewPendingCount] = useState(0);
  const { reviewChecklistHints, reviewWorkItemCount, taskContractHint, processReviewResult, resetReview } = useReviewChecklist();

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
  }, [projectRoot, ai.result]);

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
          <Button variant="ghost" size="sm" onClick={() => { setShowAiReview(true); ai.reset(); resetReview(); }}>
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
            loading={ai.loading}
            onClick={async () => {
              if (!projectRoot) return;
              await ai.run(async () => {
                const result = await runModuleAiTaskWithMeta({
                  projectRoot,
                  taskType: "timeline.review",
                  uiAction: "timeline.ai.review",
                  userInstruction: aiPrompt,
                });
                processReviewResult(result);
                const pending = await listReviewWorkItems(projectRoot, {
                  taskType: "timeline.review",
                  status: "pending",
                  limit: 200,
                });
                setReviewPendingCount(pending.length);
                return result.output || "AI 未返回内容。";
              });
            }}
            disabled={!projectRoot}
          >
            {ai.loading ? "审阅中..." : "生成审阅报告"}
          </Button>
          {ai.error && (
            <div className="p-3 rounded-lg bg-error/10 border border-error/30 text-sm text-error">
              {ai.error}
            </div>
          )}
          {ai.result && (
            <div className="p-4 rounded-xl bg-primary/5 border border-primary/20">
              <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed max-h-80 overflow-y-auto">{ai.result}</pre>
              {taskContractHint && <div className="mt-3 text-xs text-surface-400">{taskContractHint}</div>}
              {reviewChecklistHints.length > 0 && (
                <div className="mt-3 space-y-1">
                  {reviewChecklistHints.map((hint, idx) => (
                    <div key={`${hint}-${idx}`} className="text-xs text-warning">{hint}</div>
                  ))}
                </div>
              )}
              {reviewWorkItemCount > 0 && (
                <div className="mt-3 text-xs text-surface-400">
                  已生成 {reviewWorkItemCount} 条审查工单，请前往审查看板处理。
                </div>
              )}
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
}
