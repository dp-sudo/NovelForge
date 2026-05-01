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
  readChapterContent,
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
import type { ChapterRecord } from "../../api/chapterApi";
import { Modal } from "../../components/dialogs/Modal";
import { Input } from "../../components/forms/Input";
import { Select } from "../../components/forms/Select";
import { Textarea } from "../../components/forms/Textarea";
import { Button } from "../../components/ui/Button.js";
import { FindBar } from "../../components/editor/FindBar.js";
import { canonicalTaskType, getTaskRequirements } from "../../utils/taskRouting.js";
import { loadEditorChapterContentWithRecovery } from "./chapterLoadFlow.js";
import { EditorContextPanel, type EditorCandidateTargetKind } from "./components/EditorContextPanel";
import { usePipelineStream } from "./hooks/usePipelineStream";

const AUTOSAVE_DELAY_MS = 5000;

const STATUS_BADGE: Record<string, { variant: "default" | "success" | "warning" | "error" | "info"; label: string }> = {
  saved: { variant: "success", label: "已保存" },
  saving: { variant: "info", label: "保存中..." },
  unsaved: { variant: "warning", label: "未保存" },
  autosaving: { variant: "info", label: "自动保存..." },
  error: { variant: "error", label: "保存失败" }
};


export function EditorPage() {
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const selRef = useRef<{ start: number; end: number }>({ start: 0, end: 0 });
  const autosaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastChapterIdRef = useRef<string | null>(null);
  const hydratingContentRef = useRef(false);

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
  const [structuredDraftStatus, setStructuredDraftStatus] = useState<Record<string, "applying" | "error">>({});

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
  const setAiStreamStatus = useEditorStore((s) => s.setAiStreamStatus);
  const setAiStreamError = useEditorStore((s) => s.setAiStreamError);
  const appendAiPreviewContent = useEditorStore((s) => s.appendAiPreviewContent);
  const setAiRequestId = useEditorStore((s) => s.setAiRequestId);
  const store = useEditorStore();
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const { startPipeline, cancelActivePipeline, formatPipelineError } = usePipelineStream({
    setAiRequestId,
    setAiStreamStatus,
    setAiStreamError,
    appendAiPreviewContent,
  });

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

  const refreshChapterContext = useCallback(async (root: string, cid: string): Promise<ChapterContext | null> => {
    try {
      const ctx = await getChapterContext(root, cid);
      setContext(ctx);
      return ctx;
    } catch {
      setContext(null);
      return null;
    }
  }, []);

  const getCandidateKey = useCallback((assetType: string, label: string) => `${assetType}:${label}`, []);
  const normalizeDraftItemId = useCallback((rawId: string): string | undefined => {
    const normalized = rawId.trim();
    if (!normalized || normalized.startsWith("ephemeral:")) {
      return undefined;
    }
    return normalized;
  }, []);
  const getStructuredDraftDisplayStatus = useCallback(
    (draftId: string, persistedStatus: string): "pending" | "applying" | "applied" | "rejected" | "error" => {
      const transient = structuredDraftStatus[draftId];
      if (transient === "applying" || transient === "error") {
        return transient;
      }
      if (persistedStatus === "applied") {
        return "applied";
      }
      if (persistedStatus === "rejected") {
        return "rejected";
      }
      return "pending";
    },
    [structuredDraftStatus]
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
    const cid: string = chapterId;
    const root: string = projectRoot;
    let cancelled = false;
    const setLoadedContent = (value: string) => {
      // 问题1修复(1/3): 切章/进入编辑器时先写入“正式正文”，避免空内容覆盖已有章节。
      hydratingContentRef.current = true;
      store.setContent(value);
      store.setIsDirty(false);
      store.setSaveStatus("saved");
    };
    async function loadChapterRuntimeData() {
      store.setIsDirty(false);
      store.setSaveStatus("saved");
      const { persistedContent, recoveryContent } = await loadEditorChapterContentWithRecovery({
        chapterId: cid,
        projectRoot: root,
        readChapterContent,
        recoverDraft,
      });
      if (!cancelled) {
        // 问题1修复(2/3): 先读取正式正文，再进行草稿恢复决策，防止恢复提示与正文加载顺序冲突。
        setLoadedContent(persistedContent);
        if (recoveryContent) {
          setRecoveryContent(recoveryContent);
          setShowRecovery(true);
        } else {
          setShowRecovery(false);
        }
      }
      if (!cancelled) {
        await refreshChapterContext(root, cid);
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
    // 问题1修复(3/3): 加载正式正文时不标记 unsaved，避免误触发自动保存覆盖已有内容。
    if (hydratingContentRef.current) {
      hydratingContentRef.current = false;
      return;
    }
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
  useEffect(() => {
    if (lastChapterIdRef.current !== null && lastChapterIdRef.current !== chapterId) {
      void cancelActivePipeline("chapter_change");
    }
    lastChapterIdRef.current = chapterId;
  }, [chapterId, cancelActivePipeline]);

  async function handleSave() {
    if (!chapterId || !projectRoot) return;
    store.setSaveStatus("saving");
    const previousStateSignature = JSON.stringify(context?.stateSummary ?? []);
    try {
      const result = await saveChapterContent(chapterId, content, projectRoot);
      store.setSaveStatus("saved");
      store.setLastSavedAt(result.updatedAt);
      store.setIsDirty(false);
      await load();
      const nextContext = await refreshChapterContext(projectRoot, chapterId);
      if (nextContext) {
        const nextStateSignature = JSON.stringify(nextContext.stateSummary);
        if (nextStateSignature !== previousStateSignature) {
          setEditorNotice(`状态账本已更新：${nextContext.stateSummary.length} 条摘要`);
        }
      }
      setCandidateStatus({});
      setStructuredDraftStatus({});
    } catch {
      store.setSaveStatus("error");
    }
  }

  function handleSelectChapter(ch: ChapterRecord) {
    void cancelActivePipeline("select_chapter");
    store.setActiveChapter(ch.id, ch.title);
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

  async function handleAiCommand(taskType: string, userInstruction: string) {
    if (!chapterId || !projectRoot) return;

    const canonicalTask = canonicalTaskType(taskType);
    const requirements = getTaskRequirements(canonicalTask);
    const selectedText = selRef.current.start !== selRef.current.end
      ? content.slice(selRef.current.start, selRef.current.end)
      : undefined;
    const trimmedInstruction = userInstruction.trim();
    const selectedTextTrimmed = selectedText?.trim() || "";

    if (requirements.requiresSelectedText && !selectedTextTrimmed) {
      setShowAiPanel(true);
      setOriginalText(undefined);
      store.resetAiPreview();
      store.setAiTaskType(canonicalTask);
      store.setAiStreamStatus("error");
      store.setAiStreamError(
        formatPipelineError({
          phase: "validate",
          errorCode: "PIPELINE_SELECTED_TEXT_REQUIRED",
          message: "请先选中需要处理的文本"
        })
      );
      return;
    }

    if (requirements.requiresUserInstruction && !trimmedInstruction) {
      setShowAiPanel(true);
      setOriginalText(undefined);
      store.resetAiPreview();
      store.setAiTaskType(canonicalTask);
      store.setAiStreamStatus("error");
      store.setAiStreamError(
        formatPipelineError({
          phase: "validate",
          errorCode: "PIPELINE_USER_INSTRUCTION_REQUIRED",
          message: "请先输入任务描述"
        })
      );
      return;
    }

    setShowAiPanel(true);
    setOriginalText(selectedText);
    store.resetAiPreview();
    store.setAiTaskType(canonicalTask);
    store.setAiStreamStatus("streaming");
    store.setAiStreamError(null);

    await startPipeline({
      projectRoot,
      chapterId,
      taskType: canonicalTask,
      userInstruction: trimmedInstruction,
      selectedText,
      chapterContent: requirements.requiresChapterContent ? content : undefined,
    });
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
    if (aiStreamStatus === "streaming") {
      void cancelActivePipeline("discard");
    }
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

  function getCandidateActions(assetType: string): Array<{ label: string; targetKind: EditorCandidateTargetKind }> {
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
    targetKind: EditorCandidateTargetKind
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
    id: string;
    status: string;
    sourceLabel: string;
    targetLabel: string;
    relationshipType: string;
    evidence: string;
  }) {
    if (!projectRoot || !chapterId) return;
    setStructuredDraftStatus((prev) => ({ ...prev, [draft.id]: "applying" }));
    try {
      const result = await applyStructuredDraft(projectRoot, chapterId, {
        draftItemId: normalizeDraftItemId(draft.id),
        draftKind: "relationship",
        sourceLabel: draft.sourceLabel,
        targetLabel: draft.targetLabel,
        relationshipType: draft.relationshipType,
        evidence: draft.evidence,
      });
      setStructuredDraftStatus((prev) => {
        const next = { ...prev };
        delete next[draft.id];
        return next;
      });
      setEditorNotice(
        result.action === "created"
          ? `关系已入库：${draft.sourceLabel} - ${draft.targetLabel}`
          : `关系已存在：${draft.sourceLabel} - ${draft.targetLabel}`
      );
      await refreshChapterContext(projectRoot, chapterId);
    } catch (err) {
      setStructuredDraftStatus((prev) => ({ ...prev, [draft.id]: "error" }));
      setEditorNotice(err instanceof Error ? err.message : "关系草案入库失败");
    }
  }

  async function handleApplyInvolvementDraft(draft: {
    id: string;
    status: string;
    characterLabel: string;
    involvementType: string;
    evidence: string;
  }) {
    if (!projectRoot || !chapterId) return;
    setStructuredDraftStatus((prev) => ({ ...prev, [draft.id]: "applying" }));
    try {
      const result = await applyStructuredDraft(projectRoot, chapterId, {
        draftItemId: normalizeDraftItemId(draft.id),
        draftKind: "involvement",
        sourceLabel: draft.characterLabel,
        involvementType: draft.involvementType,
        evidence: draft.evidence,
      });
      setStructuredDraftStatus((prev) => {
        const next = { ...prev };
        delete next[draft.id];
        return next;
      });
      setEditorNotice(
        result.action === "created"
          ? `戏份已入库：${draft.characterLabel}`
          : `戏份已存在：${draft.characterLabel}`
      );
      await refreshChapterContext(projectRoot, chapterId);
    } catch (err) {
      setStructuredDraftStatus((prev) => ({ ...prev, [draft.id]: "error" }));
      setEditorNotice(err instanceof Error ? err.message : "戏份草案入库失败");
    }
  }

  async function handleApplySceneDraft(draft: {
    id: string;
    status: string;
    sceneLabel: string;
    sceneType: string;
    evidence: string;
  }) {
    if (!projectRoot || !chapterId) return;
    setStructuredDraftStatus((prev) => ({ ...prev, [draft.id]: "applying" }));
    try {
      const result = await applyStructuredDraft(projectRoot, chapterId, {
        draftItemId: normalizeDraftItemId(draft.id),
        draftKind: "scene",
        sourceLabel: draft.sceneLabel,
        sceneType: draft.sceneType,
        evidence: draft.evidence,
      });
      setStructuredDraftStatus((prev) => {
        const next = { ...prev };
        delete next[draft.id];
        return next;
      });
      setEditorNotice(
        result.action === "created"
          ? `场景已入库：${draft.sceneLabel}`
          : `场景已存在：${draft.sceneLabel}`
      );
      await refreshChapterContext(projectRoot, chapterId);
    } catch (err) {
      setStructuredDraftStatus((prev) => ({ ...prev, [draft.id]: "error" }));
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
      <EditorContextPanel
        chapterId={chapterId}
        context={context}
        candidateStatus={candidateStatus}
        getCandidateKey={getCandidateKey}
        getCandidateActions={getCandidateActions}
        getStructuredDraftDisplayStatus={getStructuredDraftDisplayStatus}
        onApplyCandidate={handleApplyCandidate}
        onApplyRelationshipDraft={handleApplyRelationshipDraft}
        onApplyInvolvementDraft={handleApplyInvolvementDraft}
        onApplySceneDraft={handleApplySceneDraft}
      />

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
