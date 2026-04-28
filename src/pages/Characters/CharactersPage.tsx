import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Select } from "../../components/forms/Select.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { ConfirmDialog } from "../../components/dialogs/ConfirmDialog.js";
import { listCharacters, createCharacter, deleteCharacter, aiGenerateCharacter, listCharacterRelationships, createCharacterRelationship, deleteCharacterRelationship, type CharacterRelationship } from "../../api/characterApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

const ROLE_TYPES = [
  { value: "主角", label: "主角" },
  { value: "反派", label: "反派" },
  { value: "配角", label: "配角" },
  { value: "路人", label: "路人" },
  { value: "组织角色", label: "组织角色" }
] as const;

const RELATIONSHIP_TYPES = [
  { value: "盟友", label: "盟友" },
  { value: "敌对", label: "敌对" },
  { value: "师徒", label: "师徒" },
  { value: "情侣", label: "情侣" },
  { value: "家人", label: "家人" },
  { value: "上下级", label: "上下级" },
  { value: "旧识", label: "旧识" },
  { value: "其他", label: "其他" }
] as const;

interface CharacterRow {
  id: string; name: string; role_type: string; age: string | null;
  gender: string | null; identity_text: string | null; appearance: string | null;
  motivation: string | null; desire: string | null; fear: string | null;
  flaw: string | null; arc_stage: string | null; notes: string | null;
}

const emptyForm = {
  name: "", roleType: "配角" as const, age: "", gender: "",
  identityText: "", appearance: "", motivation: "", desire: "",
  fear: "", flaw: "", arcStage: "", notes: ""
};

export function CharactersPage() {
  const [characters, setCharacters] = useState<CharacterRow[]>([]);
  const [selected, setSelected] = useState<CharacterRow | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [showDelete, setShowDelete] = useState(false);
  const [form, setForm] = useState(emptyForm);
  const [relationships, setRelationships] = useState<CharacterRelationship[]>([]);
  const [showAiCreate, setShowAiCreate] = useState(false);
  const [aiDescription, setAiDescription] = useState("");
  const [aiLoading, setAiLoading] = useState(false);
  const [aiResult, setAiResult] = useState<string | null>(null);
  const [showNewRel, setShowNewRel] = useState(false);
  const [relForm, setRelForm] = useState({ targetId: "", relType: "盟友", description: "" });
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setCharacters([]);
      return;
    }
    const data = await listCharacters(projectRoot);
    setCharacters(data);
  }, [projectRoot]);
  const loadRelationships = useCallback(async (charId: string) => {
    if (!projectRoot) return;
    const data = await listCharacterRelationships(projectRoot, charId);
    setRelationships(data);
  }, [projectRoot]);

  useEffect(() => { void load(); }, [load]);
  useEffect(() => {
    if (selected && projectRoot) {
      void loadRelationships(selected.id);
    } else {
      setRelationships([]);
    }
  }, [selected, projectRoot, loadRelationships]);

  function resetForm() { setForm(emptyForm); }

  async function handleCreate() {
    if (!form.name.trim() || !projectRoot) return;
    await createCharacter({
      name: form.name.trim(), roleType: form.roleType,
      age: form.age || undefined, gender: form.gender || undefined,
      identityText: form.identityText || undefined, appearance: form.appearance || undefined,
      motivation: form.motivation || undefined, desire: form.desire || undefined,
      fear: form.fear || undefined, flaw: form.flaw || undefined,
      arcStage: form.arcStage || undefined, notes: form.notes || undefined
    }, projectRoot);
    resetForm(); setShowNew(false); await load();
  }

  async function handleDelete() {
    if (!selected || !projectRoot) return;
    await deleteCharacter(selected.id, projectRoot);
    setShowDelete(false); setSelected(null); await load();
  }

  async function handleAiCreate() {
    if (!projectRoot || !aiDescription.trim()) return;
    setAiLoading(true); setAiResult(null);
    try {
      const result = await aiGenerateCharacter(projectRoot, aiDescription);
      setAiResult(result || "生成失败");
    } catch { setAiResult("AI 生成失败"); }
    finally { setAiLoading(false); }
  }

  async function handleCreateFromAi() {
    if (!aiResult) return;
    try {
      const json = JSON.parse(aiResult);
      await createCharacter({
        name: json.name || "未命名角色", roleType: json.roleType || "配角",
        identityText: json.identityText, appearance: json.appearance,
        motivation: json.motivation, desire: json.desire, fear: json.fear,
        flaw: json.flaw, arcStage: json.arcStage, notes: json.notes,
      }, projectRoot);
      setShowAiCreate(false); setAiResult(null); setAiDescription("");
      await load();
    } catch { /* JSON parse error */ }
  }

  async function handleAddRelationship() {
    if (!projectRoot || !selected || !relForm.targetId) return;
    await createCharacterRelationship(projectRoot, {
      sourceCharacterId: selected.id,
      targetCharacterId: relForm.targetId,
      relationshipType: relForm.relType,
      description: relForm.description || undefined,
    });
    setShowNewRel(false); setRelForm({ targetId: "", relType: "盟友", description: "" });
    await loadRelationships(selected.id);
  }

  async function handleDeleteRelationship(id: string) {
    if (!projectRoot) return;
    await deleteCharacterRelationship(projectRoot, id);
    if (selected) await loadRelationships(selected.id);
  }

  function getCharName(id: string): string {
    return characters.find(c => c.id === id)?.name ?? "未知";
  }

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-surface-100">角色工坊</h1>
        <div className="flex gap-2">
          <Button variant="ghost" size="sm" onClick={() => { setAiDescription(""); setAiResult(null); setShowAiCreate(true); }}>
            AI 创建
          </Button>
          <Button variant="primary" size="sm" onClick={() => { resetForm(); setShowNew(true); }}>新建角色</Button>
        </div>
      </div>

      <div className="flex gap-6">
        <div className="w-72 shrink-0">
          {characters.length === 0 ? (
            <Card padding="lg" className="text-center">
              <p className="text-sm text-surface-400 mb-3">还没有角色</p>
              <Button variant="primary" size="sm" onClick={() => { resetForm(); setShowNew(true); }}>新建角色</Button>
            </Card>
          ) : (
            <div className="space-y-2">
              {characters.map((c) => (
                <button
                  key={c.id}
                  onClick={() => setSelected(c)}
                  className={`w-full text-left p-3 rounded-lg transition-colors border ${
                    selected?.id === c.id
                      ? "bg-primary/10 border-primary/30"
                      : "bg-surface-800 border-surface-700 hover:border-surface-500"
                  }`}
                >
                  <div className="text-sm font-medium text-surface-100">{c.name}</div>
                  <div className="text-xs text-surface-400 mt-1">
                    {c.role_type}
                    {c.motivation && ` · ${c.motivation.slice(0, 20)}`}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="flex-1 min-w-0 space-y-4">
          {!selected ? (
            <Card padding="lg" className="text-center">
              <p className="text-surface-400 text-sm">选择一个角色查看详情</p>
            </Card>
          ) : (
            <>
              <Card padding="lg" className="space-y-4">
                <div className="flex items-center justify-between">
                  <h2 className="text-lg font-semibold text-surface-100">{selected.name}</h2>
                  <Button variant="danger" size="sm" onClick={() => setShowDelete(true)}>删除</Button>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <Input label="姓名" value={selected.name} readOnly />
                  <Select label="类型" value={selected.role_type} options={[...ROLE_TYPES]} disabled />
                  <Input label="年龄" value={selected.age ?? ""} readOnly />
                  <Input label="性别" value={selected.gender ?? ""} readOnly />
                </div>
                <Input label="身份" value={selected.identity_text ?? ""} readOnly />
                <Textarea label="外貌" value={selected.appearance ?? ""} readOnly />
                <Textarea label="核心动机" value={selected.motivation ?? ""} readOnly className="min-h-[60px]" />
                <div className="grid grid-cols-3 gap-4">
                  <Textarea label="欲望" value={selected.desire ?? ""} readOnly />
                  <Textarea label="恐惧" value={selected.fear ?? ""} readOnly />
                  <Textarea label="缺陷" value={selected.flaw ?? ""} readOnly />
                </div>
                <Textarea label="成长弧线" value={selected.arc_stage ?? ""} readOnly />
                <Textarea label="备注" value={selected.notes ?? ""} readOnly />
              </Card>

              <Card padding="lg" className="space-y-3">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-semibold text-surface-200">角色关系</h3>
                  <Button variant="secondary" size="sm" onClick={() => setShowNewRel(true)}>添加关系</Button>
                </div>
                {relationships.length === 0 ? (
                  <p className="text-xs text-surface-400">暂无关联关系</p>
                ) : (
                  <div className="space-y-2">
                    {relationships.map((r) => (
                      <div key={r.id} className="flex items-center justify-between p-2 bg-surface-800 rounded-lg">
                        <div className="text-sm text-surface-200">
                          {getCharName(r.sourceCharacterId) === selected.name
                            ? `${selected.name} → ${getCharName(r.targetCharacterId)}`
                            : `${getCharName(r.sourceCharacterId)} → ${selected.name}`}
                          <span className="text-xs text-surface-400 ml-2">[{r.relationshipType}]</span>
                          {r.description && <span className="text-xs text-surface-500 ml-2">{r.description}</span>}
                        </div>
                        <button onClick={() => void handleDeleteRelationship(r.id)} className="text-xs text-error hover:text-error-light">删除</button>
                      </div>
                    ))}
                  </div>
                )}
              </Card>
            </>
          )}
        </div>
      </div>

      <Modal open={showNew} onClose={() => setShowNew(false)} title="新建角色" width="lg">
        <div className="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
          <Input label="姓名 *" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} />
          <Select label="角色类型" value={form.roleType} onChange={(e) => setForm({ ...form, roleType: e.target.value as typeof form.roleType })} options={[...ROLE_TYPES]} />
          <div className="grid grid-cols-2 gap-4">
            <Input label="年龄" value={form.age} onChange={(e) => setForm({ ...form, age: e.target.value })} />
            <Input label="性别" value={form.gender} onChange={(e) => setForm({ ...form, gender: e.target.value })} />
          </div>
          <Input label="身份" value={form.identityText} onChange={(e) => setForm({ ...form, identityText: e.target.value })} />
          <Textarea label="外貌" value={form.appearance} onChange={(e) => setForm({ ...form, appearance: e.target.value })} />
          <Textarea label="核心动机" value={form.motivation} onChange={(e) => setForm({ ...form, motivation: e.target.value })} />
          <div className="grid grid-cols-3 gap-4">
            <Textarea label="欲望" value={form.desire} onChange={(e) => setForm({ ...form, desire: e.target.value })} />
            <Textarea label="恐惧" value={form.fear} onChange={(e) => setForm({ ...form, fear: e.target.value })} />
            <Textarea label="缺陷" value={form.flaw} onChange={(e) => setForm({ ...form, flaw: e.target.value })} />
          </div>
          <Textarea label="成长弧线" value={form.arcStage} onChange={(e) => setForm({ ...form, arcStage: e.target.value })} />
          <Textarea label="备注" value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} />
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleCreate()} disabled={!form.name.trim()}>创建</Button>
          </div>
        </div>
      </Modal>

      <Modal open={showAiCreate} onClose={() => setShowAiCreate(false)} title="AI 创建角色" width="lg">
        <div className="space-y-4">
          <Textarea label="描述角色设想" value={aiDescription} onChange={(e) => setAiDescription(e.target.value)}
            placeholder="例如：一位冷峻的剑客，表面冷酷内心温柔，背负着血海深仇..." className="min-h-[120px]" />
          <Button variant="primary" loading={aiLoading} onClick={() => void handleAiCreate()} disabled={!aiDescription.trim()}>
            {aiLoading ? "生成中..." : "生成角色卡"}
          </Button>
          {aiResult && (
            <div className="p-4 bg-surface-800 rounded-xl">
              <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans">{aiResult}</pre>
              <div className="flex gap-2 mt-3">
                <Button variant="primary" size="sm" onClick={() => void handleCreateFromAi()}>保存角色</Button>
                <Button variant="ghost" size="sm" onClick={() => setAiResult(null)}>重新生成</Button>
              </div>
            </div>
          )}
        </div>
      </Modal>

      <Modal open={showNewRel} onClose={() => setShowNewRel(false)} title="添加角色关系" width="md">
        <div className="space-y-4">
          <Select label="目标角色" value={relForm.targetId}
            onChange={(e) => setRelForm({ ...relForm, targetId: e.target.value })}
            options={characters.filter(c => c.id !== selected?.id).map(c => ({ value: c.id, label: `${c.name}（${c.role_type}）` }))} />
          <Select label="关系类型" value={relForm.relType}
            onChange={(e) => setRelForm({ ...relForm, relType: e.target.value })}
            options={[...RELATIONSHIP_TYPES]} />
          <Input label="关系描述（可选）" value={relForm.description} onChange={(e) => setRelForm({ ...relForm, description: e.target.value })} />
          <div className="flex justify-end gap-3 pt-3 border-t border-surface-700">
            <Button variant="ghost" onClick={() => setShowNewRel(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleAddRelationship()} disabled={!relForm.targetId}>添加</Button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog open={showDelete} title="删除角色"
        message={`确定删除「${selected?.name}」吗？如果角色已被章节引用，需要先解除关联。`}
        variant="danger" confirmLabel="删除"
        onConfirm={() => void handleDelete()}
        onCancel={() => setShowDelete(false)} />
    </div>
  );
}
