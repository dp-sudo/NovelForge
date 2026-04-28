import { useState } from "react";
import { DiffView } from "./DiffView.js";
import type { AiStreamStatus } from "../../stores/editorStore";

interface AiPreviewPanelProps {
  status: AiStreamStatus;
  content: string;
  errorMessage?: string | null;
  originalText?: string;
  taskType: string;
  onInsert: (strategy: "cursor" | "replace" | "append") => void;
  onDiscard: () => void;
  onCopy: () => void;
}

const STATUS_LABELS: Record<AiStreamStatus, string> = {
  idle: "",
  streaming: "生成中...",
  completed: "已完成",
  error: "生成失败"
};

const STATUS_COLORS: Record<AiStreamStatus, string> = {
  idle: "",
  streaming: "text-info",
  completed: "text-success",
  error: "text-error"
};

const TASK_LABELS: Record<string, string> = {
  generate_chapter_draft: "生成草稿",
  continue_chapter: "续写",
  rewrite_selection: "改写",
  deai_text: "去 AI 味",
  scan_consistency: "检查",
  chapter_plan: "章节计划",
  custom: "自定义"
};

/** Task types where showing a diff view is meaningful. */
const DIFF_TASKS = new Set(["rewrite_selection", "deai_text"]);

export function AiPreviewPanel({
  status,
  content,
  errorMessage,
  originalText,
  taskType,
  onInsert,
  onDiscard,
  onCopy
}: AiPreviewPanelProps) {
  const [showDiff, setShowDiff] = useState(true);
  const canDiff = DIFF_TASKS.has(taskType) && status === "completed" && !!originalText;

  if (status === "idle") return null;

  return (
    <div className="border border-primary/20 rounded-xl bg-surface-800/80 backdrop-blur-sm overflow-hidden">
      <div className="flex items-center justify-between px-4 py-2 border-b border-surface-700">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-surface-200">
            {TASK_LABELS[taskType] || "AI 生成"}
          </span>
          <span className={`text-xs ${STATUS_COLORS[status]}`}>
            {STATUS_LABELS[status]}
          </span>
        </div>
        <div className="flex items-center gap-1">
          {canDiff && (
            <button
              onClick={() => setShowDiff(!showDiff)}
              className={`px-2 py-1 text-xs rounded transition-colors ${
                showDiff
                  ? "bg-primary/20 text-primary border border-primary/30"
                  : "bg-surface-700 text-surface-300 border border-surface-600"
              }`}
            >
              {showDiff ? "差异对比" : "生成结果"}
            </button>
          )}
          {status === "completed" && (
            <>
              <button
                onClick={() => onInsert("cursor")}
                className="px-2 py-1 text-xs bg-primary/20 text-primary border border-primary/30 rounded hover:bg-primary/30 transition-colors"
              >
                插入到光标
              </button>
              <button
                onClick={() => onInsert("replace")}
                className="px-2 py-1 text-xs bg-surface-700 text-surface-300 border border-surface-600 rounded hover:bg-surface-600 transition-colors"
              >
                替换选区
              </button>
              <button
                onClick={() => onInsert("append")}
                className="px-2 py-1 text-xs bg-surface-700 text-surface-300 border border-surface-600 rounded hover:bg-surface-600 transition-colors"
              >
                追加末尾
              </button>
              <button
                onClick={onCopy}
                className="px-2 py-1 text-xs bg-surface-700 text-surface-300 border border-surface-600 rounded hover:bg-surface-600 transition-colors"
              >
                复制
              </button>
            </>
          )}
          <button
            onClick={onDiscard}
            className="px-2 py-1 text-xs text-error border border-error/30 rounded hover:bg-error/10 transition-colors"
          >
            丢弃
          </button>
        </div>
      </div>
      <div className="p-4 max-h-64 overflow-y-auto">
        {status === "error" && errorMessage && (
          <div className="mb-3 px-3 py-2 rounded-lg text-sm bg-error/10 text-error border border-error/20">
            {errorMessage}
          </div>
        )}
        {status === "streaming" && !content ? (
          <div className="flex items-center gap-2 text-surface-400 text-sm">
            <span className="w-2 h-2 bg-primary rounded-full animate-pulse" />
            等待 AI 响应...
          </div>
        ) : canDiff && showDiff ? (
          <DiffView original={originalText!} revised={content} />
        ) : (
          <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed">
            {content}
            {status === "streaming" && (
              <span className="inline-block w-1.5 h-4 bg-primary ml-0.5 animate-pulse" />
            )}
          </pre>
        )}
      </div>
    </div>
  );
}
