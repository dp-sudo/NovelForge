import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useUiStore } from "../../stores/uiStore.js";
import { useEditorStore } from "../../stores/editorStore.js";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Select } from "../../components/forms/Select.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { ConfirmDialog } from "../../components/dialogs/ConfirmDialog.js";
import { Badge } from "../../components/ui/Badge.js";
import {
  assignChapterVolume,
  createChapter,
  createVolume,
  deleteChapter,
  deleteVolume,
  importChapterFiles,
  listChapters,
  reorderChapters,
  listVolumes,
  type ChapterRecord,
  type VolumeRecord,
} from "../../api/chapterApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

const statusColors: Record<string, "default" | "success" | "warning" | "error" | "info"> = {
  planned: "info",
  drafting: "warning",
  revising: "warning",
  completed: "success",
  archived: "default"
};

const statusLabels: Record<string, string> = {
  planned: "规划中",
  drafting: "写作中",
  revising: "待修订",
  completed: "已完成",
  archived: "已归档"
};

type VolumeFilter = "all" | "unassigned" | string;

export function ChaptersPage() {
  const [chapters, setChapters] = useState<ChapterRecord[]>([]);
  const [volumes, setVolumes] = useState<VolumeRecord[]>([]);
  const [reordering, setReordering] = useState(false);
  const [draggingChapterId, setDraggingChapterId] = useState<string | null>(null);
  const [dragOverChapterId, setDragOverChapterId] = useState<string | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [showDelete, setShowDelete] = useState<string | null>(null);
  const [showCreateVolume, setShowCreateVolume] = useState(false);
  const [creatingVolume, setCreatingVolume] = useState(false);
  const [importing, setImporting] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [volumeFilter, setVolumeFilter] = useState<VolumeFilter>("all");
  const [form, setForm] = useState({ title: "", summary: "", targetWords: 3000 });
  const [volumeForm, setVolumeForm] = useState({ title: "", description: "" });
  const fileInputRef = useRef<HTMLInputElement>(null);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const setActiveChapter = useEditorStore((s) => s.setActiveChapter);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const dragInProgressRef = useRef(false);

  const visibleChapters = useMemo(() => {
    if (volumeFilter === "all") return chapters;
    if (volumeFilter === "unassigned") {
      return chapters.filter((chapter) => !chapter.volumeId);
    }
    return chapters.filter((chapter) => chapter.volumeId === volumeFilter);
  }, [chapters, volumeFilter]);
  const canDragSort = volumeFilter === "all" && chapters.length > 1;

  function reorderChapterList(source: ChapterRecord[], fromId: string, toId: string): ChapterRecord[] {
    const fromIndex = source.findIndex((chapter) => chapter.id === fromId);
    const toIndex = source.findIndex((chapter) => chapter.id === toId);
    if (fromIndex === -1 || toIndex === -1 || fromIndex === toIndex) {
      return source;
    }
    const next = [...source];
    const [moved] = next.splice(fromIndex, 1);
    next.splice(toIndex, 0, moved);
    return next;
  }

  const load = useCallback(async () => {
    if (!projectRoot) {
      setChapters([]);
      setVolumes([]);
      return;
    }
    const [chapterRows, volumeRows] = await Promise.all([
      listChapters(projectRoot),
      listVolumes(projectRoot),
    ]);
    setChapters(chapterRows);
    setVolumes(volumeRows);
  }, [projectRoot]);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleCreateChapter() {
    if (!form.title.trim() || !projectRoot) return;
    await createChapter({
      title: form.title.trim(),
      summary: form.summary || undefined,
      targetWords: form.targetWords
    }, projectRoot);
    setForm({ title: "", summary: "", targetWords: 3000 });
    setShowNew(false);
    setMessage("章节已创建");
    await load();
  }

  async function handleCreateVolume() {
    if (!projectRoot || !volumeForm.title.trim()) return;
    setCreatingVolume(true);
    setError(null);
    try {
      await createVolume(projectRoot, volumeForm.title.trim(), volumeForm.description.trim() || undefined);
      setVolumeForm({ title: "", description: "" });
      setShowCreateVolume(false);
      setMessage("卷已创建");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建卷失败");
    } finally {
      setCreatingVolume(false);
    }
  }

  async function handleDeleteVolume(id: string) {
    if (!projectRoot) return;
    if (!window.confirm("删除卷会将该卷下章节改为未分卷，确定继续？")) {
      return;
    }
    setError(null);
    try {
      await deleteVolume(projectRoot, id);
      setMessage("卷已删除");
      if (volumeFilter === id) {
        setVolumeFilter("all");
      }
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除卷失败");
    }
  }

  async function handleAssignVolume(chapterId: string, volumeId: string) {
    if (!projectRoot) return;
    setError(null);
    try {
      await assignChapterVolume(projectRoot, chapterId, volumeId || undefined);
      setChapters((prev) =>
        prev.map((chapter) =>
          chapter.id === chapterId ? { ...chapter, volumeId: volumeId || null } : chapter,
        ),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : "章节归卷失败");
    }
  }

  function handleDragStart(event: React.DragEvent<HTMLButtonElement>, chapterId: string) {
    if (!canDragSort || reordering) {
      event.preventDefault();
      return;
    }
    dragInProgressRef.current = true;
    setDraggingChapterId(chapterId);
    setDragOverChapterId(chapterId);
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", chapterId);
  }

  function handleDragOver(event: React.DragEvent<HTMLTableRowElement>, chapterId: string) {
    if (!draggingChapterId || draggingChapterId === chapterId || !canDragSort || reordering) {
      return;
    }
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    if (dragOverChapterId !== chapterId) {
      setDragOverChapterId(chapterId);
    }
  }

  function handleDragLeave(chapterId: string) {
    if (dragOverChapterId === chapterId) {
      setDragOverChapterId(null);
    }
  }

  function handleDragEnd() {
    setDraggingChapterId(null);
    setDragOverChapterId(null);
    window.setTimeout(() => {
      dragInProgressRef.current = false;
    }, 0);
  }

  async function handleDrop(event: React.DragEvent<HTMLTableRowElement>, targetChapterId: string) {
    event.preventDefault();
    event.stopPropagation();
    if (!projectRoot || !draggingChapterId || draggingChapterId === targetChapterId || !canDragSort || reordering) {
      handleDragEnd();
      return;
    }

    const previous = chapters;
    const next = reorderChapterList(previous, draggingChapterId, targetChapterId);
    handleDragEnd();
    if (next === previous) {
      return;
    }

    setError(null);
    setMessage(null);
    setReordering(true);
    setChapters(next);
    try {
      await reorderChapters(projectRoot, next.map((chapter) => chapter.id));
      await load();
      setMessage("章节顺序已更新");
    } catch (err) {
      setChapters(previous);
      setError(err instanceof Error ? err.message : "章节重排失败");
    } finally {
      setReordering(false);
    }
  }

  function handleOpenEditor(chapter: ChapterRecord) {
    if (dragInProgressRef.current) {
      return;
    }
    // 章节页只负责队列管理，真正的生产入口统一回到指挥台当前作业区。
    setActiveChapter(chapter.id, chapter.title);
    setActiveRoute("command-center");
  }

  async function handleImportFiles(event: React.ChangeEvent<HTMLInputElement>) {
    if (!projectRoot) return;

    const files = Array.from(event.target.files ?? []);
    if (files.length === 0) return;
    const supported = files.filter((file) => /\.(txt|md)$/i.test(file.name));
    if (supported.length === 0) {
      setError("仅支持 TXT / MD 文件");
      return;
    }

    setImporting(true);
    setMessage(null);
    setError(null);

    try {
      const entries = await Promise.all(
        supported.map(async (file) => ({
          file_name: file.name,
          content: await file.text(),
        })),
      );
      const result = await importChapterFiles(projectRoot, entries);
      setMessage(`导入完成：${result.importedCount} 个章节`);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "批量导入失败");
    } finally {
      setImporting(false);
      event.target.value = "";
    }
  }

  const volumeFilterOptions = [
    { value: "all", label: "全部章节" },
    { value: "unassigned", label: "未分卷" },
    ...volumes.map((volume) => ({ value: volume.id, label: volume.title })),
  ];

  return (
    <div className="max-w-6xl mx-auto space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-surface-100">章节</h1>
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => fileInputRef.current?.click()}
            loading={importing}
            disabled={!projectRoot}
          >
            {importing ? "导入中..." : "批量导入"}
          </Button>
          <Button variant="secondary" size="sm" onClick={() => setShowCreateVolume(true)} disabled={!projectRoot}>
            新建卷
          </Button>
          <Button variant="primary" size="sm" onClick={() => setShowNew(true)} disabled={!projectRoot}>
            新建章节
          </Button>
        </div>
      </div>

      <div className="rounded-lg border border-surface-700 bg-surface-800/70 px-3 py-2">
        <p className="text-xs text-surface-400">
          本页只负责章节队列和分卷维护。进入具体生产请回到全书指挥台的当前作业区。
        </p>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept=".txt,.md,text/plain,text/markdown"
        multiple
        className="hidden"
        onChange={(e) => void handleImportFiles(e)}
      />

      {(message || error) && (
        <Card
          padding="sm"
          className={error ? "border border-error/30 bg-error/10 text-error" : "border border-success/30 bg-success/10 text-success"}
        >
          <p className="text-sm">{error ?? message}</p>
        </Card>
      )}

      <Card padding="md" className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-surface-200">卷管理</h2>
          <div className="w-56">
            <Select
              value={volumeFilter}
              onChange={(e) => setVolumeFilter(e.target.value)}
              options={volumeFilterOptions}
            />
          </div>
        </div>

        {volumes.length === 0 ? (
          <p className="text-sm text-surface-500">暂无卷，章节默认归入“未分卷”。</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {volumes.map((volume) => (
              <div key={volume.id} className="inline-flex items-center gap-2 px-3 py-1.5 rounded-lg bg-surface-800 border border-surface-700">
                <span className="text-sm text-surface-200">{volume.title}</span>
                <span className="text-xs text-surface-500">{volume.chapterCount} 章</span>
                <button
                  onClick={() => void handleDeleteVolume(volume.id)}
                  className="text-xs text-surface-500 hover:text-error transition-colors"
                >
                  删除
                </button>
              </div>
            ))}
          </div>
        )}
      </Card>

      {chapters.length === 0 ? (
        <Card padding="lg" className="text-center">
          <p className="text-sm text-surface-400 mb-4">还没有章节，先新建或批量导入 TXT/MD。</p>
          <div className="flex items-center justify-center gap-2">
            <Button variant="secondary" size="sm" onClick={() => fileInputRef.current?.click()} loading={importing}>
              {importing ? "导入中..." : "批量导入"}
            </Button>
            <Button variant="primary" size="sm" onClick={() => setShowNew(true)}>新建章节</Button>
          </div>
        </Card>
      ) : (
        <Card padding="none">
          <div className="px-4 py-2 border-b border-surface-700/60 text-xs text-surface-500">
            {canDragSort
              ? reordering
                ? "正在保存章节顺序..."
                : "可拖拽左侧“≡”手柄调整章节顺序"
              : "仅在“全部章节”视图可拖拽排序"}
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-surface-700 text-surface-400 text-xs uppercase">
                  <th className="text-left px-4 py-3 font-medium w-12">拖拽</th>
                  <th className="text-left px-4 py-3 font-medium w-16">序号</th>
                  <th className="text-left px-4 py-3 font-medium">标题</th>
                  <th className="text-left px-4 py-3 font-medium w-24">状态</th>
                  <th className="text-left px-4 py-3 font-medium w-44">所属卷</th>
                  <th className="text-right px-4 py-3 font-medium w-20">字数</th>
                  <th className="text-left px-4 py-3 font-medium max-w-[220px]">摘要</th>
                  <th className="text-right px-4 py-3 font-medium w-28">最近编辑</th>
                  <th className="text-center px-4 py-3 font-medium w-20">操作</th>
                </tr>
              </thead>
              <tbody>
                {visibleChapters.map((chapter) => (
                  <tr
                    key={chapter.id}
                    className={`border-b border-surface-700/50 cursor-pointer ${
                      dragOverChapterId === chapter.id && draggingChapterId !== chapter.id
                        ? "bg-primary/10"
                        : "hover:bg-surface-800/50"
                    }`}
                    onClick={() => handleOpenEditor(chapter)}
                    onDragOver={(event) => handleDragOver(event, chapter.id)}
                    onDrop={(event) => void handleDrop(event, chapter.id)}
                    onDragLeave={() => handleDragLeave(chapter.id)}
                  >
                    <td className="px-4 py-3 text-surface-500">
                      <button
                        type="button"
                        draggable={canDragSort && !reordering}
                        onDragStart={(event) => handleDragStart(event, chapter.id)}
                        onDragEnd={handleDragEnd}
                        onClick={(event) => event.stopPropagation()}
                        disabled={!canDragSort || reordering}
                        title={canDragSort ? "拖拽调整顺序" : "仅在“全部章节”视图可排序"}
                        className={`inline-flex h-6 w-6 items-center justify-center rounded text-sm transition-colors ${
                          canDragSort && !reordering
                            ? "hover:bg-surface-700 text-surface-400 cursor-grab active:cursor-grabbing"
                            : "text-surface-600 cursor-not-allowed"
                        }`}
                      >
                        ≡
                      </button>
                    </td>
                    <td className="px-4 py-3 text-surface-400">#{chapter.chapterIndex}</td>
                    <td className="px-4 py-3 text-surface-100 font-medium">{chapter.title}</td>
                    <td className="px-4 py-3">
                      <Badge variant={statusColors[chapter.status] ?? "default"}>
                        {statusLabels[chapter.status] ?? chapter.status}
                      </Badge>
                    </td>
                    <td className="px-4 py-3">
                      <select
                        value={chapter.volumeId ?? ""}
                        onChange={(e) => {
                          e.stopPropagation();
                          void handleAssignVolume(chapter.id, e.target.value);
                        }}
                        onClick={(e) => e.stopPropagation()}
                        className="w-full px-2 py-1 text-xs bg-surface-800 border border-surface-600 rounded text-surface-100"
                      >
                        <option value="">未分卷</option>
                        {volumes.map((volume) => (
                          <option key={volume.id} value={volume.id}>
                            {volume.title}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="px-4 py-3 text-right text-surface-300">{chapter.currentWords.toLocaleString()}</td>
                    <td className="px-4 py-3 text-surface-400 max-w-[220px] truncate">{chapter.summary || "-"}</td>
                    <td className="px-4 py-3 text-right text-surface-400 text-xs">
                      {new Date(chapter.updatedAt).toLocaleDateString("zh-CN")}
                    </td>
                    <td className="px-4 py-3 text-center">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setShowDelete(chapter.id);
                        }}
                        className="text-xs text-surface-400 hover:text-error transition-colors"
                      >
                        删除
                      </button>
                    </td>
                  </tr>
                ))}
                {visibleChapters.length === 0 && (
                  <tr>
                    <td colSpan={9} className="px-4 py-8 text-center text-sm text-surface-500">
                      当前筛选下没有章节
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </Card>
      )}

      <Modal open={showNew} onClose={() => setShowNew(false)} title="新建章节" width="sm">
        <div className="space-y-4">
          <Input
            label="章节标题 *"
            value={form.title}
            onChange={(e) => setForm({ ...form, title: e.target.value })}
            placeholder="第一章 风起"
          />
          <Textarea
            label="章节摘要"
            value={form.summary}
            onChange={(e) => setForm({ ...form, summary: e.target.value })}
            placeholder="本章主要内容…"
          />
          <Input
            label="目标字数"
            type="number"
            value={form.targetWords}
            onChange={(e) => setForm({ ...form, targetWords: Number(e.target.value) })}
            min={500}
            step={500}
          />
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleCreateChapter()} disabled={!form.title.trim()}>创建</Button>
          </div>
        </div>
      </Modal>

      <Modal open={showCreateVolume} onClose={() => setShowCreateVolume(false)} title="新建卷" width="sm">
        <div className="space-y-4">
          <Input
            label="卷标题 *"
            value={volumeForm.title}
            onChange={(e) => setVolumeForm((prev) => ({ ...prev, title: e.target.value }))}
            placeholder="第一卷 初入江湖"
          />
          <Textarea
            label="卷描述"
            value={volumeForm.description}
            onChange={(e) => setVolumeForm((prev) => ({ ...prev, description: e.target.value }))}
            placeholder="卷目标、节奏与主题"
          />
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowCreateVolume(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleCreateVolume()} loading={creatingVolume} disabled={!volumeForm.title.trim()}>
              创建卷
            </Button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        open={showDelete !== null}
        title="删除章节"
        message="确定删除该章节吗？此操作不可撤销。"
        variant="danger"
        confirmLabel="删除"
        onConfirm={() => {
          if (showDelete && projectRoot) {
            void deleteChapter(showDelete, projectRoot).then(async () => {
              setMessage("章节已删除");
              await load();
            });
          }
          setShowDelete(null);
        }}
        onCancel={() => setShowDelete(null)}
      />
    </div>
  );
}
