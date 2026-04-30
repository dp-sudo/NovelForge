import { useEffect, useMemo, useState } from "react";
import { useSkillStore } from "../../stores/skillStore.js";
import { SkillEditor } from "./SkillEditor.js";
import { Button } from "../ui/Button.js";
import { Select } from "../forms/Select.js";
import { Textarea } from "../forms/Textarea.js";
import {
  getSkillContent,
  updateSkill,
  deleteSkill,
  resetBuiltinSkill,
  type SkillManifest,
  type SkillManifestPatch,
} from "../../api/skillsApi.js";

interface SkillDetailProps {
  skill: SkillManifest;
  onDeleted: () => void;
  onUpdated: (s: SkillManifest) => void;
}

interface ManifestDraft {
  skillClass: "" | "workflow" | "capability" | "extractor" | "review" | "policy";
  bundleIdsText: string;
  alwaysOn: boolean;
  triggerConditionsText: string;
  requiredContextsText: string;
  stateWritesText: string;
  automationTier: "" | "auto" | "supervised" | "confirm";
  sceneTags: string[];
  affectsLayersText: string;
}

const SKILL_CLASS_OPTIONS = [
  { value: "", label: "未分类" },
  { value: "workflow", label: "workflow" },
  { value: "capability", label: "capability" },
  { value: "extractor", label: "extractor" },
  { value: "review", label: "review" },
  { value: "policy", label: "policy" },
];

const AUTOMATION_TIER_OPTIONS = [
  { value: "", label: "未指定" },
  { value: "auto", label: "auto" },
  { value: "supervised", label: "supervised" },
  { value: "confirm", label: "confirm" },
];

const SCENE_TAG_OPTIONS = [
  "dialogue",
  "action",
  "battle",
  "emotion",
  "introspection",
  "suspense",
  "romance",
  "worldbuilding",
];

function listToLines(items: string[]): string {
  return normalizeList(items).join("\n");
}

function linesToList(raw: string): string[] {
  return raw
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function normalizeList(items: string[]): string[] {
  return items.map((item) => item.trim()).filter(Boolean);
}

function buildManifestDraft(skill: SkillManifest): ManifestDraft {
  return {
    skillClass: skill.skillClass ?? "",
    bundleIdsText: listToLines(skill.bundleIds),
    alwaysOn: skill.alwaysOn,
    triggerConditionsText: listToLines(skill.triggerConditions),
    requiredContextsText: listToLines(skill.requiredContexts),
    stateWritesText: listToLines(skill.stateWrites),
    automationTier: skill.automationTier ?? "",
    sceneTags: normalizeList(skill.sceneTags),
    affectsLayersText: listToLines(skill.affectsLayers),
  };
}

function draftToManifestPatch(draft: ManifestDraft): SkillManifestPatch {
  return {
    skillClass: draft.skillClass,
    bundleIds: linesToList(draft.bundleIdsText),
    alwaysOn: draft.alwaysOn,
    triggerConditions: linesToList(draft.triggerConditionsText),
    requiredContexts: linesToList(draft.requiredContextsText),
    stateWrites: linesToList(draft.stateWritesText),
    automationTier: draft.automationTier,
    sceneTags: normalizeList(draft.sceneTags),
    affectsLayers: linesToList(draft.affectsLayersText),
  };
}

function toComparableDraft(skill: SkillManifest): string {
  return JSON.stringify(draftToManifestPatch(buildManifestDraft(skill)));
}

function toComparablePatch(draft: ManifestDraft): string {
  return JSON.stringify(draftToManifestPatch(draft));
}

export function SkillDetail({ skill, onDeleted, onUpdated }: SkillDetailProps) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [savingManifest, setSavingManifest] = useState(false);
  const [manifestDraft, setManifestDraft] = useState<ManifestDraft>(() => buildManifestDraft(skill));
  const setStoreError = useSkillStore((s) => s.setError);

  const manifestDirty = useMemo(
    () => toComparablePatch(manifestDraft) !== toComparableDraft(skill),
    [manifestDraft, skill],
  );

  useEffect(() => {
    setEditing(false);
    setContent(null);
    setLoading(true);
    setError(null);
    setManifestDraft(buildManifestDraft(skill));
    getSkillContent(skill.id)
      .then((c) => {
        setContent(c);
        setLoading(false);
      })
      .catch((e) => {
        const msg =
          typeof e === "object" && e && "message" in e
            ? String((e as { message: string }).message)
            : "加载失败";
        setError(msg);
        setLoading(false);
      });
  }, [skill.id]);

  async function handleSave(body: string) {
    const updated = await updateSkill({ id: skill.id, body });
    setContent(body);
    setEditing(false);
    onUpdated(updated);
  }

  async function handleSaveManifest() {
    setSavingManifest(true);
    try {
      const updated = await updateSkill({
        id: skill.id,
        manifest: draftToManifestPatch(manifestDraft),
      });
      setManifestDraft(buildManifestDraft(updated));
      onUpdated(updated);
    } catch (e: unknown) {
      const msg =
        typeof e === "object" && e && "message" in e
          ? String((e as { message: string }).message)
          : "保存元数据失败";
      setStoreError(msg);
    } finally {
      setSavingManifest(false);
    }
  }

  async function handleDelete() {
    if (!confirm(`确定删除技能「${skill.name}」？此操作不可撤销。`)) return;
    setDeleting(true);
    try {
      await deleteSkill(skill.id);
      onDeleted();
    } catch (e: unknown) {
      const msg =
        typeof e === "object" && e && "message" in e
          ? String((e as { message: string }).message)
          : "删除失败";
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
      setManifestDraft(buildManifestDraft(restored));
      setEditing(false);
      onUpdated(restored);
    } catch (e: unknown) {
      const msg =
        typeof e === "object" && e && "message" in e
          ? String((e as { message: string }).message)
          : "重置失败";
      setStoreError(msg);
    } finally {
      setResetting(false);
    }
  }

  function toggleSceneTag(tag: string) {
    setManifestDraft((prev) => {
      const exists = prev.sceneTags.includes(tag);
      return {
        ...prev,
        sceneTags: exists
          ? prev.sceneTags.filter((item) => item !== tag)
          : [...prev.sceneTags, tag],
      };
    });
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
      <div className="flex items-start justify-between mb-4 shrink-0">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            {skill.icon && <span className="text-xl">{skill.icon}</span>}
            <h2 className="text-base font-semibold text-surface-100 truncate">{skill.name}</h2>
          </div>
          <p className="text-xs text-surface-400 mt-1">{skill.id} · v{skill.version}</p>
        </div>
      </div>

      <div className="flex flex-wrap gap-1.5 mb-4 shrink-0">
        <span className="inline-flex items-center px-2 py-0.5 text-xs rounded-full bg-surface-700 text-surface-300 border border-surface-600">
          {skill.category}
        </span>
        {skill.tags.map((tag) => (
          <span
            key={tag}
            className="inline-flex items-center px-2 py-0.5 text-xs rounded-full bg-primary/10 text-primary border border-primary/20"
          >
            {tag}
          </span>
        ))}
      </div>

      <p className="text-sm text-surface-300 mb-4 shrink-0">{skill.description}</p>

      <div className="space-y-4 mb-4 shrink-0 rounded-lg border border-surface-700 bg-surface-900/40 p-3">
        <h3 className="text-sm font-semibold text-surface-200">运行期元数据</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <Select
            label="技能分类"
            value={manifestDraft.skillClass}
            onChange={(e) =>
              setManifestDraft((prev) => ({
                ...prev,
                skillClass: e.target.value as ManifestDraft["skillClass"],
              }))
            }
            options={SKILL_CLASS_OPTIONS}
          />
          <Select
            label="自动化档位要求"
            value={manifestDraft.automationTier}
            onChange={(e) =>
              setManifestDraft((prev) => ({
                ...prev,
                automationTier: e.target.value as ManifestDraft["automationTier"],
              }))
            }
            options={AUTOMATION_TIER_OPTIONS}
          />
        </div>

        <div className="space-y-2">
          <p className="text-xs text-surface-400">常驻激活：策略技能可设置为默认持续注入。</p>
          <Button
            type="button"
            variant={manifestDraft.alwaysOn ? "primary" : "secondary"}
            size="sm"
            onClick={() =>
              setManifestDraft((prev) => ({
                ...prev,
                alwaysOn: !prev.alwaysOn,
              }))
            }
          >
            {manifestDraft.alwaysOn ? "Always On 已启用" : "Always On 未启用"}
          </Button>
        </div>

        <Textarea
          label="能力包绑定（bundleIds，每行一个）"
          value={manifestDraft.bundleIdsText}
          onChange={(e) =>
            setManifestDraft((prev) => ({
              ...prev,
              bundleIdsText: e.target.value,
            }))
          }
          rows={3}
          placeholder="chapter-core&#10;character-presence"
        />
        <Textarea
          label="触发条件（triggerConditions，每行一个）"
          value={manifestDraft.triggerConditionsText}
          onChange={(e) =>
            setManifestDraft((prev) => ({
              ...prev,
              triggerConditionsText: e.target.value,
            }))
          }
          rows={3}
          placeholder="chapter.plan&#10;scene.rewrite"
        />
        <Textarea
          label="所需上下文（requiredContexts，每行一个）"
          value={manifestDraft.requiredContextsText}
          onChange={(e) =>
            setManifestDraft((prev) => ({
              ...prev,
              requiredContextsText: e.target.value,
            }))
          }
          rows={2}
          placeholder="canon&#10;state"
        />
        <Textarea
          label="状态写入声明（stateWrites，每行一个）"
          value={manifestDraft.stateWritesText}
          onChange={(e) =>
            setManifestDraft((prev) => ({
              ...prev,
              stateWritesText: e.target.value,
            }))
          }
          rows={2}
          placeholder="character.emotion&#10;plot.progress"
        />
        <Textarea
          label="影响层（affectsLayers，每行一个）"
          value={manifestDraft.affectsLayersText}
          onChange={(e) =>
            setManifestDraft((prev) => ({
              ...prev,
              affectsLayersText: e.target.value,
            }))
          }
          rows={2}
          placeholder="constitution&#10;canon&#10;state"
        />

        <div className="space-y-2">
          <p className="text-sm font-medium text-surface-200">场景标签（sceneTags）</p>
          <p className="text-xs text-surface-400">支持多选，用于标记技能适用场景。</p>
          <div className="flex flex-wrap gap-2">
            {SCENE_TAG_OPTIONS.map((tag) => {
              const active = manifestDraft.sceneTags.includes(tag);
              return (
                <button
                  key={tag}
                  type="button"
                  onClick={() => toggleSceneTag(tag)}
                  className={`px-2 py-1 text-xs rounded-md border transition-colors ${
                    active
                      ? "bg-primary/20 text-primary border-primary/30"
                      : "bg-surface-800 text-surface-400 border-surface-600 hover:text-surface-200"
                  }`}
                >
                  {tag}
                </button>
              );
            })}
          </div>
        </div>

        <div className="flex items-center gap-2 pt-2 border-t border-surface-700">
          <Button
            size="sm"
            variant="primary"
            onClick={() => void handleSaveManifest()}
            loading={savingManifest}
            disabled={!manifestDirty || savingManifest}
          >
            保存元数据
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setManifestDraft(buildManifestDraft(skill))}
            disabled={!manifestDirty || savingManifest}
          >
            重置改动
          </Button>
          {manifestDirty && <span className="text-xs text-warning">未保存改动</span>}
        </div>
      </div>

      <div className="flex flex-wrap gap-3 mb-4 shrink-0 text-xs text-surface-400">
        <span>{skill.requiresUserConfirmation ? "需要确认" : "自动执行"}</span>
        <span>{skill.writesToProject ? "写入项目" : "不写入"}</span>
        <span>{skill.alwaysOn ? "Always On" : "按需激活"}</span>
      </div>

      <div className="flex-1 min-h-0 mb-4">
        <p className="text-xs font-medium text-surface-400 mb-2">内容预览</p>
        <pre className="h-[calc(100%-1.5rem)] overflow-y-auto text-xs text-surface-300 bg-surface-800/60 rounded-lg p-3 whitespace-pre-wrap font-mono leading-relaxed">
          {content || "(空)"}
        </pre>
      </div>

      <div className="flex gap-2 shrink-0 pt-3 border-t border-surface-700">
        <Button variant="primary" size="sm" onClick={() => setEditing(true)}>
          编辑正文
        </Button>
        {isBuiltin ? (
          <Button
            variant="secondary"
            size="sm"
            onClick={handleReset}
            loading={resetting}
            disabled={resetting}
          >
            重置出厂
          </Button>
        ) : (
          <Button
            variant="danger"
            size="sm"
            onClick={handleDelete}
            loading={deleting}
            disabled={deleting}
          >
            删除
          </Button>
        )}
      </div>
    </div>
  );
}
