import { useState, useEffect } from "react";
import { Button } from "../ui/Button.js";

interface SkillEditorProps {
  initialContent: string;
  readOnlyFields?: string[];
  onSave: (content: string) => Promise<void>;
  onCancel: () => void;
}

export function SkillEditor({ initialContent, readOnlyFields = ["id", "source", "version", "createdAt"], onSave, onCancel }: SkillEditorProps) {
  const [content, setContent] = useState(initialContent);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    setContent(initialContent);
    setDirty(false);
  }, [initialContent]);

  function handleChange(value: string) {
    setContent(value);
    setDirty(value !== initialContent);
  }

  async function handleSave() {
    setSaving(true);
    try {
      await onSave(content);
      setDirty(false);
    } finally {
      setSaving(false);
    }
  }

  const frontmatterHint = readOnlyFields.length > 0
    ? `以下字段自动保护，不可修改：${readOnlyFields.join(", ")}`
    : "";

  return (
    <div className="flex flex-col h-full min-h-0 rounded-lg border border-surface-700 bg-surface-900/40 p-4">
      <div className="flex items-center justify-between mb-3 shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-xs text-surface-400">Markdown 编辑器</span>
          {dirty && <span className="text-xs text-warning">● 未保存</span>}
        </div>
        <div className="flex gap-2">
          <Button variant="ghost" size="sm" onClick={onCancel}>取消</Button>
          <Button variant="primary" size="sm" onClick={handleSave} disabled={!dirty || saving} loading={saving}>
            保存
          </Button>
        </div>
      </div>
      {frontmatterHint && (
        <p className="text-xs text-surface-500 mb-2 shrink-0">{frontmatterHint}</p>
      )}
      <div className="flex-1 min-h-0">
        <textarea
          value={content}
          onChange={(e) => handleChange(e.target.value)}
          className="h-full min-h-full w-full rounded-lg border border-input bg-surface-800 px-3 py-2 font-mono text-sm leading-relaxed text-surface-100 placeholder:text-surface-400 focus:outline-none focus:ring-2 focus:ring-ring/50 focus:border-primary resize-none"
          spellCheck={false}
        />
      </div>
    </div>
  );
}
