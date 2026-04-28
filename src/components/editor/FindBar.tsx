import { useState, useEffect, useRef, useMemo } from "react";

interface FindBarProps {
  open: boolean;
  content: string;
  onClose: () => void;
  /** Called when user wants to jump to a match — provides the index to select. */
  onSelectMatch: (start: number, end: number) => void;
}

export function FindBar({ open, content, onClose, onSelectMatch }: FindBarProps) {
  const [query, setQuery] = useState("");
  const [currentIdx, setCurrentIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Find all match positions (case-insensitive)
  const matches = useMemo(() => {
    if (!query.trim()) return [];
    const q = query.toLowerCase();
    const positions: number[] = [];
    let i = 0;
    while (i < content.length) {
      const idx = content.toLowerCase().indexOf(q, i);
      if (idx === -1) break;
      positions.push(idx);
      i = idx + 1;
    }
    return positions;
  }, [query, content]);

  // Reset index when query changes
  useEffect(() => { setCurrentIdx(0); }, [query]);

  // Focus input when bar opens
  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  // Navigate to current match
  useEffect(() => {
    if (matches.length > 0 && matches[currentIdx] !== undefined) {
      onSelectMatch(matches[currentIdx], matches[currentIdx] + query.length);
    }
  }, [currentIdx, matches, query.length, onSelectMatch]);

  useEffect(() => {
    if (!open) return;
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
      if (e.key === "Enter") {
        e.preventDefault();
        if (e.shiftKey) {
          setCurrentIdx((i) => (i > 0 ? i - 1 : matches.length - 1));
        } else {
          setCurrentIdx((i) => (i < matches.length - 1 ? i + 1 : 0));
        }
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [open, matches.length, onClose]);

  if (!open) return null;

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 bg-surface-800 border-b border-border rounded-t-lg">
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="查找..."
        className="flex-1 max-w-[200px] px-2 py-1 text-xs bg-surface-700 border border-border rounded text-surface-100 placeholder:text-surface-400 outline-none focus:border-primary/50"
      />
      {query.trim() && (
        <span className="text-xs text-surface-400 min-w-[60px]">
          {matches.length > 0
            ? `${currentIdx + 1}/${matches.length}`
            : "无匹配"}
        </span>
      )}
      {query.trim() && matches.length > 0 && (
        <>
          <button
            onClick={() => setCurrentIdx((i) => (i > 0 ? i - 1 : matches.length - 1))}
            className="px-1.5 py-0.5 text-xs text-surface-300 hover:text-surface-100 rounded hover:bg-surface-700 transition-colors"
            title="上一个 (Shift+Enter)"
          >
            ▲
          </button>
          <button
            onClick={() => setCurrentIdx((i) => (i < matches.length - 1 ? i + 1 : 0))}
            className="px-1.5 py-0.5 text-xs text-surface-300 hover:text-surface-100 rounded hover:bg-surface-700 transition-colors"
            title="下一个 (Enter)"
          >
            ▼
          </button>
        </>
      )}
      <button
        onClick={onClose}
        className="ml-auto text-xs text-surface-400 hover:text-surface-200 transition-colors"
      >
        ✕
      </button>
    </div>
  );
}
