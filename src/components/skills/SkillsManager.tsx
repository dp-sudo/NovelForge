import { useEffect, useState, useCallback, useRef } from "react";
import { useSkillStore } from "../../stores/skillStore.js";
import { SkillList } from "./SkillList.js";
import { SkillDetail } from "./SkillDetail.js";
import { Button } from "../ui/Button.js";
import { Input } from "../forms/Input.js";
import { listSkills, importSkillFile, refreshSkills, type SkillManifest } from "../../api/skillsApi.js";

const CATEGORIES = [
  { key: "", label: "全部" },
  { key: "workflow", label: "Workflow" },
  { key: "capability", label: "Capability" },
  { key: "extractor", label: "Extractor" },
  { key: "review", label: "Review" },
  { key: "policy", label: "Policy" },
  { key: "__unclassified__", label: "未分类" },
];

export function SkillsManager() {
  const skills = useSkillStore((s) => s.skills);
  const selectedId = useSkillStore((s) => s.selectedId);
  const error = useSkillStore((s) => s.error);
  const loading = useSkillStore((s) => s.loading);
  const setSkills = useSkillStore((s) => s.setSkills);
  const setSelectedId = useSkillStore((s) => s.setSelectedId);
  const setLoading = useSkillStore((s) => s.setLoading);
  const setError = useSkillStore((s) => s.setError);

  const [filter, setFilter] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("");
  const fileInputRef = useRef<HTMLInputElement>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await listSkills();
      setSkills(list);
      if (selectedId && !list.find((s) => s.id === selectedId)) {
        setSelectedId(null);
      }
    } catch (e: unknown) {
      const msg = typeof e === "object" && e && "message" in e
        ? String((e as { message: string }).message)
        : "加载技能列表失败";
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  async function handleRefresh() {
    try {
      const list = await refreshSkills();
      setSkills(list);
    } catch (e: unknown) {
      const msg = typeof e === "object" && e && "message" in e
        ? String((e as { message: string }).message)
        : "刷新失败";
      setError(msg);
    }
  }

  function handleImportClick() {
    fileInputRef.current?.click();
  }

  async function handleFileSelected(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const filePath = (file as File & { path?: string }).path;
      if (!filePath) {
        setError("当前环境无法读取文件路径，请在桌面端导入技能文件。");
        e.target.value = "";
        return;
      }
      const imported = await importSkillFile(filePath);
      setError(null);
      await load();
      setSelectedId(imported.id);
    } catch (err: unknown) {
      const msg = typeof err === "object" && err && "message" in err
        ? String((err as { message: string }).message)
        : "导入失败";
      setError(msg);
    }
    e.target.value = "";
  }

  function handleDeleted() {
    setSelectedId(null);
    load();
  }

  function handleUpdated(updated: SkillManifest) {
    setSkills(skills.map((s) => (s.id === updated.id ? updated : s)));
  }

  const selected = skills.find((s) => s.id === selectedId);
  const filteredSkills = categoryFilter
    ? categoryFilter === "__unclassified__"
      ? skills.filter((s) => !s.skillClass)
      : skills.filter((s) => s.skillClass === categoryFilter)
    : skills;

  return (
    <div className="flex h-full min-h-0 gap-4">
      {/* Left panel: list */}
      <div className="w-[300px] lg:w-[340px] shrink-0 flex flex-col gap-3 min-h-0">
        {/* Toolbar */}
        <div className="flex gap-2">
          <Input
            placeholder="搜索技能..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="flex-1"
          />
          <Button variant="secondary" size="sm" onClick={handleRefresh} title="刷新">
            ↻
          </Button>
          <Button variant="secondary" size="sm" onClick={handleImportClick} title="导入 .md 文件">
            +
          </Button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".md"
            className="hidden"
            onChange={handleFileSelected}
          />
        </div>

        {/* Category filter */}
        <div className="flex gap-1 flex-wrap">
          {CATEGORIES.map((cat) => (
            <button
              key={cat.key}
              onClick={() => setCategoryFilter(cat.key)}
              className={`px-2 py-1 text-xs rounded-lg transition-colors ${
                categoryFilter === cat.key
                  ? "bg-primary/20 text-primary border border-primary/30"
                  : "bg-surface-700 text-surface-400 border border-surface-600 hover:text-surface-200"
              }`}
            >
              {cat.label}
            </button>
          ))}
        </div>

        {/* Error banner */}
        {error && (
          <div className="px-3 py-2 rounded-lg text-xs bg-error/10 text-error border border-error/20">
            {error}
            <button className="ml-2 underline" onClick={() => setError(null)}>关闭</button>
          </div>
        )}

        {/* Skill list */}
        <div className="flex-1 overflow-y-auto min-h-0">
          {loading ? (
            <div className="flex items-center justify-center h-32 text-sm text-surface-500">加载中...</div>
          ) : (
            <SkillList skills={filteredSkills} filter={filter} />
          )}
        </div>

        <div className="text-xs text-surface-500 shrink-0">
          共 {filteredSkills.length} 个技能
        </div>
      </div>

      {/* Right panel: detail */}
      <div className="flex-1 min-w-0 h-full">
        {selected ? (
          <SkillDetail
            skill={selected}
            onDeleted={handleDeleted}
            onUpdated={handleUpdated}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-sm text-surface-500">
            选择一个技能查看详情
          </div>
        )}
      </div>
    </div>
  );
}
