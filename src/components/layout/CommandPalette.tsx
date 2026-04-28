import { useEffect, useState, useCallback } from "react";

interface PageItem {
  label: string;
  path: string;
  keywords: string;
}

const PAGES: PageItem[] = [
  { label: "项目中心", path: "/", keywords: "project center 项目 首页" },
  { label: "项目仪表盘", path: "/dashboard", keywords: "dashboard 仪表盘 统计 stats" },
  { label: "创作蓝图", path: "/blueprint", keywords: "blueprint 蓝图 规划" },
  { label: "角色工坊", path: "/characters", keywords: "characters 角色 人物" },
  { label: "世界设定库", path: "/world", keywords: "world 世界 设定 规则" },
  { label: "名词库", path: "/glossary", keywords: "glossary 名词 术语" },
  { label: "剧情骨架", path: "/plot", keywords: "plot 剧情 主线 骨架" },
  { label: "章节管理", path: "/chapters", keywords: "chapters 章节 卷" },
  { label: "章节编辑器", path: "/editor", keywords: "editor 编辑 写作" },
  { label: "一致性检查", path: "/consistency", keywords: "consistency 检查 审稿" },
  { label: "导出中心", path: "/export", keywords: "export 导出" },
  { label: "设置", path: "/settings", keywords: "settings 设置 配置 模型" },
  { label: "叙事义务", path: "/narrative", keywords: "narrative 伏笔 义务 叙事" },
  { label: "时间线", path: "/timeline", keywords: "timeline 时间线" },
  { label: "角色关系", path: "/relationships", keywords: "relationships 关系 图谱" },
];

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  onNavigate: (path: string) => void;
}

export function CommandPalette({ open, onClose, onNavigate }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);

  const filtered = query.trim()
    ? PAGES.filter((p) => {
        const q = query.toLowerCase();
        return p.label.toLowerCase().includes(q) || p.keywords.toLowerCase().includes(q);
      })
    : PAGES;

  useEffect(() => {
    setSelectedIdx(0);
  }, [query]);

  useEffect(() => {
    if (!open) {
      setQuery("");
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIdx((i) => Math.min(i + 1, filtered.length - 1));
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIdx((i) => Math.max(i - 1, 0));
      }
      if (e.key === "Enter" && filtered[selectedIdx]) {
        e.preventDefault();
        onNavigate(filtered[selectedIdx].path);
        onClose();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [open, filtered, selectedIdx, onClose, onNavigate]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[60] flex items-start justify-center pt-[15vh] bg-black/60 animate-fade-in"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="w-full max-w-lg bg-card border border-border rounded-xl shadow-2xl overflow-hidden animate-scale-in">
        <input
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="搜索页面..."
          className="w-full px-4 py-3 text-sm bg-transparent text-foreground placeholder:text-surface-400 border-b border-border outline-none"
        />
        <div className="max-h-64 overflow-y-auto p-2">
          {filtered.length === 0 ? (
            <p className="text-xs text-surface-400 text-center py-4">无匹配页面</p>
          ) : (
            filtered.map((page, idx) => (
              <button
                key={page.path}
                onClick={() => { onNavigate(page.path); onClose(); }}
                className={`w-full text-left px-3 py-2 text-sm rounded-lg transition-colors ${
                  idx === selectedIdx
                    ? "bg-primary/10 text-primary"
                    : "text-surface-200 hover:bg-surface-700"
                }`}
              >
                {page.label}
                <span className="text-xs text-surface-500 ml-2">{page.path}</span>
              </button>
            ))
          )}
        </div>
        <div className="px-4 py-2 border-t border-border flex gap-4 text-xs text-surface-500">
          <span>↑↓ 导航</span>
          <span>Enter 跳转</span>
          <span>Esc 关闭</span>
        </div>
      </div>
    </div>
  );
}
