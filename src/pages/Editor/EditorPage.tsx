import { useEffect, useState, useCallback, useMemo, useRef } from "react";
import { useEditorStore } from "../../stores/editorStore";
import { useProjectStore } from "../../stores/projectStore";
import { Card } from "../../components/cards/Card";
import { Badge } from "../../components/ui/Badge";
import { AiCommandBar } from "../../components/ai/AiCommandBar";
import { AiPreviewPanel } from "../../components/ai/AiPreviewPanel";
import {
  assignChapterVolume,
  autosaveDraft,
  createSnapshot,
  listChapters,
  listSnapshots,
  listVolumes,
  readSnapshotContent,
  recoverDraft,
  saveChapterContent,
  type SnapshotRecord,
  type VolumeRecord
} from "../../api/chapterApi";
import {
  applyAssetCandidate,
  applyStructuredDraft,
  getChapterContext,
  type ChapterContext
} from "../../api/contextApi";
import { streamAiChapterTask } from "../../api/aiApi";
import type { ChapterRecord } from "../../api/chapterApi";
import { Modal } from "../../components/dialogs/Modal";
import { Input } from "../../components/forms/Input";
import { Select } from "../../components/forms/Select";
import { Textarea } from "../../components/forms/Textarea";
import { Button } from "../../components/ui/Button.js";
import { FindBar } from "../../components/editor/FindBar.js";

const AUTOSAVE_DELAY_MS = 5000;

const STATUS_BADGE: Record<string, { variant: "default" | "success" | "warning" | "error" | "info"; label: string }> = {
  saved: { variant: "success", label: "已保存" },
  saving: { variant: "info", label: "保存中..." },
  unsaved: { variant: "warning", label: "未保存" },
  autosaving: { variant: "info", label: "自动保存..." },
  error: { variant: "error", label: "保存失败" }
};

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

const STRUCTURED_DRAFT_STATUS_LABEL: Record<"idle" | "applying" | "applied" | "error", string> = {
  idle: "待确认",
  applying: "处理中",
  applied: "已入库",
  error: "失败"
};

export function EditorPage() {
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const selRef = useRef<{ start: number; end: number }>({ start: 0, end: 0 });
  const autosaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [chapters, setChapters] = useState<ChapterRecord[]>([]);
  const [volumes, setVolumes] = useState<VolumeRecord[]>([]);
  const [volumeFilter, setVolumeFilter] = useState<string>("all");
  const [context, setContext] = useState<ChapterContext | null>(null);
  const [showRecovery, setShowRecovery] = useState(false);
  const [recoveryContent, setRecoveryContent] = useState("");
  const [showAiPanel, setShowAiPanel] = useState(false);
  const [originalText, setOriginalText] = useState<string | undefined>(undefined);
  const [findOpen, setFindOpen] = useState(false);
  const [showSnapshotModal, setShowSnapshotModal] = useState(false);
  const [snapshotTitle, setSnapshotTitle] = useState("");
  const [snapshotNote, setSnapshotNote] = useState("");
  const [snapshots, setSnapshots] = useState<SnapshotRecord[]>([]);
  const [snapshotContent, setSnapshotContent] = useState("");
  const [selectedSnapshotId, setSelectedSnapshotId] = useState<string | null>(null);
  const [snapshotLoading, setSnapshotLoading] = useState(false);
  const [creatingSnapshot, setCreatingSnapshot] = useState(false);
  const [editorNotice, setEditorNotice] = useState<string | null>(null);
  const [candidateStatus, setCandidateStatus] = useState<Record<string, "idle" | "applying" | "applied" | "error">>({});
  const [structuredDraftStatus, setStructuredDraftStatus] = useState<Record<string, "idle" | "applying" | "applied" | "error">>({});
  const [currentTab, setCurrentTab] = useState<"characters" | "world" | "plot" | "glossary">("characters");

  const chapterId = useEditorStore((s) => s.activeChapterId);
  const chapterTitle = useEditorStore((s) => s.activeChapterTitle);
  const content = useEditorStore((s) => s.content);
  const saveStatus = useEditorStore((s) => s.saveStatus);
  const wordCount = useEditorStore((s) => s.wordCount);
  const isDirty = useEditorStore((s) => s.isDirty);
  const aiStreamStatus = useEditorStore((s) => s.aiStreamStatus);
  const aiPreviewContent = useEditorStore((s) => s.aiPreviewContent);
  const aiTaskType = useEditorStore((s) => s.aiTaskType);
  const aiStreamError = useEditorStore((s) => s.aiStreamError);
  const store = useEditorStore();
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setChapters([]);
      setVolumes([]);
      return;
    }
    const [chapterRows, volumeRows] = await Promise.all([
      listChapters(projectRoot),
      listVolumes(projectRoot)
    ]);
    setChapters(chapterRows);
    setVolumes(volumeRows);
  }, [projectRoot]);

  const refreshChapterContext = useCallback(async (root: string, cid: string) => {
    try {
      const ctx = await getChapterContext(root, cid);
      setContext(ctx);
    } catch {
      setContext(null);
    }
  }, []);

  const getCandidateKey = useCallback((assetType: string, label: string) => `${assetType}:${label}`, []);
  const getStructuredDraftKey = useCallback(
    (kind: "relationship" | "involvement" | "scene", sourceLabel: string, targetLabel?: string) =>
      `${kind}:${sourceLabel}:${targetLabel ?? ""}`,
    []
  );

  useEffect(() => { void load(); }, [load]);

  // Load chapter content and check recovery
  useEffect(() => {
    if (!chapterId || !projectRoot) {
      setContext(null);
      setCandidateStatus({});
      setStructuredDraftStatus({});
      return;
    }
    const cid = chapterId;
    let cancelled = false;
    async function loadChapterRuntimeData() {
      try {
        const result = await recoverDraft(cid, projectRoot);
        if (!cancelled && result.hasNewerDraft && result.draftContent) {
          setRecoveryContent(result.draftContent);
          setShowRecovery(true);
        }
      } catch {
        if (!cancelled) {
          setShowRecovery(false);
        }
      }
      if (!cancelled) {
        await refreshChapterContext(projectRoot, cid);
      }
    }
    void loadChapterRuntimeData();
    return () => {
      cancelled = true;
    };
  }, [chapterId, projectRoot, refreshChapterContext]);

  // Update word count when content changes
  useEffect(() => {
    const wc = content.replace(/\s+/g, "").length;
    store.setWordCount(wc);
    store.setIsDirty(true);
    store.setSaveStatus("unsaved");
  }, [content]);

  // Autosave debounce
  useEffect(() => {
    if (!chapterId || !isDirty || !projectRoot) return;
    if (autosaveTimer.current) clearTimeout(autosaveTimer.current);
    autosaveTimer.current = setTimeout(async () => {
      store.setSaveStatus("autosaving");
      try {
        await autosaveDraft(chapterId, content, projectRoot);
        store.setSaveStatus("unsaved");
      } catch {
        store.setSaveStatus("error");
      }
    }, AUTOSAVE_DELAY_MS);
    return () => {
      if (autosaveTimer.current) clearTimeout(autosaveTimer.current);
    };
  }, [content, isDirty, chapterId, projectRoot]);

  // Keyboard shortcuts: Ctrl+S (save), Ctrl+F (find), Ctrl+P (navigate — handled globally)
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.ctrlKey || e.metaKey) {
        switch (e.key) {
          case "s":
            e.preventDefault();
            if (chapterId) void handleSave();
            break;
          case "f":
            e.preventDefault();
            setFindOpen(true);
            break;
        }
      }
      if (e.key === "Escape" && findOpen) {
        setFindOpen(false);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [chapterId, content, findOpen]);

  function handleFindSelectMatch(start: number, end: number) {
    if (editorRef.current) {
      editorRef.current.focus();
      editorRef.current.setSelectionRange(start, end);
      // Rough scroll: estimate line height and scroll to the match area
      const textBefore = content.slice(0, start);
      const linesBefore = textBefore.split("\n").length - 1;
      const lineHeight = 22; // approximate
      editorRef.current.scrollTop = Math.max(0, linesBefore * lineHeight - 100);
    }
  }

  async function handleSave() {
    if (!chapterId || !projectRoot) return;
    store.setSaveStatus("saving");
    try {
      const result = await saveChapterContent(chapterId, content, projectRoot);
      store.setSaveStatus("saved");
      store.setLastSavedAt(result.updatedAt);
      store.setIsDirty(false);
      await load();
      await refreshChapterContext(projectRoot, chapterId);
      setCandidateStatus({});
      setStructuredDraftStatus({});
    } catch {
      store.setSaveStatus("error");
    }
  }

  function handleSelectChapter(ch: ChapterRecord) {
    store.setActiveChapter(ch.id, ch.title);
    store.setContent("");
    store.setSaveStatus("saved");
    store.setIsDirty(false);
    setCandidateStatus({});
    setStructuredDraftStatus({});
    setShowRecovery(false);
    setShowAiPanel(false);
    store.resetAiPreview();
  }

  function handleRecoverAccept() {
    store.setContent(recoveryContent);
    setShowRecovery(false);
    store.setIsDirty(true);
    store.setSaveStatus("unsaved");
  }

  function handleRecoverDiscard() {
    setShowRecovery(false);
  }

  function handleEditorInput(e: React.ChangeEvent<HTMLTextAreaElement>) {
    store.setContent(e.target.value);
  }

  function handleEditorSelect() {
    if (editorRef.current) {
      selRef.current = {
        start: editorRef.current.selectionStart,
        end: editorRef.current.selectionEnd
      };
    }
  }

  async function handleAiCommand(taskType: string, _userInstruction: string) {
    if (!chapterId || !projectRoot) return;
    setShowAiPanel(true);
    store.resetAiPreview();
    store.setAiTaskType(taskType);
    store.setAiStreamStatus("streaming");

    const selectedText = selRef.current.start !== selRef.current.end
      ? content.slice(selRef.current.start, selRef.current.end)
      : undefined;
    setOriginalText(selectedText);

    try {
      const stream = streamAiChapterTask({
        projectRoot,
        taskType: taskType,
        userInstruction: _userInstruction,
        chapterId,
        selectedText
      });

      for await (const event of stream) {
        if (event.type === "delta" && event.delta) {
          store.appendAiPreviewContent(event.delta);
        } else if (event.type === "delta" && event.reasoning) {
          store.setAiPreviewContent((store.getState().aiPreviewContent || "") + "[思考]");
        } else if (event.type === "done") {
          store.setAiStreamStatus("completed");
        } else if (event.type === "error") {
          if (event.error) store.setAiStreamError(event.error);
          store.setAiStreamStatus("error");
        }
      }
    } catch {
      store.setAiStreamError("AI 生成异常，请检查控制台日志");
      store.setAiStreamStatus("error");
    }
  }

  function handleAiInsert(strategy: "cursor" | "replace" | "append") {
    if (!aiPreviewContent) return;
    let newContent = content;
    if (strategy === "append") {
      newContent = content + "\n\n" + aiPreviewContent;
    } else if (strategy === "replace") {
      const sel = selRef.current;
      if (sel.start !== sel.end) {
        newContent = content.slice(0, sel.start) + aiPreviewContent + content.slice(sel.end);
      } else {
        newContent = content.slice(0, sel.start) + aiPreviewContent + content.slice(sel.start);
      }
    } else {
      // cursor
      newContent = content.slice(0, selRef.current.start) + aiPreviewContent + content.slice(selRef.current.start);
    }
    store.setContent(newContent);
    setShowAiPanel(false);
    store.resetAiPreview();
  }

  function handleAiDiscard() {
    setShowAiPanel(false);
    store.resetAiPreview();
  }

  function handleAiCopy() {
    if (aiPreviewContent) {
      void navigator.clipboard.writeText(aiPreviewContent);
    }
  }

  async function loadSnapshotsForCurrentChapter() {
    if (!projectRoot || !chapterId) {
      setSnapshots([]);
      return;
    }
    setSnapshotLoading(true);
    try {
      const rows = await listSnapshots(projectRoot, chapterId);
      setSnapshots(rows);
      if (rows.length === 0) {
        setSelectedSnapshotId(null);
        setSnapshotContent("");
      }
    } finally {
      setSnapshotLoading(false);
    }
  }

  async function handleOpenSnapshotModal() {
    setShowSnapshotModal(true);
    setEditorNotice(null);
    await loadSnapshotsForCurrentChapter();
  }

  async function handleCreateSnapshot() {
    if (!projectRoot || !chapterId) return;
    setCreatingSnapshot(true);
    setEditorNotice(null);
    try {
      await createSnapshot(
        projectRoot,
        chapterId,
        snapshotTitle.trim() || undefined,
        snapshotNote.trim() || undefined
      );
      setSnapshotTitle("");
      setSnapshotNote("");
      setEditorNotice("快照已创建");
      await loadSnapshotsForCurrentChapter();
    } catch (err) {
      setEditorNotice(err instanceof Error ? err.message : "创建快照失败");
    } finally {
      setCreatingSnapshot(false);
    }
  }

  async function handleSelectSnapshot(snapshotId: string) {
    if (!projectRoot) return;
    setSelectedSnapshotId(snapshotId);
    setSnapshotLoading(true);
    try {
      const text = await readSnapshotContent(projectRoot, snapshotId);
      setSnapshotContent(text);
    } catch (err) {
      setSnapshotContent("");
      setEditorNotice(err instanceof Error ? err.message : "读取快照失败");
    } finally {
      setSnapshotLoading(false);
    }
  }

  async function handleAssignCurrentChapterVolume(volumeId: string) {
    if (!projectRoot || !chapterId) return;
    try {
      await assignChapterVolume(projectRoot, chapterId, volumeId || undefined);
      setChapters((prev) =>
        prev.map((row) =>
          row.id === chapterId ? { ...row, volumeId: volumeId || null } : row
        )
      );
      await load();
    } catch (err) {
      setEditorNotice(err instanceof Error ? err.message : "章节归卷失败");
    }
  }

  function getCandidateActions(assetType: string): Array<{ label: string; targetKind: "character" | "world_rule" | "plot_node" | "glossary_term" }> {
    if (assetType === "character") {
      return [
        { label: "加入角色", targetKind: "character" },
        { label: "加入支线", targetKind: "plot_node" },
      ];
    }
    if (assetType === "location" || assetType === "organization" || assetType === "world_rule") {
      return [
        { label: "加入设定", targetKind: "world_rule" },
        { label: "加入支线", targetKind: "plot_node" },
        { label: "加入名词", targetKind: "glossary_term" },
      ];
    }
    return [
      { label: "加入名词", targetKind: "glossary_term" },
      { label: "加入支线", targetKind: "plot_node" },
    ];
  }

  async function handleApplyCandidate(
    candidate: { label: string; assetType: string; evidence: string },
    targetKind: "character" | "world_rule" | "plot_node" | "glossary_term"
  ) {
    if (!projectRoot || !chapterId) return;
    const key = getCandidateKey(candidate.assetType, candidate.label);
    setCandidateStatus((prev) => ({ ...prev, [key]: "applying" }));
    try {
      const result = await applyAssetCandidate(projectRoot, chapterId, {
        label: candidate.label,
        assetType: candidate.assetType,
        evidence: candidate.evidence,
        targetKind,
      });
      setCandidateStatus((prev) => ({ ...prev, [key]: "applied" }));
      setEditorNotice(
        result.action === "created"
          ? `已创建并关联：${candidate.label}`
          : `已关联已有资产：${candidate.label}`
      );
      await refreshChapterContext(projectRoot, chapterId);
    } catch (err) {
      setCandidateStatus((prev) => ({ ...prev, [key]: "error" }));
      setEditorNotice(err instanceof Error ? err.message : "候选采纳失败");
    }
  }

  async function handleApplyRelationshipDraft(draft: {
    sourceLabel: string;
    targetLabel: string;
    relationshipType: string;
    evidence: string;
  }) {
    if (!projectRoot || !chapterId) return;
    const key = getStructuredDraftKey("relationship", draft.sourceLabel, draft.targetLabel);
    setStructuredDraftStatus((prev) => ({ ...prev, [key]: "applying" }));
    try {
      const result = await applyStructuredDraft(projectRoot, chapterId, {
        draftKind: "relationship",
        sourceLabel: draft.sourceLabel,
        targetLabel: draft.targetLabel,
        relationshipType: draft.relationshipType,
        evidence: draft.evidence,
      });
      setStructuredDraftStatus((prev) => ({ ...prev, [key]: "applied" }));
      setEditorNotice(
        result.action === "created"
          ? `关系已入库：${draft.sourceLabel} - ${draft.targetLabel}`
          : `关系已存在：${draft.sourceLabel} - ${draft.targetLabel}`
      );
      await refreshChapterContext(projectRoot, chapterId);
    } catch (err) {
      setStructuredDraftStatus((prev) => ({ ...prev, [key]: "error" }));
      setEditorNotice(err instanceof Error ? err.message : "关系草案入库失败");
    }
  }

  async function handleApplyInvolvementDraft(draft: {
    characterLabel: string;
    involvementType: string;
    evidence: string;
  }) {
    if (!projectRoot || !chapterId) return;
    const key = getStructuredDraftKey("involvement", draft.characterLabel);
    setStructuredDraftStatus((prev) => ({ ...prev, [key]: "applying" }));
    try {
      const result = await applyStructuredDraft(projectRoot, chapterId, {
        draftKind: "involvement",
        sourceLabel: draft.characterLabel,
        involvementType: draft.involvementType,
        evidence: draft.evidence,
      });
      setStructuredDraftStatus((prev) => ({ ...prev, [key]: "applied" }));
      setEditorNotice(
        result.action === "created"
          ? `戏份已入库：${draft.characterLabel}`
          : `戏份已存在：${draft.characterLabel}`
      );
      await refreshChapterContext(projectRoot, chapterId);
    } catch (err) {
      setStructuredDraftStatus((prev) => ({ ...prev, [key]: "error" }));
      setEditorNotice(err instanceof Error ? err.message : "戏份草案入库失败");
    }
  }

  async function handleApplySceneDraft(draft: {
    sceneLabel: string;
    sceneType: string;
    evidence: string;
  }) {
    if (!projectRoot || !chapterId) return;
    const key = getStructuredDraftKey("scene", draft.sceneLabel);
    setStructuredDraftStatus((prev) => ({ ...prev, [key]: "applying" }));
    try {
      const result = await applyStructuredDraft(projectRoot, chapterId, {
        draftKind: "scene",
        sourceLabel: draft.sceneLabel,
        sceneType: draft.sceneType,
        evidence: draft.evidence,
      });
      setStructuredDraftStatus((prev) => ({ ...prev, [key]: "applied" }));
      setEditorNotice(
        result.action === "created"
          ? `场景已入库：${draft.sceneLabel}`
          : `场景已存在：${draft.sceneLabel}`
      );
      await refreshChapterContext(projectRoot, chapterId);
    } catch (err) {
      setStructuredDraftStatus((prev) => ({ ...prev, [key]: "error" }));
      setEditorNotice(err instanceof Error ? err.message : "场景草案入库失败");
    }
  }

  const volumeFilterOptions = useMemo(
    () => [
      { value: "all", label: "全部卷" },
      { value: "unassigned", label: "未分卷" },
      ...volumes.map((volume) => ({ value: volume.id, label: volume.title })),
    ],
    [volumes]
  );

  const filteredChapters = useMemo(() => {
    if (volumeFilter === "all") return chapters;
    if (volumeFilter === "unassigned") {
      return chapters.filter((chapter) => !chapter.volumeId);
    }
    return chapters.filter((chapter) => chapter.volumeId === volumeFilter);
  }, [chapters, volumeFilter]);

  const groupedChapters = useMemo(() => {
    if (volumeFilter !== "all") {
      return [{ id: volumeFilter, title: "筛选结果", items: filteredChapters }];
    }
    const groups = volumes.map((volume) => ({
      id: volume.id,
      title: `${volume.title} (${volume.chapterCount})`,
      items: filteredChapters.filter((chapter) => chapter.volumeId === volume.id)
    }));
    const unassigned = filteredChapters.filter((chapter) => !chapter.volumeId);
    groups.push({
      id: "unassigned",
      title: `未分卷 (${unassigned.length})`,
      items: unassigned
    });
    return groups;
  }, [volumeFilter, volumes, filteredChapters]);

  const currentChapter = chapters.find((chapter) => chapter.id === chapterId);

  return (
    <div className="flex gap-4 h-[calc(100vh-8rem)]">
      {/* Left: Chapter Tree */}
      <div className="w-56 shrink-0 bg-surface-800 border border-surface-700 rounded-xl flex flex-col overflow-hidden">
        <div className="px-3 py-2.5 border-b border-surface-700 space-y-2">
          <h3 className="text-xs font-semibold text-surface-400 uppercase tracking-wider">章节树</h3>
          <Select
            value={volumeFilter}
            onChange={(e) => setVolumeFilter(e.target.value)}
            options={volumeFilterOptions}
          />
        </div>
        <div className="flex-1 overflow-y-auto p-2 space-y-2">
          {chapters.length === 0 ? (
            <p className="text-xs text-surface-500 text-center py-8">暂无章节</p>
          ) : (
            groupedChapters.map((group) => (
              <div key={group.id} className="space-y-1">
                {volumeFilter === "all" && (
                  <div className="text-[11px] uppercase tracking-wide text-surface-500 px-1">{group.title}</div>
                )}
                {group.items.length === 0 ? (
                  volumeFilter === "all" ? (
                    <p className="text-[11px] text-surface-600 px-2 py-1">暂无章节</p>
                  ) : null
                ) : (
                  group.items.map((chapter) => (
                    <button
                      key={chapter.id}
                      onClick={() => handleSelectChapter(chapter)}
                      className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors ${
                        chapterId === chapter.id
                          ? "bg-primary/10 text-primary border border-primary/20"
                          : "text-surface-300 hover:bg-surface-700 border border-transparent"
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="truncate">
                          <span className="text-surface-500 mr-1.5">#{chapter.chapterIndex}</span>
                          {chapter.title}
                        </span>
                      </div>
                      <div className="flex items-center gap-2 mt-0.5">
                        <span className="text-xs text-surface-500">{chapter.currentWords}字</span>
                        <Badge variant={chapter.status === "completed" ? "success" : chapter.status === "drafting" ? "info" : "default"}>
                          {chapter.status === "completed" ? "已完成" : chapter.status === "drafting" ? "写作中" : "待修订"}
                        </Badge>
                      </div>
                    </button>
                  ))
                )}
              </div>
            ))
          )}
        </div>
      </div>

      {/* Center: Editor */}
      <div className="flex-1 min-w-0 flex flex-col gap-3">
        {/* Recovery Banner */}
        {showRecovery && (
          <Card padding="sm" className="border-warning/30 bg-warning/5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-warning">检测到未保存的草稿</span>
              <div className="flex gap-2">
                <button onClick={handleRecoverAccept} className="px-3 py-1 text-xs bg-warning/20 text-warning border border-warning/30 rounded-lg hover:bg-warning/30 transition-colors">恢复草稿</button>
                <button onClick={handleRecoverDiscard} className="px-3 py-1 text-xs bg-surface-700 text-surface-300 border border-surface-600 rounded-lg hover:bg-surface-600 transition-colors">忽略</button>
              </div>
            </div>
          </Card>
        )}

        {/* Editor Top Bar */}
        <Card padding="md">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-3 min-w-0">
              <h2 className="text-base font-semibold text-surface-100">
                {chapterTitle || "选择或创建章节"}
              </h2>
              {chapterId && (
                <span className="text-xs text-surface-400">
                  #{chapters.find((c) => c.id === chapterId)?.chapterIndex ?? "-"}
                </span>
              )}
            </div>
            <div className="flex items-center gap-2 shrink-0">
              {chapterId && (
                <>
                  <span className="text-xs text-surface-400">
                    {wordCount.toLocaleString()} 字
                  </span>
                  <select
                    value={currentChapter?.volumeId ?? ""}
                    onChange={(e) => void handleAssignCurrentChapterVolume(e.target.value)}
                    className="px-2 py-1 text-xs bg-surface-800 border border-surface-600 rounded text-surface-100"
                  >
                    <option value="">未分卷</option>
                    {volumes.map((volume) => (
                      <option key={volume.id} value={volume.id}>
                        {volume.title}
                      </option>
                    ))}
                  </select>
                  {(() => {
                    const sb = STATUS_BADGE[saveStatus] ?? STATUS_BADGE.saved;
                    return <Badge variant={sb.variant}>{sb.label}</Badge>;
                  })()}
                  <button
                    onClick={() => void handleOpenSnapshotModal()}
                    disabled={!chapterId}
                    className="px-3 py-1 text-xs bg-surface-700 text-surface-200 border border-surface-600 rounded-lg hover:bg-surface-600 transition-colors disabled:opacity-40"
                  >
                    快照
                  </button>
                  <button
                    onClick={() => void handleSave()}
                    disabled={saveStatus === "saving" || !chapterId}
                    className="px-3 py-1 text-xs bg-primary/20 text-primary border border-primary/30 rounded-lg hover:bg-primary/30 transition-colors disabled:opacity-40"
                  >
                    保存
                  </button>
                </>
              )}
            </div>
          </div>
          {editorNotice && (
            <p className="text-xs text-info mt-2">{editorNotice}</p>
          )}
        </Card>

        {/* Editor Textarea */}
        <Card padding="none" className="flex-1 flex">
          {!chapterId ? (
            <div className="flex-1 flex items-center justify-center text-surface-500 text-sm">
              从左侧选择一个章节开始写作
            </div>
          ) : (
            <div className="flex-1 flex flex-col">
              <FindBar
                open={findOpen}
                content={content}
                onClose={() => setFindOpen(false)}
                onSelectMatch={handleFindSelectMatch}
              />
              <textarea
                ref={editorRef}
                value={content}
                onChange={handleEditorInput}
                onSelect={handleEditorSelect}
                onClick={handleEditorSelect}
                className="flex-1 w-full bg-surface-900 text-surface-100 p-6 resize-none focus:outline-none text-base leading-relaxed font-sans placeholder-surface-500"
                placeholder="开始写作..."
                spellCheck={false}
              />
            </div>
          )}
        </Card>

        {/* AI Command Bar */}
        {chapterId && (
          <AiCommandBar
            onCommand={handleAiCommand}
            disabled={aiStreamStatus === "streaming"}
          />
        )}

        {/* AI Preview Panel */}
        {showAiPanel && (
          <AiPreviewPanel
            status={aiStreamStatus}
            content={aiPreviewContent}
            errorMessage={aiStreamError}
            originalText={originalText}
            taskType={aiTaskType}
            onInsert={handleAiInsert}
            onDiscard={handleAiDiscard}
            onCopy={handleAiCopy}
          />
        )}
      </div>

      {/* Right: Context Panel */}
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
              {/* Tabs */}
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

              {/* Character Tab */}
              {currentTab === "characters" && (
                <div className="space-y-2">
                  {context.characters.length === 0 ? (
                    <p className="text-xs text-surface-500">暂无关联角色</p>
                  ) : (
                    context.characters.map((c) => (
                      <div key={c.id} className="p-2 bg-surface-700/50 rounded-lg">
                        <div className="text-sm font-medium text-surface-200">{c.name}</div>
                        <div className="text-xs text-surface-400">{c.roleType}</div>
                        {c.motivation && (
                          <div className="text-xs text-surface-500 mt-1">
                            动机: {c.motivation.slice(0, 60)}
                          </div>
                        )}
                      </div>
                    ))
                  )}
                </div>
              )}

              {/* World Tab */}
              {currentTab === "world" && (
                <div className="space-y-2">
                  {context.worldRules.length === 0 ? (
                    <p className="text-xs text-surface-500">暂无设定</p>
                  ) : (
                    context.worldRules.map((w) => (
                      <div key={w.id} className="p-2 bg-surface-700/50 rounded-lg">
                        <div className="text-sm font-medium text-surface-200">{w.title}</div>
                        <div className="text-xs text-surface-400">{w.category}</div>
                        <div className="text-xs text-surface-500 mt-1">{w.description.slice(0, 80)}</div>
                      </div>
                    ))
                  )}
                </div>
              )}

              {/* Plot Tab */}
              {currentTab === "plot" && (
                <div className="space-y-2">
                  {context.plotNodes.length === 0 ? (
                    <p className="text-xs text-surface-500">暂无主线节点</p>
                  ) : (
                    context.plotNodes.map((p) => (
                      <div key={p.id} className="p-2 bg-surface-700/50 rounded-lg">
                        <div className="flex items-center gap-2">
                          <span className="text-xs text-surface-500">#{p.sortOrder}</span>
                          <span className="text-sm font-medium text-surface-200">{p.title}</span>
                        </div>
                        <span className="text-xs text-surface-400">{p.nodeType}</span>
                        {p.goal && <div className="text-xs text-surface-500 mt-1">{p.goal.slice(0, 60)}</div>}
                      </div>
                    ))
                  )}
                </div>
              )}

              {/* Glossary Tab */}
              {currentTab === "glossary" && (
                <div className="space-y-2">
                  {context.glossary.length === 0 ? (
                    <p className="text-xs text-surface-500">暂无名词</p>
                  ) : (
                    context.glossary.map((g, i) => (
                      <div key={i} className="flex items-center gap-2 p-2 bg-surface-700/50 rounded-lg">
                        <span className="text-sm font-medium text-surface-200">{g.term}</span>
                        <span className="text-xs text-surface-400">{g.termType}</span>
                        {g.locked && <span className="text-xs text-info ml-auto">锁定</span>}
                        {g.banned && <span className="text-xs text-error ml-auto">禁用</span>}
                      </div>
                    ))
                  )}
                </div>
              )}

              {/* Chapter Info */}
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
                                onClick={() => void handleApplyCandidate(candidate, action.targetKind)}
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
                      const key = getStructuredDraftKey("relationship", draft.sourceLabel, draft.targetLabel);
                      const status = structuredDraftStatus[key] ?? "idle";
                      return (
                        <div key={key} className="p-2 bg-surface-700/50 rounded-lg">
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
                          <button
                            onClick={() => void handleApplyRelationshipDraft(draft)}
                            disabled={status === "applying"}
                            className="mt-2 px-2 py-1 text-[11px] bg-surface-800 text-surface-200 border border-surface-600 rounded hover:bg-surface-700 disabled:opacity-40 transition-colors"
                          >
                            确认入库
                          </button>
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
                      const key = getStructuredDraftKey("involvement", draft.characterLabel);
                      const status = structuredDraftStatus[key] ?? "idle";
                      return (
                        <div key={key} className="p-2 bg-surface-700/50 rounded-lg">
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
                          <button
                            onClick={() => void handleApplyInvolvementDraft(draft)}
                            disabled={status === "applying"}
                            className="mt-2 px-2 py-1 text-[11px] bg-surface-800 text-surface-200 border border-surface-600 rounded hover:bg-surface-700 disabled:opacity-40 transition-colors"
                          >
                            确认入库
                          </button>
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
                      const key = getStructuredDraftKey("scene", draft.sceneLabel);
                      const status = structuredDraftStatus[key] ?? "idle";
                      return (
                        <div key={key} className="p-2 bg-surface-700/50 rounded-lg">
                          <div className="flex items-center justify-between gap-2">
                            <span className="text-sm text-surface-200">{draft.sceneLabel}</span>
                            <span className="text-[11px] text-surface-500">
                              {STRUCTURED_DRAFT_STATUS_LABEL[status]}
                            </span>
                          </div>
                          <p className="text-[11px] text-primary mt-1">{draft.sceneType}</p>
                          <p className="text-xs text-surface-400 mt-1">{draft.evidence}</p>
                          <button
                            onClick={() => void handleApplySceneDraft(draft)}
                            disabled={status === "applying"}
                            className="mt-2 px-2 py-1 text-[11px] bg-surface-800 text-surface-200 border border-surface-600 rounded hover:bg-surface-700 disabled:opacity-40 transition-colors"
                          >
                            确认入库
                          </button>
                        </div>
                      );
                    })
                  )}
                </div>
              </div>

              <div className="mt-3 pt-3 border-t border-surface-700">
                <div className="text-xs text-surface-400">
                  <div>目标字数: {context.chapter.targetWords.toLocaleString()}</div>
                  <div>当前字数: {context.chapter.currentWords.toLocaleString()}</div>
                  <div>状态: {context.chapter.status}</div>
                  {context.previousChapterSummary && (
                    <div className="mt-2">
                      <div className="text-surface-500 mb-1">前章摘要:</div>
                      <div className="text-surface-400">{context.previousChapterSummary.slice(0, 100)}</div>
                    </div>
                  )}
                </div>
              </div>
            </>
          )}
        </Card>
      </div>

      <Modal open={showSnapshotModal} onClose={() => setShowSnapshotModal(false)} title="章节快照" width="lg">
        {!chapterId ? (
          <p className="text-sm text-surface-400">请先选择章节</p>
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-[280px_1fr] gap-4">
            <div className="space-y-3">
              <Input
                label="快照标题（可选）"
                value={snapshotTitle}
                onChange={(e) => setSnapshotTitle(e.target.value)}
                placeholder="例如：改稿前"
              />
              <Textarea
                label="备注（可选）"
                value={snapshotNote}
                onChange={(e) => setSnapshotNote(e.target.value)}
                placeholder="记录这次快照目的"
              />
              <Button variant="primary" onClick={() => void handleCreateSnapshot()} loading={creatingSnapshot}>
                创建快照
              </Button>

              <div className="border-t border-surface-700 pt-3">
                <h4 className="text-xs font-semibold text-surface-400 uppercase tracking-wider mb-2">
                  历史快照
                </h4>
                <div className="max-h-64 overflow-y-auto space-y-1">
                  {snapshotLoading && snapshots.length === 0 ? (
                    <p className="text-xs text-surface-500">加载中...</p>
                  ) : snapshots.length === 0 ? (
                    <p className="text-xs text-surface-500">暂无快照</p>
                  ) : (
                    snapshots.map((snapshot) => (
                      <button
                        key={snapshot.id}
                        onClick={() => void handleSelectSnapshot(snapshot.id)}
                        className={`w-full text-left px-2 py-2 rounded text-xs border transition-colors ${
                          selectedSnapshotId === snapshot.id
                            ? "bg-primary/10 border-primary/30 text-primary"
                            : "bg-surface-800 border-surface-700 text-surface-300 hover:bg-surface-700"
                        }`}
                      >
                        <div className="truncate">{snapshot.title || "未命名快照"}</div>
                        <div className="text-[11px] text-surface-500 mt-1">
                          {new Date(snapshot.createdAt).toLocaleString("zh-CN")}
                        </div>
                      </button>
                    ))
                  )}
                </div>
              </div>
            </div>

            <div className="min-h-[360px] border border-surface-700 rounded-lg bg-surface-900 p-3">
              {!selectedSnapshotId ? (
                <p className="text-sm text-surface-500">选择左侧快照以查看内容</p>
              ) : snapshotLoading ? (
                <p className="text-sm text-surface-500">读取中...</p>
              ) : (
                <pre className="text-xs text-surface-200 whitespace-pre-wrap break-words leading-relaxed">
                  {snapshotContent}
                </pre>
              )}
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
