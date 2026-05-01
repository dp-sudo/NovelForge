import { useState } from "react";
import { Card } from "../../../components/cards/Card";
import type { ChapterContext, ReviewTrailRecord } from "../../../api/contextApi";

const ASSET_TYPE_LABEL: Record<string, string> = {
  character: "角色",
  location: "地点",
  organization: "组织",
  world_rule: "规则",
  term: "术语"
};

const CANDIDATE_STATUS_LABEL: Record<"idle" | "applying" | "applied" | "error", string> = {
  idle: "待处理",
  applying: "处理中",
  applied: "已采纳",
  error: "失败"
};

const STRUCTURED_DRAFT_STATUS_LABEL: Record<"pending" | "applying" | "applied" | "rejected" | "error", string> = {
  pending: "待确认",
  applying: "处理中",
  applied: "已入库",
  rejected: "已忽略",
  error: "失败"
};

const REVIEW_ACTION_LABEL: Record<string, string> = {
  approved: "采纳",
  rejected: "否决",
  edited: "人工修改",
};

export type EditorCandidateTargetKind = "character" | "world_rule" | "plot_node" | "glossary_term";

export interface EditorCandidateAction {
  label: string;
  targetKind: EditorCandidateTargetKind;
}

interface EditorContextPanelProps {
  chapterId: string | null;
  context: ChapterContext | null;
  reviewTrail: ReviewTrailRecord[];
  candidateStatus: Record<string, "idle" | "applying" | "applied" | "error">;
  getCandidateKey: (assetType: string, label: string) => string;
  getCandidateActions: (assetType: string) => EditorCandidateAction[];
  getStructuredDraftDisplayStatus: (
    draftId: string,
    persistedStatus: string,
  ) => "pending" | "applying" | "applied" | "rejected" | "error";
  onApplyCandidate: (
    candidate: ChapterContext["assetCandidates"][number],
    targetKind: EditorCandidateTargetKind,
  ) => Promise<void>;
  onApplyRelationshipDraft: (draft: ChapterContext["relationshipDrafts"][number]) => Promise<void>;
  onApplyInvolvementDraft: (draft: ChapterContext["involvementDrafts"][number]) => Promise<void>;
  onApplySceneDraft: (draft: ChapterContext["sceneDrafts"][number]) => Promise<void>;
  onRejectStructuredDraft: (
    draft:
      | ChapterContext["relationshipDrafts"][number]
      | ChapterContext["involvementDrafts"][number]
      | ChapterContext["sceneDrafts"][number],
  ) => Promise<void>;
}

export function EditorContextPanel(props: EditorContextPanelProps) {
  const {
    chapterId,
    context,
    reviewTrail,
    candidateStatus,
    getCandidateKey,
    getCandidateActions,
    getStructuredDraftDisplayStatus,
    onApplyCandidate,
    onApplyRelationshipDraft,
    onApplyInvolvementDraft,
    onApplySceneDraft,
    onRejectStructuredDraft,
  } = props;

  const [currentTab, setCurrentTab] = useState<"characters" | "world" | "plot" | "glossary">("characters");

  return (
    <div className="w-72 shrink-0 hidden xl:block">
      <Card padding="md" className="h-full overflow-y-auto">
        <h3 className="text-xs font-semibold text-surface-400 uppercase tracking-wider mb-3">
          上下文
        </h3>

        {!chapterId ? (
          <p className="text-xs text-surface-500">选择章节后显示关联上下文</p>
        ) : !context ? (
          <p className="text-xs text-surface-500">加载中...</p>
        ) : (
          <>
            <div className="flex gap-1 mb-3 border-b border-surface-700 pb-2">
              {(["characters", "world", "plot", "glossary"] as const).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setCurrentTab(tab)}
                  className={`text-xs px-2 py-1 rounded transition-colors ${
                    currentTab === tab
                      ? "bg-primary/10 text-primary"
                      : "text-surface-400 hover:text-surface-200"
                  }`}
                >
                  {tab === "characters" ? "角色" : tab === "world" ? "设定" : tab === "plot" ? "剧情" : "名词"}
                </button>
              ))}
            </div>

            {currentTab === "characters" && (
              <div className="space-y-2">
                {context.characters.length === 0 ? (
                  <p className="text-xs text-surface-500">暂无关联角色</p>
                ) : (
                  context.characters.map((character) => (
                    <div key={character.id} className="p-2 bg-surface-700/50 rounded-lg">
                      <div className="text-sm font-medium text-surface-200">{character.name}</div>
                      <div className="text-xs text-surface-400">{character.roleType}</div>
                      {character.motivation && (
                        <div className="text-xs text-surface-500 mt-1">
                          动机: {character.motivation.slice(0, 60)}
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            )}

            {currentTab === "world" && (
              <div className="space-y-2">
                {context.worldRules.length === 0 ? (
                  <p className="text-xs text-surface-500">暂无设定</p>
                ) : (
                  context.worldRules.map((rule) => (
                    <div key={rule.id} className="p-2 bg-surface-700/50 rounded-lg">
                      <div className="text-sm font-medium text-surface-200">{rule.title}</div>
                      <div className="text-xs text-surface-400">{rule.category}</div>
                      <div className="text-xs text-surface-500 mt-1">{rule.description.slice(0, 80)}</div>
                    </div>
                  ))
                )}
              </div>
            )}

            {currentTab === "plot" && (
              <div className="space-y-2">
                {context.plotNodes.length === 0 ? (
                  <p className="text-xs text-surface-500">暂无主线节点</p>
                ) : (
                  context.plotNodes.map((node) => (
                    <div key={node.id} className="p-2 bg-surface-700/50 rounded-lg">
                      <div className="flex items-center gap-2">
                        <span className="text-xs text-surface-500">#{node.sortOrder}</span>
                        <span className="text-sm font-medium text-surface-200">{node.title}</span>
                      </div>
                      <span className="text-xs text-surface-400">{node.nodeType}</span>
                      {node.goal && <div className="text-xs text-surface-500 mt-1">{node.goal.slice(0, 60)}</div>}
                    </div>
                  ))
                )}
              </div>
            )}

            {currentTab === "glossary" && (
              <div className="space-y-2">
                {context.glossary.length === 0 ? (
                  <p className="text-xs text-surface-500">暂无名词</p>
                ) : (
                  context.glossary.map((term, index) => (
                    <div key={index} className="flex items-center gap-2 p-2 bg-surface-700/50 rounded-lg">
                      <span className="text-sm font-medium text-surface-200">{term.term}</span>
                      <span className="text-xs text-surface-400">{term.termType}</span>
                      {term.locked && <span className="text-xs text-info ml-auto">锁定</span>}
                      {term.banned && <span className="text-xs text-error ml-auto">禁用</span>}
                    </div>
                  ))
                )}
              </div>
            )}

            <div className="mt-3 pt-3 border-t border-surface-700">
              <div className="flex items-center justify-between mb-2">
                <div className="text-xs font-semibold text-surface-400 uppercase tracking-wider">
                  资产候选
                </div>
                <span className="text-[11px] text-surface-500">
                  {context.assetCandidates.length} 条
                </span>
              </div>
              {context.assetCandidates.length === 0 ? (
                <p className="text-xs text-surface-500">未发现可抽取候选</p>
              ) : (
                <div className="space-y-2">
                  {context.assetCandidates.slice(0, 8).map((candidate) => (
                    <div key={`${candidate.assetType}:${candidate.label}`} className="p-2 bg-surface-700/50 rounded-lg">
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-sm text-surface-200">{candidate.label}</span>
                        <div className="flex items-center gap-2">
                          <span className="text-[11px] text-primary">
                            {ASSET_TYPE_LABEL[candidate.assetType] ?? candidate.assetType}
                          </span>
                          <span className="text-[11px] text-surface-500">
                            {CANDIDATE_STATUS_LABEL[candidateStatus[getCandidateKey(candidate.assetType, candidate.label)] ?? "idle"]}
                          </span>
                        </div>
                      </div>
                      <p className="text-[11px] text-surface-500 mt-1">
                        命中 {candidate.occurrences} 次 · 置信度 {(candidate.confidence * 100).toFixed(0)}%
                      </p>
                      <p className="text-xs text-surface-400 mt-1 whitespace-pre-wrap break-words">
                        {candidate.evidence}
                      </p>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {getCandidateActions(candidate.assetType).map((action) => {
                          const status = candidateStatus[getCandidateKey(candidate.assetType, candidate.label)] ?? "idle";
                          const isApplying = status === "applying";
                          return (
                            <button
                              key={`${candidate.assetType}:${candidate.label}:${action.targetKind}`}
                              onClick={() => void onApplyCandidate(candidate, action.targetKind)}
                              disabled={isApplying}
                              className="px-2 py-1 text-[11px] bg-surface-800 text-surface-200 border border-surface-600 rounded hover:bg-surface-700 disabled:opacity-40 transition-colors"
                            >
                              {action.label}
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="mt-3 pt-3 border-t border-surface-700 space-y-3">
              <div className="flex items-center justify-between">
                <div className="text-xs font-semibold text-surface-400 uppercase tracking-wider">
                  结构化草案
                </div>
                <span className="text-[11px] text-surface-500">
                  {(context.relationshipDrafts.length + context.involvementDrafts.length + context.sceneDrafts.length).toString()} 条
                </span>
              </div>

              <div className="space-y-2">
                <div className="text-[11px] text-surface-500">关系</div>
                {context.relationshipDrafts.length === 0 ? (
                  <p className="text-xs text-surface-500">未发现关系草案</p>
                ) : (
                  context.relationshipDrafts.slice(0, 4).map((draft) => {
                    const status = getStructuredDraftDisplayStatus(draft.id, draft.status);
                    return (
                      <div key={draft.id} className="p-2 bg-surface-700/50 rounded-lg">
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-sm text-surface-200">
                            {draft.sourceLabel} ↔ {draft.targetLabel}
                          </span>
                          <span className="text-[11px] text-surface-500">
                            {STRUCTURED_DRAFT_STATUS_LABEL[status]}
                          </span>
                        </div>
                        <p className="text-[11px] text-primary mt-1">{draft.relationshipType}</p>
                        <p className="text-xs text-surface-400 mt-1">{draft.evidence}</p>
                        <div className="mt-2 flex items-center gap-2">
                          <button
                            onClick={() => void onApplyRelationshipDraft(draft)}
                            disabled={status === "applying" || status === "applied" || status === "rejected"}
                            className="px-2 py-1 text-[11px] bg-surface-800 text-surface-200 border border-surface-600 rounded hover:bg-surface-700 disabled:opacity-40 transition-colors"
                          >
                            确认入库
                          </button>
                          <button
                            onClick={() => void onRejectStructuredDraft(draft)}
                            disabled={status === "applying" || status === "applied" || status === "rejected"}
                            className="px-2 py-1 text-[11px] bg-surface-900 text-surface-300 border border-surface-700 rounded hover:bg-surface-800 disabled:opacity-40 transition-colors"
                          >
                            忽略
                          </button>
                        </div>
                      </div>
                    );
                  })
                )}
              </div>

              <div className="space-y-2">
                <div className="text-[11px] text-surface-500">戏份</div>
                {context.involvementDrafts.length === 0 ? (
                  <p className="text-xs text-surface-500">未发现戏份草案</p>
                ) : (
                  context.involvementDrafts.slice(0, 4).map((draft) => {
                    const status = getStructuredDraftDisplayStatus(draft.id, draft.status);
                    return (
                      <div key={draft.id} className="p-2 bg-surface-700/50 rounded-lg">
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-sm text-surface-200">{draft.characterLabel}</span>
                          <span className="text-[11px] text-surface-500">
                            {STRUCTURED_DRAFT_STATUS_LABEL[status]}
                          </span>
                        </div>
                        <p className="text-[11px] text-primary mt-1">
                          {draft.involvementType} · {draft.occurrences} 次
                        </p>
                        <p className="text-xs text-surface-400 mt-1">{draft.evidence}</p>
                        <div className="mt-2 flex items-center gap-2">
                          <button
                            onClick={() => void onApplyInvolvementDraft(draft)}
                            disabled={status === "applying" || status === "applied" || status === "rejected"}
                            className="px-2 py-1 text-[11px] bg-surface-800 text-surface-200 border border-surface-600 rounded hover:bg-surface-700 disabled:opacity-40 transition-colors"
                          >
                            确认入库
                          </button>
                          <button
                            onClick={() => void onRejectStructuredDraft(draft)}
                            disabled={status === "applying" || status === "applied" || status === "rejected"}
                            className="px-2 py-1 text-[11px] bg-surface-900 text-surface-300 border border-surface-700 rounded hover:bg-surface-800 disabled:opacity-40 transition-colors"
                          >
                            忽略
                          </button>
                        </div>
                      </div>
                    );
                  })
                )}
              </div>

              <div className="space-y-2">
                <div className="text-[11px] text-surface-500">场景</div>
                {context.sceneDrafts.length === 0 ? (
                  <p className="text-xs text-surface-500">未发现场景草案</p>
                ) : (
                  context.sceneDrafts.slice(0, 4).map((draft) => {
                    const status = getStructuredDraftDisplayStatus(draft.id, draft.status);
                    return (
                      <div key={draft.id} className="p-2 bg-surface-700/50 rounded-lg">
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-sm text-surface-200">{draft.sceneLabel}</span>
                          <span className="text-[11px] text-surface-500">
                            {STRUCTURED_DRAFT_STATUS_LABEL[status]}
                          </span>
                        </div>
                        <p className="text-[11px] text-primary mt-1">{draft.sceneType}</p>
                        <p className="text-xs text-surface-400 mt-1">{draft.evidence}</p>
                        <div className="mt-2 flex items-center gap-2">
                          <button
                            onClick={() => void onApplySceneDraft(draft)}
                            disabled={status === "applying" || status === "applied" || status === "rejected"}
                            className="px-2 py-1 text-[11px] bg-surface-800 text-surface-200 border border-surface-600 rounded hover:bg-surface-700 disabled:opacity-40 transition-colors"
                          >
                            确认入库
                          </button>
                          <button
                            onClick={() => void onRejectStructuredDraft(draft)}
                            disabled={status === "applying" || status === "applied" || status === "rejected"}
                            className="px-2 py-1 text-[11px] bg-surface-900 text-surface-300 border border-surface-700 rounded hover:bg-surface-800 disabled:opacity-40 transition-colors"
                          >
                            忽略
                          </button>
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </div>

            <div className="mt-3 pt-3 border-t border-surface-700">
              <div className="flex items-center justify-between mb-2">
                <div className="text-xs font-semibold text-surface-400 uppercase tracking-wider">
                  审查轨迹
                </div>
                <span className="text-[11px] text-surface-500">
                  {reviewTrail.length} 条
                </span>
              </div>
              {reviewTrail.length === 0 ? (
                <p className="text-xs text-surface-500">暂无审查记录</p>
              ) : (
                <div className="space-y-2 mb-3">
                  {reviewTrail.slice(0, 8).map((item) => (
                    <div key={item.id} className="p-2 bg-surface-700/50 rounded-lg">
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-xs text-surface-200">
                          {REVIEW_ACTION_LABEL[item.action] ?? item.action}
                        </span>
                        <span className="text-[11px] text-surface-500">
                          {new Date(item.createdAt).toLocaleString("zh-CN")}
                        </span>
                      </div>
                      <p className="text-[11px] text-surface-500 mt-1">
                        {item.entityType} / {item.entityId}
                      </p>
                      {item.reason && (
                        <p className="text-xs text-surface-400 mt-1">理由：{item.reason}</p>
                      )}
                    </div>
                  ))}
                </div>
              )}

              <div className="text-xs text-surface-400">
                <div>目标字数: {context.chapter.targetWords.toLocaleString()}</div>
                <div>当前字数: {context.chapter.currentWords.toLocaleString()}</div>
                <div>状态: {context.chapter.status}</div>
                <div>状态账本摘要: {context.stateSummary.length}</div>
                {context.previousChapterSummary && (
                  <div className="mt-2">
                    <div className="text-surface-500 mb-1">前章摘要:</div>
                    <div className="text-surface-400">{context.previousChapterSummary.slice(0, 100)}</div>
                  </div>
                )}
                {context.stateSummary.length > 0 && (
                  <div className="mt-2">
                    <div className="text-surface-500 mb-1">最新状态:</div>
                    {context.stateSummary.slice(0, 3).map((item, index) => {
                      const payloadPreview = JSON.stringify(item.payload);
                      return (
                        <div key={`${item.subjectType}:${item.subjectId}:${item.stateKind}:${index}`} className="text-surface-400 mb-1">
                          <div>{item.subjectType}/{item.subjectId} · {item.stateKind}</div>
                          <div className="text-[11px] text-surface-500 break-words">{payloadPreview.slice(0, 140)}</div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>
          </>
        )}
      </Card>
    </div>
  );
}
