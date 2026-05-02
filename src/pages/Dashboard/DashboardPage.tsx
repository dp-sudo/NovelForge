import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore.js";
import { useUiStore } from "../../stores/uiStore.js";
import { Card } from "../../components/cards/Card.js";
import { Badge } from "../../components/ui/Badge.js";
import { Button } from "../../components/ui/Button.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { useDashboardStats } from "../../hooks/useApi.js";
import { runModuleAiTask } from "../../api/moduleAiApi.js";
import {
  acknowledgeFeedbackEvent,
  getFeedbackEvents,
  ignoreFeedbackEvent,
  resolveFeedbackEvent,
  type FeedbackEvent,
} from "../../api/statsApi.js";

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
  const [feedbackEvents, setFeedbackEvents] = useState<FeedbackEvent[]>([]);
  const [feedbackActionError, setFeedbackActionError] = useState<string | null>(null);
  const [feedbackUpdatingId, setFeedbackUpdatingId] = useState<string | null>(null);
  const [showResolvedIgnored, setShowResolvedIgnored] = useState(false);
  const [feedbackActionTarget, setFeedbackActionTarget] = useState<{
    eventId: string;
    mode: "resolve" | "ignore";
  } | null>(null);
  const [feedbackActionNote, setFeedbackActionNote] = useState("");

  async function loadFeedbackEvents() {
    if (!projectRoot) {
      setFeedbackEvents([]);
      return;
    }
    const events = await getFeedbackEvents(projectRoot);
    setFeedbackEvents(events);
  }

  useEffect(() => {
    let cancelled = false;
    if (!projectRoot) {
      setFeedbackEvents([]);
      return () => {
        cancelled = true;
      };
    }
    void getFeedbackEvents(projectRoot)
      .then((events) => {
        if (!cancelled) {
          setFeedbackEvents(events);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFeedbackEvents([]);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [projectRoot]);

  const openEvents = feedbackEvents.filter((event) => event.status === "open");
  const acknowledgedEvents = feedbackEvents.filter((event) => event.status === "acknowledged");
  const resolvedEvents = feedbackEvents.filter((event) => event.status === "resolved");
  const ignoredEvents = feedbackEvents.filter((event) => event.status === "ignored");

  async function handleAcknowledgeFeedbackEvent(eventId: string) {
    if (!projectRoot) return;
    setFeedbackActionError(null);
    setFeedbackUpdatingId(eventId);
    try {
      await acknowledgeFeedbackEvent(projectRoot, eventId);
      await loadFeedbackEvents();
    } catch (error) {
      setFeedbackActionError(error instanceof Error ? error.message : "确认回报事件失败");
    } finally {
      setFeedbackUpdatingId(null);
    }
  }

  async function handleSubmitFeedbackAction() {
    if (!projectRoot || !feedbackActionTarget) return;
    const note = feedbackActionNote.trim();
    if (!note) {
      setFeedbackActionError(feedbackActionTarget.mode === "resolve" ? "请填写解决备注" : "请填写忽略原因");
      return;
    }
    setFeedbackActionError(null);
    setFeedbackUpdatingId(feedbackActionTarget.eventId);
    try {
      if (feedbackActionTarget.mode === "resolve") {
        await resolveFeedbackEvent(projectRoot, feedbackActionTarget.eventId, note);
      } else {
        await ignoreFeedbackEvent(projectRoot, feedbackActionTarget.eventId, note);
      }
      setFeedbackActionTarget(null);
      setFeedbackActionNote("");
      await loadFeedbackEvents();
    } catch (error) {
      setFeedbackActionError(
        error instanceof Error
          ? error.message
          : feedbackActionTarget.mode === "resolve"
            ? "解决回报事件失败"
            : "忽略回报事件失败",
      );
    } finally {
      setFeedbackUpdatingId(null);
    }
  }

  function renderFeedbackEvent(event: FeedbackEvent, options?: { actionable?: boolean }) {
    const actionable = options?.actionable ?? false;
    const isUpdating = feedbackUpdatingId === event.id;
    return (
      <div key={event.id} className="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2">
        <div className="text-xs font-medium text-warning">
          {event.ruleType} · {event.severity}
        </div>
        <div className="mt-1 text-xs text-surface-200">{event.conditionSummary}</div>
        {event.suggestedAction && (
          <div className="mt-1 text-xs text-surface-400">建议：{event.suggestedAction}</div>
        )}
        {event.resolutionNote && (
          <div className="mt-1 text-xs text-surface-400">处理记录：{event.resolutionNote}</div>
        )}
        {actionable && (
          <div className="mt-2 flex flex-wrap gap-2">
            {event.status === "open" && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => void handleAcknowledgeFeedbackEvent(event.id)}
                disabled={isUpdating}
              >
                {isUpdating ? "处理中..." : "确认"}
              </Button>
            )}
            <Button
              size="sm"
              variant="primary"
              onClick={() => {
                setFeedbackActionError(null);
                setFeedbackActionTarget({ eventId: event.id, mode: "resolve" });
                setFeedbackActionNote("");
              }}
              disabled={isUpdating}
            >
              解决
            </Button>
            <Button
              size="sm"
              variant="danger"
              onClick={() => {
                setFeedbackActionError(null);
                setFeedbackActionTarget({ eventId: event.id, mode: "ignore" });
                setFeedbackActionNote("");
              }}
              disabled={isUpdating}
            >
              忽略
            </Button>
          </div>
        )}
      </div>
    );
  }

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
          AI 诊断
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

      <Card padding="lg" className="mt-6">
        <h2 className="text-sm font-semibold text-surface-200 mb-3">回报事件</h2>
        {feedbackEvents.length === 0 ? (
          <p className="text-xs text-surface-500">暂无回报事件</p>
        ) : (
          <div className="space-y-4">
            <div>
              <div className="mb-2 flex items-center justify-between">
                <span className="text-xs text-surface-300">待处理（open）</span>
                <Badge variant="default">{openEvents.length}</Badge>
              </div>
              {openEvents.length === 0 ? (
                <p className="text-xs text-surface-500">无待处理事件</p>
              ) : (
                <div className="space-y-2">{openEvents.map((event) => renderFeedbackEvent(event, { actionable: true }))}</div>
              )}
            </div>
            <div>
              <div className="mb-2 flex items-center justify-between">
                <span className="text-xs text-surface-300">已确认（acknowledged）</span>
                <Badge variant="info">{acknowledgedEvents.length}</Badge>
              </div>
              {acknowledgedEvents.length === 0 ? (
                <p className="text-xs text-surface-500">无已确认事件</p>
              ) : (
                <div className="space-y-2">{acknowledgedEvents.map((event) => renderFeedbackEvent(event, { actionable: true }))}</div>
              )}
            </div>
            <div>
              <button
                type="button"
                className="text-xs text-primary hover:underline"
                onClick={() => setShowResolvedIgnored((prev) => !prev)}
              >
                {showResolvedIgnored ? "收起已解决/已忽略" : "展开已解决/已忽略"}
              </button>
              {showResolvedIgnored && (
                <div className="mt-2 space-y-3">
                  <div>
                    <div className="mb-1 flex items-center justify-between">
                      <span className="text-xs text-surface-300">已解决（resolved）</span>
                      <Badge variant="success">{resolvedEvents.length}</Badge>
                    </div>
                    {resolvedEvents.length === 0 ? (
                      <p className="text-xs text-surface-500">无已解决事件</p>
                    ) : (
                      <div className="space-y-2">{resolvedEvents.map((event) => renderFeedbackEvent(event))}</div>
                    )}
                  </div>
                  <div>
                    <div className="mb-1 flex items-center justify-between">
                      <span className="text-xs text-surface-300">已忽略（ignored）</span>
                      <Badge variant="warning">{ignoredEvents.length}</Badge>
                    </div>
                    {ignoredEvents.length === 0 ? (
                      <p className="text-xs text-surface-500">无已忽略事件</p>
                    ) : (
                      <div className="space-y-2">{ignoredEvents.map((event) => renderFeedbackEvent(event))}</div>
                    )}
                  </div>
                </div>
              )}
            </div>
            {feedbackActionError && (
              <div className="px-3 py-2 rounded-lg text-xs bg-error/10 text-error border border-error/20">
                {feedbackActionError}
              </div>
            )}
          </div>
        )}
      </Card>

      <Modal
        open={Boolean(feedbackActionTarget)}
        onClose={() => {
          setFeedbackActionTarget(null);
          setFeedbackActionNote("");
        }}
        title={feedbackActionTarget?.mode === "resolve" ? "解决回报事件" : "忽略回报事件"}
        width="md"
      >
        <div className="space-y-4">
          <Textarea
            label={feedbackActionTarget?.mode === "resolve" ? "解决备注" : "忽略原因"}
            value={feedbackActionNote}
            onChange={(e) => setFeedbackActionNote(e.target.value)}
            className="min-h-[100px]"
          />
          <div className="flex justify-end gap-2">
            <Button
              variant="ghost"
              onClick={() => {
                setFeedbackActionTarget(null);
                setFeedbackActionNote("");
              }}
            >
              取消
            </Button>
            <Button
              variant="primary"
              onClick={() => void handleSubmitFeedbackAction()}
              disabled={!feedbackActionTarget}
            >
              提交
            </Button>
          </div>
        </div>
      </Modal>

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
                const result = await runModuleAiTask({
                  projectRoot,
                  taskType: "dashboard.review",
                  uiAction: "dashboard.ai.review",
                  userInstruction: aiPrompt,
                  persistMode: "derived_review",
                  automationTier: "auto",
                });
                setAiResult(result || "AI 未返回内容。");
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
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
}
