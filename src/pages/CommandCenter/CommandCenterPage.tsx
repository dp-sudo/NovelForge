import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";

import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Badge } from "../../components/ui/Badge.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { useProjectStore } from "../../stores/projectStore.js";
import { useEditorStore } from "../../stores/editorStore.js";
import { useUiStore, type AppRoute } from "../../stores/uiStore.js";
import {
  getCommandCenterSnapshot,
  type CommandCenterSnapshot,
} from "../../api/commandCenterApi.js";
import { EditorPage } from "../Editor/EditorPage.js";
import { BlueprintPage } from "../Blueprint/BlueprintPage.js";
import { NarrativePage } from "../Narrative/NarrativePage.js";
import { ConsistencyPage } from "../Consistency/ConsistencyPage.js";
import { ChaptersPage } from "../Chapters/ChaptersPage.js";
import { selectCommandCenterChapter } from "./model.js";

function getErrorMessage(error: unknown, fallback: string): string {
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: string }).message);
  }
  return fallback;
}

function chapterStatusLabel(status: string): string {
  switch (status) {
    case "planned":
      return "规划中";
    case "drafting":
      return "写作中";
    case "revising":
      return "待修订";
    case "completed":
      return "已完成";
    case "archived":
      return "已归档";
    default:
      return status;
  }
}

function chapterStatusVariant(status: string): "default" | "success" | "warning" | "error" | "info" {
  switch (status) {
    case "completed":
      return "success";
    case "drafting":
      return "warning";
    case "revising":
      return "info";
    case "planned":
      return "default";
    default:
      return "default";
  }
}

function issueSeverityVariant(severity: string): "default" | "success" | "warning" | "error" | "info" {
  switch (severity) {
    case "blocker":
    case "high":
      return "error";
    case "medium":
      return "warning";
    case "low":
      return "info";
    default:
      return "default";
  }
}

function roleTypeLabel(character: { roleType?: string; role_type?: string }): string {
  return character.roleType || character.role_type || "角色";
}

function worldRuleConstraintLabel(rule: { constraintLevel?: string; constraint_level?: string }): string {
  const value = rule.constraintLevel || rule.constraint_level || "normal";
  if (value === "absolute") return "绝对";
  if (value === "strong") return "强约束";
  if (value === "weak") return "弱设定";
  return "普通";
}

function plotNodeTypeLabel(node: { nodeType?: string; node_type?: string }): string {
  return node.nodeType || node.node_type || "节点";
}

function reviewStatusLabel(status: string): string {
  switch (status) {
    case "open":
      return "待处理";
    case "acknowledged":
      return "已确认";
    default:
      return status;
  }
}

function AssetDrawer({
  open,
  snapshot,
  onClose,
  onNavigate,
}: {
  open: boolean;
  snapshot: CommandCenterSnapshot | null;
  onClose: () => void;
  onNavigate: (route: AppRoute) => void;
}) {
  if (!open || !snapshot) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/55" onClick={onClose}>
      <aside
        className="absolute right-0 top-0 h-full w-full max-w-[30rem] bg-surface-900 border-l border-surface-700 shadow-2xl overflow-y-auto"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="sticky top-0 z-10 flex items-center justify-between px-5 py-4 bg-surface-900/95 border-b border-surface-700">
          <div>
            <h2 className="text-lg font-semibold text-surface-100">正式资产抽屉</h2>
            <p className="text-xs text-surface-400 mt-1">主链内快速核对，必要时跳转深度整理页。</p>
          </div>
          <Button variant="ghost" size="sm" onClick={onClose}>关闭</Button>
        </div>

        <div className="p-5 space-y-4">
          <Card padding="md" className="space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-surface-100">角色</h3>
                <p className="text-xs text-surface-500">共 {snapshot.assetAuthority.characterCount} 条</p>
              </div>
              <Button variant="secondary" size="sm" onClick={() => onNavigate("characters")}>深度整理</Button>
            </div>
            <div className="space-y-2">
              {snapshot.assetAuthority.previewCharacters.length === 0 ? (
                <p className="text-xs text-surface-500">暂无角色</p>
              ) : (
                snapshot.assetAuthority.previewCharacters.map((character) => (
                  <div key={character.id} className="rounded-lg bg-surface-800 px-3 py-2">
                    <div className="text-sm text-surface-100">{character.name}</div>
                    <div className="text-xs text-surface-400">{roleTypeLabel(character)}</div>
                  </div>
                ))
              )}
            </div>
          </Card>

          <Card padding="md" className="space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-surface-100">设定</h3>
                <p className="text-xs text-surface-500">共 {snapshot.assetAuthority.worldRuleCount} 条</p>
              </div>
              <Button variant="secondary" size="sm" onClick={() => onNavigate("world")}>深度整理</Button>
            </div>
            <div className="space-y-2">
              {snapshot.assetAuthority.previewWorldRules.length === 0 ? (
                <p className="text-xs text-surface-500">暂无设定</p>
              ) : (
                snapshot.assetAuthority.previewWorldRules.map((rule) => (
                  <div key={rule.id} className="rounded-lg bg-surface-800 px-3 py-2">
                    <div className="text-sm text-surface-100">{rule.title}</div>
                    <div className="text-xs text-surface-400">
                      {rule.category} · {worldRuleConstraintLabel(rule)}
                    </div>
                  </div>
                ))
              )}
            </div>
          </Card>

          <Card padding="md" className="space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-surface-100">名词</h3>
                <p className="text-xs text-surface-500">共 {snapshot.assetAuthority.glossaryCount} 条</p>
              </div>
              <Button variant="secondary" size="sm" onClick={() => onNavigate("glossary")}>深度整理</Button>
            </div>
            <div className="space-y-2">
              {snapshot.assetAuthority.previewGlossary.length === 0 ? (
                <p className="text-xs text-surface-500">暂无名词</p>
              ) : (
                snapshot.assetAuthority.previewGlossary.map((term) => (
                  <div key={term.id} className="rounded-lg bg-surface-800 px-3 py-2">
                    <div className="text-sm text-surface-100">{term.term}</div>
                    <div className="text-xs text-surface-400">
                      {term.termType}
                      {term.locked ? " · 锁定" : ""}
                      {term.banned ? " · 禁用" : ""}
                    </div>
                  </div>
                ))
              )}
            </div>
          </Card>

          <Card padding="md" className="space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-surface-100">剧情</h3>
                <p className="text-xs text-surface-500">共 {snapshot.assetAuthority.plotNodeCount} 条</p>
              </div>
              <Button variant="secondary" size="sm" onClick={() => onNavigate("plot")}>深度整理</Button>
            </div>
            <div className="space-y-2">
              {snapshot.assetAuthority.previewPlotNodes.length === 0 ? (
                <p className="text-xs text-surface-500">暂无剧情节点</p>
              ) : (
                snapshot.assetAuthority.previewPlotNodes.map((node) => (
                  <div key={node.id} className="rounded-lg bg-surface-800 px-3 py-2">
                    <div className="text-sm text-surface-100">{node.title}</div>
                    <div className="text-xs text-surface-400">{plotNodeTypeLabel(node)}</div>
                  </div>
                ))
              )}
            </div>
          </Card>

          <div className="grid grid-cols-2 gap-3">
            <Button variant="ghost" size="sm" onClick={() => onNavigate("relationships")}>关系整理</Button>
            <Button variant="ghost" size="sm" onClick={() => onNavigate("timeline")}>时间线整理</Button>
          </div>
        </div>
      </aside>
    </div>
  );
}

export function CommandCenterPage() {
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const setStats = useProjectStore((s) => s.setStats);
  const activeChapterId = useEditorStore((s) => s.activeChapterId);
  const activeChapterTitle = useEditorStore((s) => s.activeChapterTitle);
  const setActiveChapter = useEditorStore((s) => s.setActiveChapter);
  const activeRoute = useUiStore((s) => s.activeRoute);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const [assetDrawerOpen, setAssetDrawerOpen] = useState(false);
  const showBlueprintPanel = activeRoute === "blueprint";
  const showNarrativePanel = activeRoute === "narrative";
  const showConsistencyPanel = activeRoute === "consistency";
  const showChapterPanel = activeRoute === "chapters";

  const {
    data: snapshot,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["command-center", projectRoot ?? "", activeChapterId ?? ""],
    queryFn: () => getCommandCenterSnapshot(projectRoot!, activeChapterId),
    enabled: Boolean(projectRoot),
  });

  useEffect(() => {
    if (!snapshot) {
      return;
    }
    setStats({
      totalWords: snapshot.stats.totalWords,
      chapterCount: snapshot.stats.chapterCount,
      characterCount: snapshot.stats.characterCount,
      worldRuleCount: snapshot.stats.worldRuleCount,
      plotNodeCount: snapshot.stats.plotNodeCount,
      openIssueCount: snapshot.stats.openIssueCount,
      blueprintProgress: snapshot.stats.blueprintProgress,
    });
  }, [snapshot, setStats]);

  useEffect(() => {
    if (!snapshot) {
      return;
    }
    const selected = selectCommandCenterChapter(
      snapshot.productionQueue.chapters,
      activeChapterId || snapshot.productionQueue.activeChapterId,
    );
    if (selected && selected.id !== activeChapterId) {
      setActiveChapter(selected.id, selected.title);
    }
  }, [snapshot, activeChapterId, setActiveChapter]);

  const selectedChapter = useMemo(() => {
    if (!snapshot) {
      return null;
    }
    return selectCommandCenterChapter(
      snapshot.productionQueue.chapters,
      activeChapterId || snapshot.productionQueue.activeChapterId,
    );
  }, [snapshot, activeChapterId]);

  if (!projectRoot) {
    return (
      <div className="max-w-4xl mx-auto">
        <Card padding="lg">
          <p className="text-sm text-surface-400">请先打开项目。</p>
        </Card>
      </div>
    );
  }

  if (isLoading && !snapshot) {
    return (
      <div className="max-w-6xl mx-auto">
        <Card padding="lg">
          <p className="text-sm text-surface-400">指挥台加载中...</p>
        </Card>
      </div>
    );
  }

  if (error && !snapshot) {
    return (
      <div className="max-w-6xl mx-auto">
        <Card padding="lg" className="space-y-4">
          <p className="text-sm text-error">{getErrorMessage(error, "指挥台加载失败")}</p>
          <Button variant="primary" size="sm" onClick={() => void refetch()}>重试</Button>
        </Card>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-surface-100">全书指挥台</h1>
          <p className="text-sm text-surface-400 mt-1">
            以故事宪法约束生产，以当前窗口和当前章节持续推进成稿。
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="secondary" size="sm" onClick={() => setAssetDrawerOpen(true)}>
            资产抽屉
          </Button>
          <Button variant="ghost" size="sm" onClick={() => void refetch()}>
            刷新
          </Button>
        </div>
      </div>

      {snapshot && (
        <>
          <div className="grid grid-cols-2 md:grid-cols-4 xl:grid-cols-6 gap-4">
            <Card padding="md">
              <div className="text-xs text-surface-400">总字数</div>
              <div className="mt-2 text-2xl font-semibold text-surface-100">
                {snapshot.stats.totalWords.toLocaleString()}
              </div>
            </Card>
            <Card padding="md">
              <div className="text-xs text-surface-400">章节进度</div>
              <div className="mt-2 text-2xl font-semibold text-surface-100">
                {snapshot.stats.completedChapterCount}/{snapshot.stats.chapterCount}
              </div>
            </Card>
            <Card padding="md">
              <div className="text-xs text-surface-400">宪法基线</div>
              <div className="mt-2 text-2xl font-semibold text-primary">
                {snapshot.stats.blueprintProgress}%
              </div>
            </Card>
            <Card padding="md">
              <div className="text-xs text-surface-400">待处理反馈</div>
              <div className="mt-2 text-2xl font-semibold text-warning">
                {snapshot.reviewQueue.openFeedbackCount}
              </div>
            </Card>
            <Card padding="md">
              <div className="text-xs text-surface-400">待处理风险</div>
              <div className="mt-2 text-2xl font-semibold text-error">
                {snapshot.reviewQueue.openIssueCount}
              </div>
            </Card>
            <Card padding="md">
              <div className="text-xs text-surface-400">状态更新</div>
              <div className="mt-2 text-2xl font-semibold text-info">
                {snapshot.reviewQueue.stateUpdateCount}
              </div>
            </Card>
          </div>

          <div className="grid xl:grid-cols-[1.2fr_1fr] gap-6">
            <Card padding="lg" className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-lg font-semibold text-surface-100">作品宪法区</h2>
                  <p className="text-xs text-surface-400 mt-1">
                    蓝图、叙事义务、锁定术语与强约束规则共同构成故事权威层。
                  </p>
                </div>
                <div className="flex gap-2">
                  <Button variant="ghost" size="sm" onClick={() => setActiveRoute("blueprint")}>蓝图整理</Button>
                  <Button variant="ghost" size="sm" onClick={() => setActiveRoute("narrative")}>义务整理</Button>
                </div>
              </div>

              <div className="grid md:grid-cols-3 gap-3">
                <div className="rounded-lg bg-surface-800 px-3 py-3">
                  <div className="text-xs text-surface-500">蓝图步骤</div>
                  <div className="mt-2 text-lg text-surface-100">
                    {snapshot.constitution.blueprintSteps.filter((step) => step.status === "completed").length}
                    /{snapshot.constitution.blueprintSteps.length}
                  </div>
                </div>
                <div className="rounded-lg bg-surface-800 px-3 py-3">
                  <div className="text-xs text-surface-500">叙事义务</div>
                  <div className="mt-2 text-lg text-surface-100">{snapshot.constitution.obligations.length}</div>
                </div>
                <div className="rounded-lg bg-surface-800 px-3 py-3">
                  <div className="text-xs text-surface-500">锁定项 / 强规则</div>
                  <div className="mt-2 text-lg text-surface-100">
                    {snapshot.constitution.lockedTerms.length + snapshot.constitution.strongRules.length}
                  </div>
                </div>
              </div>

              <div className="grid md:grid-cols-2 gap-4">
                <div className="space-y-2">
                  <div className="text-sm font-medium text-surface-200">蓝图基线</div>
                  {snapshot.constitution.blueprintSteps.length === 0 ? (
                    <p className="text-xs text-surface-500">暂无蓝图步骤</p>
                  ) : (
                    snapshot.constitution.blueprintSteps.slice(0, 6).map((step) => (
                      <div key={step.id} className="rounded-lg bg-surface-800 px-3 py-2">
                        <div className="flex items-center justify-between gap-3">
                          <span className="text-sm text-surface-100">{step.title || step.stepKey}</span>
                          <Badge variant={step.status === "completed" ? "success" : step.status === "in_progress" ? "info" : "default"}>
                            {step.status === "completed" ? "完成" : step.status === "in_progress" ? "进行中" : "未开始"}
                          </Badge>
                        </div>
                      </div>
                    ))
                  )}
                </div>

                <div className="space-y-2">
                  <div className="text-sm font-medium text-surface-200">承诺与硬约束</div>
                  {snapshot.constitution.obligations.slice(0, 4).map((item) => (
                    <div key={item.id} className="rounded-lg bg-surface-800 px-3 py-2">
                      <div className="text-sm text-surface-100">{item.description}</div>
                      <div className="mt-1 text-xs text-surface-400">
                        {item.obligationType} · {item.payoffStatus}
                      </div>
                    </div>
                  ))}
                  {snapshot.constitution.lockedTerms.slice(0, 3).map((item) => (
                    <div key={item.id} className="rounded-lg bg-surface-800 px-3 py-2">
                      <div className="text-sm text-surface-100">{item.term}</div>
                      <div className="mt-1 text-xs text-info">锁定术语</div>
                    </div>
                  ))}
                  {snapshot.constitution.strongRules.slice(0, 3).map((item) => (
                    <div key={item.id} className="rounded-lg bg-surface-800 px-3 py-2">
                      <div className="text-sm text-surface-100">{item.title}</div>
                      <div className="mt-1 text-xs text-warning">{worldRuleConstraintLabel(item)}</div>
                    </div>
                  ))}
                </div>
              </div>
            </Card>

            <Card padding="lg" className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-lg font-semibold text-surface-100">风险与回报区</h2>
                  <p className="text-xs text-surface-400 mt-1">
                    一致性问题、回报事件与漂移告警在这里聚合，不再漂在外围页面。
                  </p>
                </div>
                <Button variant="ghost" size="sm" onClick={() => setActiveRoute("consistency")}>
                  审查详情
                </Button>
              </div>

              <div className="grid md:grid-cols-2 gap-3">
                <div className="rounded-lg bg-surface-800 px-3 py-3">
                  <div className="text-xs text-surface-500">反馈事件</div>
                  <div className="mt-2 text-lg text-surface-100">
                    {snapshot.reviewQueue.openFeedbackCount} open / {snapshot.reviewQueue.acknowledgedFeedbackCount} ack
                  </div>
                </div>
                <div className="rounded-lg bg-surface-800 px-3 py-3">
                  <div className="text-xs text-surface-500">高优先级风险</div>
                  <div className="mt-2 text-lg text-surface-100">{snapshot.reviewQueue.highSeverityIssueCount}</div>
                </div>
              </div>

              <div className="space-y-2">
                <div className="text-sm font-medium text-surface-200">待处理回报</div>
                {snapshot.reviewQueue.feedbackEvents.length === 0 ? (
                  <p className="text-xs text-surface-500">暂无待处理回报</p>
                ) : (
                  snapshot.reviewQueue.feedbackEvents.map((event) => (
                    <div key={event.id} className="rounded-lg bg-surface-800 px-3 py-2">
                      <div className="flex items-center justify-between gap-3">
                        <span className="text-sm text-surface-100">{event.ruleType}</span>
                        <Badge variant={event.status === "open" ? "warning" : "info"}>{reviewStatusLabel(event.status)}</Badge>
                      </div>
                      <div className="mt-1 text-xs text-surface-400">{event.conditionSummary}</div>
                    </div>
                  ))
                )}
              </div>

              <div className="space-y-2">
                <div className="text-sm font-medium text-surface-200">一致性风险</div>
                {snapshot.reviewQueue.consistencyIssues.length === 0 ? (
                  <p className="text-xs text-surface-500">暂无开放问题</p>
                ) : (
                  snapshot.reviewQueue.consistencyIssues.map((issue) => (
                    <div key={issue.id} className="rounded-lg bg-surface-800 px-3 py-2">
                      <div className="flex items-center justify-between gap-3">
                        <span className="text-sm text-surface-100">{issue.issueType}</span>
                        <Badge variant={issueSeverityVariant(issue.severity)}>{issue.severity}</Badge>
                      </div>
                      <div className="mt-1 text-xs text-surface-400">{issue.explanation}</div>
                    </div>
                  ))
                )}
              </div>

              {snapshot.reviewQueue.driftWarnings.length > 0 && (
                <div className="space-y-2">
                  <div className="text-sm font-medium text-surface-200">漂移告警</div>
                  {snapshot.reviewQueue.driftWarnings.map((warning, index) => (
                    <div key={`${warning}-${index}`} className="rounded-lg bg-warning/10 border border-warning/30 px-3 py-2 text-xs text-warning">
                      {warning}
                    </div>
                  ))}
                </div>
              )}
            </Card>
          </div>

          <Card padding="lg" className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-lg font-semibold text-surface-100">生产队列区</h2>
                <p className="text-xs text-surface-400 mt-1">
                  以窗口规划和章节队列驱动当前推进，编辑器只作为当前作业区存在。
                </p>
              </div>
              <Button variant="ghost" size="sm" onClick={() => setActiveRoute("chapters")}>
                章节深度整理
              </Button>
            </div>

            <div className="grid xl:grid-cols-[320px_1fr] gap-4">
              <div className="space-y-3">
                <div className="rounded-xl bg-surface-800 px-4 py-4 space-y-2">
                  <div className="text-xs text-surface-500">窗口规划</div>
                  <div className="text-sm text-surface-100">
                    未来 {snapshot.productionQueue.windowPlanning.windowPlanningHorizon} 章
                  </div>
                  <div className="text-xs text-surface-400">
                    卷结构：{snapshot.productionQueue.windowPlanning.volumeStructure || "暂无"}
                  </div>
                  <div className="text-xs text-surface-400">
                    计划章节数：{snapshot.productionQueue.windowPlanning.plannedChapterCount}
                  </div>
                  <div className="text-xs text-surface-400">
                    当前卷进度：{snapshot.productionQueue.windowPlanning.currentVolumeProgress}%
                  </div>
                </div>

                <div className="rounded-xl bg-surface-800 px-4 py-4 space-y-2">
                  <div className="text-xs text-surface-500">下一步动作</div>
                  {snapshot.productionQueue.nextActions.map((action, index) => (
                    <div key={`${action}-${index}`} className="text-sm text-surface-100">
                      {index + 1}. {action}
                    </div>
                  ))}
                </div>
              </div>

              <div className="rounded-xl border border-surface-700 overflow-hidden">
                <div className="px-4 py-3 bg-surface-800 border-b border-surface-700">
                  <div className="text-sm font-medium text-surface-100">章节队列</div>
                </div>
                <div className="max-h-[24rem] overflow-y-auto divide-y divide-surface-800">
                  {snapshot.productionQueue.chapters.length === 0 ? (
                    <div className="px-4 py-6 text-sm text-surface-500">暂无章节</div>
                  ) : (
                    snapshot.productionQueue.chapters.map((chapter) => {
                      const isActive = selectedChapter?.id === chapter.id;
                      return (
                        <button
                          key={chapter.id}
                          onClick={() => setActiveChapter(chapter.id, chapter.title)}
                          className={`w-full text-left px-4 py-3 transition-colors ${
                            isActive ? "bg-primary/10" : "hover:bg-surface-800/60"
                          }`}
                        >
                          <div className="flex items-center justify-between gap-3">
                            <div className="min-w-0">
                              <div className="text-sm text-surface-100 truncate">
                                #{chapter.chapterIndex} {chapter.title}
                              </div>
                              <div className="mt-1 text-xs text-surface-400">
                                {chapter.currentWords} / {chapter.targetWords || 0} 字
                              </div>
                            </div>
                            <Badge variant={chapterStatusVariant(chapter.status)}>
                              {chapterStatusLabel(chapter.status)}
                            </Badge>
                          </div>
                        </button>
                      );
                    })
                  )}
                </div>
              </div>
            </div>
          </Card>

          <div className="space-y-4">
            <div className="flex items-center justify-between gap-4">
              <div>
                <h2 className="text-lg font-semibold text-surface-100">当前作业区</h2>
                <p className="text-xs text-surface-400 mt-1">
                  当前任务、当前章节、AI 生成、结构化草案、状态账本都在此闭环。
                </p>
              </div>
              <div className="text-right">
                <div className="text-sm text-surface-100">{activeChapterTitle || selectedChapter?.title || "未选择章节"}</div>
                {selectedChapter && (
                  <div className="text-xs text-surface-400">
                    第 {selectedChapter.chapterIndex} 章 · {chapterStatusLabel(selectedChapter.status)}
                  </div>
                )}
              </div>
            </div>

            <EditorPage embedded hideChapterTree />
          </div>
        </>
      )}

      <AssetDrawer
        open={assetDrawerOpen}
        snapshot={snapshot ?? null}
        onClose={() => setAssetDrawerOpen(false)}
        onNavigate={(route) => {
          setAssetDrawerOpen(false);
          setActiveRoute(route);
        }}
      />

      <Modal
        open={showBlueprintPanel}
        onClose={() => setActiveRoute("command-center")}
        title="宪法整理：蓝图"
        width="lg"
      >
        <BlueprintPage embedded />
      </Modal>
      <Modal
        open={showNarrativePanel}
        onClose={() => setActiveRoute("command-center")}
        title="宪法整理：叙事义务"
        width="lg"
      >
        <NarrativePage />
      </Modal>
      <Modal
        open={showConsistencyPanel}
        onClose={() => setActiveRoute("command-center")}
        title="审查详情"
        width="lg"
      >
        <ConsistencyPage />
      </Modal>
      <Modal
        open={showChapterPanel}
        onClose={() => setActiveRoute("command-center")}
        title="章节队列整理"
        width="lg"
      >
        <ChaptersPage />
      </Modal>
    </div>
  );
}
