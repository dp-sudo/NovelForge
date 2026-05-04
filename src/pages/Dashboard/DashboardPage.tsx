import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore.js";
import { useUiStore } from "../../stores/uiStore.js";
import { Card } from "../../components/cards/Card.js";
import { Badge } from "../../components/ui/Badge.js";
import { Button } from "../../components/ui/Button.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { useQuery } from "@tanstack/react-query";
import { getDashboardStats } from "../../api/statsApi.js";
import { useAiTask } from "../../hooks/useAiTask.js";
import { runModuleAiTaskWithMeta } from "../../api/moduleAiApi.js";
import { listReviewWorkItems } from "../../api/contextApi.js";
import { useReviewChecklist } from "../../hooks/useReviewChecklist.js";

export function DashboardPage() {
  const project = useProjectStore((s) => s.currentProject);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const { data: stats, isLoading } = useQuery({
    queryKey: ["project", "stats", projectRoot ?? ""],
    queryFn: () => getDashboardStats(projectRoot!),
    enabled: !!projectRoot,
  });
  const [showAiReview, setShowAiReview] = useState(false);
  const [aiPrompt, setAiPrompt] = useState("");
  const ai = useAiTask();
  const [reviewPendingCount, setReviewPendingCount] = useState(0);
  const { reviewChecklistHints, reviewWorkItemCount, taskContractHint, processReviewResult, resetReview } = useReviewChecklist();

  const blueprintProgress = stats
    ? Math.round((stats.completedBlueprintCount / (stats.totalBlueprintSteps > 0 ? stats.totalBlueprintSteps : 8)) * 100)
    : 0;

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
  }, [projectRoot, ai.result]);

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
            ai.reset();
            setAiPrompt("");
            resetReview();
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
            <span className="text-surface-200">{blueprintProgress}%</span>
          </div>
          <div className="w-full bg-surface-700 rounded-full h-2">
            <div
              className="bg-primary h-2 rounded-full transition-all duration-500"
              style={{ width: `${blueprintProgress}%` }}
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

      {stats && !isLoading && (
        <Card padding="lg" className="mb-6">
          <h2 className="text-sm font-semibold text-surface-200 mb-3">📋 创作引导 — 下一步建议</h2>
          <div className="space-y-2">
            {(() => {
              type SR = typeof shortcuts[0]["route"];
              const steps: Array<{ done: boolean; label: string; action: string; route: SR }> = [
                { done: blueprintProgress >= 50, label: "完成创作蓝图", action: "规划故事大纲和结构，让 AI 有方向可循", route: "blueprint" as SR },
                { done: stats.characterCount > 0, label: "创建主要角色", action: "定义角色身份、动机与缺陷，AI 生成时自动引用", route: "characters" as SR },
                { done: stats.worldRuleCount > 0, label: "建立世界设定", action: "定义世界规则和约束，确保叙事一致性", route: "world" as SR },
                { done: stats.plotNodeCount > 0, label: "搭建剧情骨架", action: "规划情节节点和冲突，控制故事节奏", route: "plot" as SR },
                { done: stats.chapterCount > 0, label: "开始生成章节", action: "AI 在宪法+资产+状态约束下生成正文", route: "chapters" as SR },
                { done: reviewPendingCount === 0 && stats.chapterCount > 0, label: "完成审查闭环", action: "审核 AI 产出，通过后正式落库", route: "review-board" as SR },
              ];
              const nextStep = steps.find((s) => !s.done);
              const completedCount = steps.filter((s) => s.done).length;
              return (
                <>
                  <div className="flex items-center gap-2 mb-3">
                    <div className="flex-1 bg-surface-700 rounded-full h-1.5">
                      <div className="bg-primary h-1.5 rounded-full transition-all duration-500" style={{ width: `${Math.round((completedCount / steps.length) * 100)}%` }} />
                    </div>
                    <span className="text-xs text-surface-400 shrink-0">{completedCount}/{steps.length}</span>
                  </div>
                  {steps.map((step, idx) => (
                    <button key={step.label} onClick={() => setActiveRoute(step.route)} className={`w-full flex items-center gap-3 p-3 rounded-lg text-left transition-colors border ${!step.done && step === nextStep ? "bg-primary/10 border-primary/30 ring-1 ring-primary/20" : step.done ? "bg-surface-800/50 border-surface-700/50" : "bg-surface-800 border-surface-700 hover:border-surface-500"}`}>
                      <span className={`text-base shrink-0 ${step.done ? "opacity-50" : ""}`}>{step.done ? "✅" : step === nextStep ? "👉" : `${idx + 1}`}</span>
                      <div className="flex-1 min-w-0">
                        <p className={`text-sm font-medium ${step.done ? "text-surface-500 line-through" : "text-surface-100"}`}>{step.label}</p>
                        <p className={`text-xs mt-0.5 ${step.done ? "text-surface-600" : "text-surface-400"}`}>{step.action}</p>
                      </div>
                      {!step.done && step === nextStep && <Badge variant="info">推荐</Badge>}
                    </button>
                  ))}
                </>
              );
            })()}
          </div>
        </Card>
      )}

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
            loading={ai.loading}
            onClick={async () => {
              if (!projectRoot) return;
              await ai.run(async () => {
                const result = await runModuleAiTaskWithMeta({
                  projectRoot,
                  taskType: "dashboard.review",
                  uiAction: "dashboard.ai.review",
                  userInstruction: aiPrompt,
                });
                processReviewResult(result);
                const pending = await listReviewWorkItems(projectRoot, {
                  taskType: "dashboard.review",
                  status: "pending",
                  limit: 200,
                });
                setReviewPendingCount(pending.length);
                return result.output || "AI 未返回内容。";
              });
            }}
            disabled={!projectRoot}
          >
            {ai.loading ? "分析中..." : "生成诊断"}
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
                <div className="mt-3 flex items-center gap-2">
                  <span className="text-xs text-surface-400">已生成 {reviewWorkItemCount} 条审查工单</span>
                  <button
                    onClick={() => { setShowAiReview(false); setActiveRoute("review-board" as never); }}
                    className="text-xs text-primary hover:text-primary-light underline"
                  >
                    前往审查看板 →
                  </button>
                </div>
              )}
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
}
