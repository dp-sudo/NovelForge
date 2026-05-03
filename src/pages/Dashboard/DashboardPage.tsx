import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore.js";
import { useUiStore } from "../../stores/uiStore.js";
import { Card } from "../../components/cards/Card.js";
import { Badge } from "../../components/ui/Badge.js";
import { Button } from "../../components/ui/Button.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { useDashboardStats } from "../../hooks/useApi.js";
import { runModuleAiTaskWithMeta, type ModuleReviewWorkItem } from "../../api/moduleAiApi.js";
import { listReviewWorkItems, updateReviewQueueItemStatus } from "../../api/contextApi.js";

export function DashboardPage() {
  const project = useProjectStore((s) => s.currentProject);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const { data: stats, isLoading } = useDashboardStats(projectRoot);
  const [showAiReview, setShowAiReview] = useState(false);
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiResult, setAiResult] = useState<string | null>(null);
  const [aiLoading, setAiLoading] = useState(false);
  const [aiError, setAiError] = useState<string | null>(null);
  const [reviewPendingCount, setReviewPendingCount] = useState(0);
  const [reviewChecklistHints, setReviewChecklistHints] = useState<string[]>([]);
  const [reviewWorkItems, setReviewWorkItems] = useState<ModuleReviewWorkItem[]>([]);
  const [reviewItemUpdating, setReviewItemUpdating] = useState<Record<string, boolean>>({});
  const [taskContractHint, setTaskContractHint] = useState<string | null>(null);

  const statCards = [
    { label: "总字数", value: stats?.totalWords.toLocaleString() ?? "0", color: "text-info" },
    { label: "章节数", value: stats?.chapterCount ?? 0, color: "text-success" },
    { label: "角色数", value: stats?.characterCount ?? 0, color: "text-primary" },
    { label: "设定数", value: stats?.worldRuleCount ?? 0, color: "text-warning" },
    { label: "剧情节点", value: stats?.plotNodeCount ?? 0, color: "text-surface-200" },
    { label: "未解决问题", value: stats?.openIssueCount ?? 0, color: "text-error" },
  ];

  const shortcuts = [
    { label: "继续写作", route: "chapters" as const, icon: "✍️" },
    { label: "完成蓝图", route: "blueprint" as const, icon: "📐" },
    { label: "创建角色", route: "characters" as const, icon: "👤" },
    { label: "创建章节", route: "chapters" as const, icon: "📑" },
    { label: "运行检查", route: "consistency" as const, icon: "🔍" },
    { label: "导出作品", route: "export" as const, icon: "📤" },
  ];

  useEffect(() => {
    if (!projectRoot) {
      setReviewPendingCount(0);
      return;
    }
    void listReviewWorkItems(projectRoot, {
      taskType: "dashboard.review",
      status: "pending",
      limit: 200,
    }).then((items) => setReviewPendingCount(items.length)).catch(() => setReviewPendingCount(0));
  }, [projectRoot, aiResult]);

  return (
    <div className="max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-1">
        <h1 className="text-2xl font-bold text-surface-100">
          {project?.name ?? "项目仪表盘"}
        </h1>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setAiError(null);
            setAiResult(null);
            setAiPrompt("");
            setShowAiReview(true);
          }}
        >
          AI 诊断（待办 {reviewPendingCount}）
        </Button>
      </div>
      <p className="text-sm text-surface-400 mb-6">
        {project?.genre ? `类型: ${project.genre}` : ""}
        {project?.targetWords
          ? ` · 目标: ${project.targetWords.toLocaleString()} 字`
          : ""}
      </p>

      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8 stagger-children">
        {statCards.map((card) => (
          <Card key={card.label} padding="md" className="text-center">
            <div className={`text-2xl font-bold ${card.color}`}>{isLoading ? "..." : card.value}</div>
            <div className="text-xs text-surface-400 mt-1">{card.label}</div>
          </Card>
        ))}
      </div>

      <Card padding="lg" className="mb-6">
        <h2 className="text-sm font-semibold text-surface-200 mb-3">创作进度</h2>
        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span className="text-surface-400">蓝图完成度</span>
            <span className="text-surface-200">{stats?.blueprintProgress ?? 0}%</span>
          </div>
          <div className="w-full bg-surface-700 rounded-full h-2">
            <div
              className="bg-primary h-2 rounded-full transition-all duration-500"
              style={{ width: `${stats?.blueprintProgress ?? 0}%` }}
            />
          </div>
        </div>
        <div className="grid grid-cols-3 gap-4 mt-4 text-sm">
          <div>
            <span className="text-surface-400">已完成章节</span>
            <p className="text-success font-semibold">{stats?.completedChapterCount ?? stats?.chapterCount ?? 0}</p>
          </div>
          <div>
            <span className="text-surface-400">未解决问题</span>
            <p className="text-error font-semibold">{stats?.openIssueCount ?? 0}</p>
          </div>
          <div>
            <span className="text-surface-400">目标字数</span>
            <p className="text-surface-200 font-semibold">{project?.targetWords?.toLocaleString() ?? 0}</p>
          </div>
        </div>
      </Card>

      <h2 className="text-sm font-semibold text-surface-200 mb-3">快捷操作</h2>
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        {shortcuts.map((s) => (
          <button
            key={s.label}
            onClick={() => setActiveRoute(s.route)}
            className="flex flex-col items-center gap-2 p-4 bg-surface-800 border border-surface-700 rounded-xl hover:border-surface-500 hover:bg-surface-700 transition-colors"
          >
            <span className="text-2xl">{s.icon}</span>
            <span className="text-xs text-surface-300">{s.label}</span>
          </button>
        ))}
      </div>

      {stats?.recentChapters && stats.recentChapters.length > 0 && (
        <Card padding="lg" className="mt-6">
          <h2 className="text-sm font-semibold text-surface-200 mb-3">最近编辑</h2>
          <div className="space-y-2">
            {stats.recentChapters.map((ch) => (
              <div key={ch.id} className="flex items-center justify-between py-1.5">
                <span className="text-sm text-surface-200">{ch.title}</span>
                <span className="text-xs text-surface-400">{ch.updatedAt.slice(0, 10)}</span>
              </div>
            ))}
          </div>
        </Card>
      )}

      <Modal open={showAiReview} onClose={() => setShowAiReview(false)} title="AI 仪表盘诊断" width="lg">
        <div className="space-y-4">
          <Textarea
            label="附加要求（可选）"
            value={aiPrompt}
            onChange={(e) => setAiPrompt(e.target.value)}
            placeholder="例如：更关注本周应该优先推进的两个任务"
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
                  taskType: "dashboard.review",
                  uiAction: "dashboard.ai.review",
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
                  taskType: "dashboard.review",
                  status: "pending",
                  limit: 200,
                });
                setReviewPendingCount(pending.length);
              } catch (error) {
                setAiError(error instanceof Error ? error.message : "AI 诊断失败");
              } finally {
                setAiLoading(false);
              }
            }}
            disabled={!projectRoot}
          >
            {aiLoading ? "分析中..." : "生成诊断"}
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
                                taskType: "dashboard.review",
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
                                taskType: "dashboard.review",
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
