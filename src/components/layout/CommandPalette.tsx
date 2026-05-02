import { useEffect, useState, useCallback } from "react";

interface PageItem {
  label: string;
  path: string;
  keywords: string;
}

const PAGES: PageItem[] = [
  { label: "项目中心", path: "/", keywords: "project center 项目 首页" },
  { label: "全书指挥台", path: "/command-center", keywords: "command center 指挥台 工作台 production workbench" },
  { label: "导出中心", path: "/export", keywords: "export 导出" },
  { label: "设置", path: "/settings", keywords: "settings 设置 配置 模型" },
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
