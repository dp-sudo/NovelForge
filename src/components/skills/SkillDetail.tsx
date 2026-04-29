import { useEffect, useState } from "react";
import { useSkillStore } from "../../stores/skillStore.js";
import { SkillEditor } from "./SkillEditor.js";
import { Button } from "../ui/Button.js";
import { getSkillContent, updateSkill, deleteSkill, resetBuiltinSkill } from "../../api/skillsApi.js";
import type { SkillManifest } from "../../api/skillsApi.js";

interface SkillDetailProps {
  skill: SkillManifest;
  onDeleted: () => void;
  onUpdated: (s: SkillManifest) => void;
}

export function SkillDetail({ skill, onDeleted, onUpdated }: SkillDetailProps) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [resetting, setResetting] = useState(false);
  const setStoreError = useSkillStore((s) => s.setError);
  const selectedId = useSkillStore((s) => s.selectedId);

  useEffect(() => {
    setEditing(false);
    setContent(null);
    setLoading(true);
    setError(null);
    getSkillContent(skill.id)
      .then((c) => { setContent(c); setLoading(false); })
      .catch((e) => {
        const msg = typeof e === "object" && e && "message" in e ? String((e as {message:string}).message) : "加载失败";
        setError(msg);
        setLoading(false);
      });
  }, [skill.id]);

  async function handleSave(body: string) {
    const updated = await updateSkill(skill.id, body);
    setContent(body);
    setEditing(false);
    onUpdated(updated);
  }

  async function handleDelete() {
    if (!confirm(`确定删除技能「${skill.name}」？此操作不可撤销。`)) return;
    setDeleting(true);
    try {
      await deleteSkill(skill.id);
      onDeleted();
    } catch (e: unknown) {
      const msg = typeof e === "object" && e && "message" in e ? String((e as {message:string}).message) : "删除失败";
      setStoreError(msg);
    } finally {
      setDeleting(false);
    }
  }

  async function handleReset() {
    if (!confirm(`重置「${skill.name}」到出厂版本？自定义修改将丢失。`)) return;
    setResetting(true);
    try {
      const restored = await resetBuiltinSkill(skill.id);
      setContent(await getSkillContent(skill.id));
      setEditing(false);
      onUpdated(restored);
    } catch (e: unknown) {
      const msg = typeof e === "object" && e && "message" in e ? String((e as {message:string}).message) : "重置失败";
      setStoreError(msg);
    } finally {
      setResetting(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-sm text-surface-500">
        加载中...
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-sm text-error">
        {error}
      </div>
    );
  }

  if (editing && content !== null) {
    return (
      <SkillEditor
        initialContent={content}
        onSave={handleSave}
        onCancel={() => setEditing(false)}
      />
    );
  }

  const isBuiltin = skill.source === "builtin";

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-start justify-between mb-4 shrink-0">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            {skill.icon && <span className="text-xl">{skill.icon}</span>}
            <h2 className="text-base font-semibold text-surface-100 truncate">{skill.name}</h2>
          </div>
          <p className="text-xs text-surface-400 mt-1">{skill.id} · v{skill.version}</p>
        </div>
      </div>

      {/* Meta */}
      <div className="flex flex-wrap gap-1.5 mb-4 shrink-0">
        <span className="inline-flex items-center px-2 py-0.5 text-xs rounded-full bg-surface-700 text-surface-300 border border-surface-600">
          {skill.category}
        </span>
        {skill.tags.map((tag) => (
          <span key={tag} className="inline-flex items-center px-2 py-0.5 text-xs rounded-full bg-primary/10 text-primary border border-primary/20">
            {tag}
          </span>
        ))}
      </div>

      {/* Description */}
      <p className="text-sm text-surface-300 mb-4 shrink-0">{skill.description}</p>

      {/* Capabilities */}
      <div className="flex flex-wrap gap-3 mb-4 shrink-0 text-xs text-surface-400">
        <span>{skill.requiresUserConfirmation ? "需要确认" : "自动执行"}</span>
        <span>{skill.writesToProject ? "写入项目" : "不写入"}</span>
      </div>

      {/* Content preview */}
      <div className="flex-1 min-h-0 mb-4">
        <p className="text-xs font-medium text-surface-400 mb-2">内容预览</p>
        <pre className="h-[calc(100%-1.5rem)] overflow-y-auto text-xs text-surface-300 bg-surface-800/60 rounded-lg p-3 whitespace-pre-wrap font-mono leading-relaxed">
          {content || "(空)"}
        </pre>
      </div>

      {/* Actions */}
      <div className="flex gap-2 shrink-0 pt-3 border-t border-surface-700">
        <Button variant="primary" size="sm" onClick={() => setEditing(true)}>编辑</Button>
        {isBuiltin ? (
          <Button variant="secondary" size="sm" onClick={handleReset} loading={resetting} disabled={resetting}>
            重置出厂
          </Button>
        ) : (
          <Button variant="danger" size="sm" onClick={handleDelete} loading={deleting} disabled={deleting}>
            删除
          </Button>
        )}
      </div>
    </div>
  );
}
