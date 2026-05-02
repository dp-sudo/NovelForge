import { useEffect, useRef, useState } from "react";
import { useProjectStore } from "../../stores/projectStore.js";
import { useEditorStore } from "../../stores/editorStore.js";
import { useUiStore, type AppRoute } from "../../stores/uiStore.js";
import { rebuildSearchIndex, searchProject, type SearchResult } from "../../api/chapterApi.js";

const saveStatusLabels: Record<string, { text: string; color: string }> = {
  saved: { text: "已保存", color: "text-success" },
  saving: { text: "正在保存…", color: "text-info" },
  unsaved: { text: "有未保存修改", color: "text-warning" },
  autosaving: { text: "自动保存中…", color: "text-info" },
  error: { text: "保存失败", color: "text-error" }
};

const resultTypeLabels: Record<string, string> = {
  chapter: "章节",
  character: "角色",
  world_rule: "设定",
  glossary: "名词",
  plot_node: "剧情"
};

function resolveRouteByResultType(type: string): AppRoute {
  if (type === "chapter") return "command-center";
  if (type === "character") return "characters";
  if (type === "world_rule") return "world";
  if (type === "glossary") return "glossary";
  if (type === "plot_node") return "plot";
  return "chapters";
}

export function TopBar() {
  const projectName = useProjectStore((s) => s.currentProject?.name ?? "未命名项目");
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const saveStatus = useEditorStore((s) => s.saveStatus);
  const wordCount = useEditorStore((s) => s.wordCount);
  const setActiveChapter = useEditorStore((s) => s.setActiveChapter);
  const activeRoute = useUiStore((s) => s.activeRoute);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [searchNotice, setSearchNotice] = useState<string | null>(null);
  const searchBoxRef = useRef<HTMLDivElement>(null);

  const statusInfo = saveStatusLabels[saveStatus] ?? saveStatusLabels.saved;
  const isEditor = activeRoute === "command-center";

  useEffect(() => {
    function handleOutsideClick(event: MouseEvent) {
      if (!searchBoxRef.current) return;
      if (!searchBoxRef.current.contains(event.target as Node)) {
        setDropdownOpen(false);
      }
    }

    document.addEventListener("mousedown", handleOutsideClick);
    return () => document.removeEventListener("mousedown", handleOutsideClick);
  }, []);

  useEffect(() => {
    if (!projectRoot) {
      setResults([]);
      setDropdownOpen(false);
      return;
    }

    const normalized = query.trim();
    if (normalized.length < 2) {
      setResults([]);
      setSearching(false);
      return;
    }

    setSearching(true);
    const timer = window.setTimeout(() => {
      void searchProject(projectRoot, normalized, 12)
        .then((rows) => {
          setResults(rows);
          setDropdownOpen(true);
        })
        .catch(() => {
          setResults([]);
        })
        .finally(() => {
          setSearching(false);
        });
    }, 200);

    return () => {
      window.clearTimeout(timer);
    };
  }, [query, projectRoot]);

  function handleSelectResult(row: SearchResult) {
    if (row.entityType === "chapter") {
      setActiveChapter(row.entityId, row.title);
    }
    setActiveRoute(resolveRouteByResultType(row.entityType));
    setQuery("");
    setResults([]);
    setDropdownOpen(false);
  }

  async function handleRebuildIndex() {
    if (!projectRoot) return;
    setRebuilding(true);
    setSearchNotice(null);
    try {
      const count = await rebuildSearchIndex(projectRoot);
      setSearchNotice(`索引重建完成，共 ${count} 条记录`);
    } catch (err) {
      setSearchNotice(err instanceof Error ? err.message : "索引重建失败");
    } finally {
      setRebuilding(false);
    }
  }

  return (
    <header className="flex items-center justify-between h-12 px-4 border-b border-surface-700 bg-surface-800 shrink-0 select-none">
      <div className="flex items-center gap-3 min-w-0">
        <span className="text-primary font-semibold text-sm tracking-wide whitespace-nowrap">
          NovelForge
        </span>
        <span className="text-surface-500">/</span>
        <span className="text-surface-100 text-sm truncate">{projectName}</span>
      </div>

      <div className="flex items-center gap-3">
        <div ref={searchBoxRef} className="relative">
          <input
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSearchNotice(null);
              if (!dropdownOpen) setDropdownOpen(true);
            }}
            onFocus={() => setDropdownOpen(true)}
            placeholder="全局搜索（章节/角色/设定）"
            className="w-[320px] px-3 py-1.5 text-xs bg-surface-900 border border-surface-700 rounded-lg text-surface-100 placeholder-surface-500 focus:outline-none focus:ring-2 focus:ring-primary/50 focus:border-primary"
          />
          {dropdownOpen && (query.trim().length >= 2 || searchNotice) && (
            <div className="absolute right-0 top-9 w-[420px] max-h-80 overflow-y-auto bg-surface-900 border border-surface-700 rounded-lg shadow-xl z-30">
              <div className="flex items-center justify-between px-3 py-2 border-b border-surface-700">
                <span className="text-xs text-surface-400">
                  {searching ? "搜索中..." : `结果 ${results.length}`}
                </span>
                <button
                  onClick={() => void handleRebuildIndex()}
                  className="text-xs text-primary hover:text-primary-light disabled:opacity-50"
                  disabled={rebuilding}
                >
                  {rebuilding ? "重建中..." : "重建索引"}
                </button>
              </div>
              {searchNotice && (
                <div className="px-3 py-2 text-xs text-info border-b border-surface-700">
                  {searchNotice}
                </div>
              )}
              {!searching && results.length === 0 ? (
                <div className="px-3 py-3 text-xs text-surface-500">无匹配结果</div>
              ) : (
                results.map((row) => (
                  <button
                    key={`${row.entityType}:${row.entityId}`}
                    onClick={() => handleSelectResult(row)}
                    className="w-full text-left px-3 py-2 border-b border-surface-800 last:border-b-0 hover:bg-surface-800 transition-colors"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <span className="text-sm text-surface-100 truncate">{row.title}</span>
                      <span className="text-[10px] text-surface-500">
                        {resultTypeLabels[row.entityType] ?? row.entityType}
                      </span>
                    </div>
                    {row.bodySnippet && (
                      <p className="text-xs text-surface-400 mt-1 truncate">{row.bodySnippet}</p>
                    )}
                  </button>
                ))
              )}
            </div>
          )}
        </div>

        <div className="flex items-center gap-4 text-xs min-w-[140px] justify-end">
          {isEditor && (
            <>
              <span className="text-surface-300">
                字数: <span className="text-surface-100 font-medium">{wordCount}</span>
              </span>
              <span className={statusInfo.color}>{statusInfo.text}</span>
            </>
          )}
        </div>
      </div>
    </header>
  );
}
