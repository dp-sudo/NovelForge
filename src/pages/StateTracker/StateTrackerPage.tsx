import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Badge } from "../../components/ui/Badge.js";
import { ConfirmDialog } from "../../components/dialogs/ConfirmDialog.js";
import {
  listStateSnapshots,
  getLatestStateSnapshot,
  deleteStateSnapshot,
  type StateSnapshotSummary,
  type StoryStateSnapshot,
} from "../../api/stateTrackerApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

const SNAPSHOT_TYPE_LABELS: Record<string, string> = {
  post_chapter: "章节完成",
  manual: "手动创建",
  auto: "自动生成",
};

export function StateTrackerPage() {
  const [summaries, setSummaries] = useState<StateSnapshotSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<StoryStateSnapshot | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [showDelete, setShowDelete] = useState(false);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setSummaries([]);
      return;
    }
    const data = await listStateSnapshots(projectRoot);
    setSummaries(data);
  }, [projectRoot]);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleSelect(summary: StateSnapshotSummary) {
    if (selectedId === summary.snapshotId) {
      setSelectedId(null);
      setDetail(null);
      return;
    }
    setSelectedId(summary.snapshotId);
    setDetailLoading(true);
    try {
      const snap = await getLatestStateSnapshot(
        projectRoot!,
        summary.chapterId
      );
      setDetail(snap);
    } catch {
      setDetail(null);
    } finally {
      setDetailLoading(false);
    }
  }

  async function handleDelete() {
    if (!selectedId || !projectRoot) return;
    await deleteStateSnapshot(projectRoot, selectedId);
    setShowDelete(false);
    setSelectedId(null);
    setDetail(null);
    await load();
  }

  const totalCharacters = summaries.reduce(
    (acc, s) => acc + s.characterCount,
    0
  );
  const totalPlots = summaries.reduce((acc, s) => acc + s.plotCount, 0);
  const totalWorlds = summaries.reduce((acc, s) => acc + s.worldCount, 0);

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-surface-100">
            故事状态追踪
          </h1>
          <p className="text-sm text-surface-400 mt-1">
            每章结束时自动生成状态快照，追踪角色、情节、世界的实时变化
          </p>
        </div>
      </div>

      {/* Stats bar */}
      <div className="grid grid-cols-4 gap-4 mb-6">
        <Card padding="md">
          <p className="text-xs text-surface-400">总快照数</p>
          <p className="text-2xl font-bold text-surface-100 mt-1">
            {summaries.length}
          </p>
        </Card>
        <Card padding="md">
          <p className="text-xs text-surface-400">角色状态条目</p>
          <p className="text-2xl font-bold text-info mt-1">
            {totalCharacters}
          </p>
        </Card>
        <Card padding="md">
          <p className="text-xs text-surface-400">情节状态条目</p>
          <p className="text-2xl font-bold text-warning mt-1">{totalPlots}</p>
        </Card>
        <Card padding="md">
          <p className="text-xs text-surface-400">世界状态条目</p>
          <p className="text-2xl font-bold text-success mt-1">{totalWorlds}</p>
        </Card>
      </div>

      {/* Snapshot table */}
      {summaries.length === 0 ? (
        <Card padding="lg" className="text-center">
          <p className="text-surface-400 text-sm">
            暂无状态快照。运行 AI 生成章节后将自动创建。
          </p>
        </Card>
      ) : (
        <div className="space-y-2">
          {summaries.map((s) => {
            const isExpanded = selectedId === s.snapshotId;
            return (
              <div key={s.snapshotId}>
                <button
                  onClick={() => void handleSelect(s)}
                  className={`w-full text-left p-4 rounded-lg transition-colors border ${
                    isExpanded
                      ? "bg-primary/10 border-primary/30"
                      : "bg-surface-800 border-surface-700 hover:border-surface-500"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <span className="text-sm font-medium text-surface-100">
                        {s.snapshotId.slice(0, 8)}...
                      </span>
                      <Badge variant="info">
                        {SNAPSHOT_TYPE_LABELS[s.snapshotType] ??
                          s.snapshotType}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-4 text-xs text-surface-400">
                      <span>👤 {s.characterCount}</span>
                      <span>📈 {s.plotCount}</span>
                      <span>🌍 {s.worldCount}</span>
                      <span className="text-surface-500">
                        {isExpanded ? "▲" : "▼"}
                      </span>
                    </div>
                  </div>
                  <p className="text-xs text-surface-400 mt-1">
                    章节 ID: {s.chapterId.slice(0, 8)}...
                  </p>
                </button>

                {/* Expanded detail */}
                {isExpanded && (
                  <div className="ml-4 mt-2 space-y-3">
                    {detailLoading ? (
                      <Card padding="md">
                        <p className="text-sm text-surface-400">加载中...</p>
                      </Card>
                    ) : !detail ? (
                      <Card padding="md">
                        <p className="text-sm text-surface-400">
                          无法加载快照详情
                        </p>
                      </Card>
                    ) : (
                      <>
                        {/* Character states */}
                        {detail.characterStates.length > 0 && (
                          <Card padding="md" className="space-y-2">
                            <h4 className="text-xs font-semibold text-surface-300 uppercase tracking-wide">
                              角色状态
                            </h4>
                            {detail.characterStates.map((cs) => (
                              <div
                                key={cs.id}
                                className="p-2 bg-surface-700/50 rounded text-xs text-surface-200 space-y-0.5"
                              >
                                <p className="font-medium">
                                  角色 ID: {cs.characterId.slice(0, 8)}...
                                </p>
                                {cs.location && <p>📍 位置: {cs.location}</p>}
                                {cs.emotionalState && (
                                  <p>💭 情绪: {cs.emotionalState}</p>
                                )}
                                {cs.arcProgress && (
                                  <p>📊 成长弧: {cs.arcProgress}</p>
                                )}
                                {cs.knowledgeGained && (
                                  <p>💡 新获信息: {cs.knowledgeGained}</p>
                                )}
                                {cs.relationshipsChanged && (
                                  <p>🤝 关系变化: {cs.relationshipsChanged}</p>
                                )}
                              </div>
                            ))}
                          </Card>
                        )}

                        {/* Plot states */}
                        {detail.plotStates.length > 0 && (
                          <Card padding="md" className="space-y-2">
                            <h4 className="text-xs font-semibold text-surface-300 uppercase tracking-wide">
                              情节进度
                            </h4>
                            {detail.plotStates.map((ps) => (
                              <div
                                key={ps.id}
                                className="p-2 bg-surface-700/50 rounded text-xs text-surface-200 flex items-center gap-3"
                              >
                                <Badge
                                  variant={
                                    ps.progressStatus === "resolved"
                                      ? "success"
                                      : ps.progressStatus === "in_progress"
                                        ? "warning"
                                        : "default"
                                  }
                                >
                                  {ps.progressStatus}
                                </Badge>
                                {ps.tensionLevel !== null && (
                                  <span>紧张度: {ps.tensionLevel}/10</span>
                                )}
                                {ps.openThreads && (
                                  <span className="text-surface-400">
                                    线索: {ps.openThreads}
                                  </span>
                                )}
                              </div>
                            ))}
                          </Card>
                        )}

                        {/* World states */}
                        {detail.worldStates.length > 0 && (
                          <Card padding="md" className="space-y-2">
                            <h4 className="text-xs font-semibold text-surface-300 uppercase tracking-wide">
                              世界状态
                            </h4>
                            {detail.worldStates.map((ws) => (
                              <div
                                key={ws.id}
                                className="p-2 bg-surface-700/50 rounded text-xs text-surface-200 flex items-center gap-2"
                              >
                                {ws.changedInChapter && (
                                  <Badge variant="warning">变化</Badge>
                                )}
                                <span>{ws.stateDescription}</span>
                              </div>
                            ))}
                          </Card>
                        )}

                        <div className="flex justify-end">
                          <Button
                            variant="danger"
                            size="sm"
                            onClick={() => setShowDelete(true)}
                          >
                            删除此快照
                          </Button>
                        </div>
                      </>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      <ConfirmDialog
        open={showDelete}
        title="删除状态快照"
        message="确定删除此快照及所有关联的状态条目？"
        variant="danger"
        confirmLabel="删除"
        onConfirm={() => void handleDelete()}
        onCancel={() => setShowDelete(false)}
      />
    </div>
  );
}
