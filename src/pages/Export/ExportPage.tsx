import { useEffect, useState } from "react";
import { Card } from "../../components/cards/Card";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/dialogs/Modal";
import { Textarea } from "../../components/forms/Textarea";
import { listChapters, type ChapterRecord } from "../../api/chapterApi";
import { exportBook, exportChapter, type ExportFormat } from "../../api/exportApi";
import { runModuleAiTask } from "../../api/moduleAiApi";
import { useProjectStore } from "../../stores/projectStore";

export function ExportPage() {
  const [range, setRange] = useState<"chapter" | "book">("book");
  const [format, setFormat] = useState<ExportFormat>("md");
  const [includeTitle, setIncludeTitle] = useState(true);
  const [includeSummary, setIncludeSummary] = useState(false);
  const [separateByVolume, setSeparateByVolume] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [result, setResult] = useState<{ path: string; content?: string } | null>(null);
  const [chapters, setChapters] = useState<ChapterRecord[]>([]);
  const [selectedChapterId, setSelectedChapterId] = useState("");
  const [showAiReview, setShowAiReview] = useState(false);
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiResult, setAiResult] = useState<string | null>(null);
  const [aiError, setAiError] = useState<string | null>(null);
  const [aiLoading, setAiLoading] = useState(false);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const projectName = useProjectStore((s) => s.currentProject?.name ?? "project");

  useEffect(() => {
    if (!projectRoot) {
      setChapters([]);
      setSelectedChapterId("");
      return;
    }

    listChapters(projectRoot)
      .then((rows) => {
        setChapters(rows);
        setSelectedChapterId((current) => current || rows[0]?.id || "");
      })
      .catch(() => {
        setChapters([]);
        setSelectedChapterId("");
      });
  }, [projectRoot]);

  async function handleExport() {
    if (!projectRoot) {
      return;
    }
    setExporting(true);
    setResult(null);
    try {
      const outputPath = `exports/${projectName}-${Date.now()}.${format}`;
      const options = {
        includeChapterTitle: includeTitle,
        includeChapterSummary: includeSummary,
        separateByVolume,
      };

      if (range === "book") {
        const r = await exportBook(projectRoot, format, outputPath, options);
        setResult({ path: r.outputPath, content: r.content });
      } else if (range === "chapter" && selectedChapterId) {
        const r = await exportChapter(projectRoot, selectedChapterId, format, outputPath, options);
        setResult({ path: r.outputPath, content: r.content });
      }
    } finally {
      setExporting(false);
    }
  }

  return (
    <div className="max-w-3xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-surface-100">导出中心</h1>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setAiPrompt("");
            setAiResult(null);
            setAiError(null);
            setShowAiReview(true);
          }}
          disabled={!projectRoot}
        >
          AI 审阅
        </Button>
      </div>

      <Card padding="lg" className="space-y-6">
        {/* Range */}
        <div>
          <label className="block text-sm font-medium text-surface-200 mb-3">
            导出范围
          </label>
          <div className="flex gap-3">
            {[
              { value: "chapter" as const, label: "当前章节" },
              { value: "book" as const, label: "全书" }
            ].map((opt) => (
              <label
                key={opt.value}
                className={`flex items-center gap-2 px-4 py-2.5 rounded-lg cursor-pointer transition-colors border ${
                  range === opt.value
                    ? "bg-primary/10 border-primary/30 text-primary"
                    : "bg-surface-700 border-surface-600 text-surface-300 hover:bg-surface-600"
                }`}
              >
                <input
                  type="radio"
                  name="export-range"
                  checked={range === opt.value}
                  onChange={() => setRange(opt.value)}
                  className="accent-primary"
                />
                <span className="text-sm">{opt.label}</span>
              </label>
            ))}
          </div>
        </div>

        {range === "chapter" && (
          <div>
            <label className="block text-sm font-medium text-surface-200 mb-3">章节</label>
            <select
              value={selectedChapterId}
              onChange={(e) => setSelectedChapterId(e.target.value)}
              className="w-full px-3 py-2 text-sm bg-surface-700 border border-surface-600 rounded-lg text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary/40"
            >
              {chapters.length === 0 ? (
                <option value="">暂无章节</option>
              ) : (
                chapters.map((chapter) => (
                  <option key={chapter.id} value={chapter.id}>
                    #{chapter.chapterIndex} {chapter.title}
                  </option>
                ))
              )}
            </select>
          </div>
        )}

        {/* Format */}
        <div>
          <label className="block text-sm font-medium text-surface-200 mb-3">
            格式
          </label>
          <div className="flex gap-3">
            {[
              { value: "txt" as const, label: "TXT", desc: "纯文本格式" },
              { value: "md" as const, label: "Markdown", desc: "结构化标记格式" },
              { value: "docx" as const, label: "DOCX", desc: "可编辑文档格式" },
              { value: "pdf" as const, label: "PDF", desc: "固定排版阅读格式" },
              { value: "epub" as const, label: "EPUB", desc: "电子书格式" }
            ].map((opt) => (
              <label
                key={opt.value}
                className={`flex items-center gap-3 px-4 py-2.5 rounded-lg cursor-pointer transition-colors border flex-1 ${
                  format === opt.value
                    ? "bg-primary/10 border-primary/30 text-primary"
                    : "bg-surface-700 border-surface-600 text-surface-300 hover:bg-surface-600"
                }`}
              >
                <input
                  type="radio"
                  name="export-format"
                  checked={format === opt.value}
                  onChange={() => setFormat(opt.value)}
                  className="accent-primary"
                />
                <div>
                  <div className="text-sm font-medium">{opt.label}</div>
                  <div className="text-xs text-surface-400">{opt.desc}</div>
                </div>
              </label>
            ))}
          </div>
        </div>

        {/* Options */}
        <div>
          <label className="block text-sm font-medium text-surface-200 mb-3">
            选项
          </label>
          <div className="space-y-2.5">
            <label className="flex items-center gap-2.5 text-sm text-surface-300 cursor-pointer">
              <input
                type="checkbox"
                checked={includeTitle}
                onChange={(e) => setIncludeTitle(e.target.checked)}
                className="accent-primary w-4 h-4"
              />
              包含章节标题
            </label>
            <label className="flex items-center gap-2.5 text-sm text-surface-300 cursor-pointer">
              <input
                type="checkbox"
                checked={includeSummary}
                onChange={(e) => setIncludeSummary(e.target.checked)}
                className="accent-primary w-4 h-4"
              />
              包含章节摘要
            </label>
            <label className="flex items-center gap-2.5 text-sm text-surface-300 cursor-pointer">
              <input
                type="checkbox"
                checked={separateByVolume}
                onChange={(e) => setSeparateByVolume(e.target.checked)}
                className="accent-primary w-4 h-4"
              />
              按卷分隔
            </label>
          </div>
        </div>

        {/* Action */}
        <div className="pt-3 border-t border-surface-700 flex items-center justify-between">
          <div className="text-xs text-surface-400">
            导出位置: <code className="text-surface-300 bg-surface-700 px-1.5 py-0.5 rounded">exports/</code>
          </div>
          <div className="flex items-center gap-3">
            {exporting && <span className="text-sm text-info">导出中...</span>}
            <Button
              variant="primary"
              onClick={() => void handleExport()}
              disabled={exporting || !projectRoot || (range === "chapter" && !selectedChapterId)}
            >
              开始导出
            </Button>
          </div>
        </div>

        {/* Result */}
        {result && (
          <div className="pt-3 border-t border-surface-700">
            <Card padding="md" className="bg-success/5 border-success/20">
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <span className="text-success text-sm font-medium">导出完成</span>
                </div>
                <button
                  onClick={() => {
                    const slash = result.path.lastIndexOf("/");
                    const backslash = result.path.lastIndexOf("\\");
                    const splitAt = Math.max(slash, backslash);
                    const dir = splitAt > -1 ? result.path.slice(0, splitAt) : result.path;
                    navigator.clipboard.writeText(dir).catch(() => {});
                  }}
                  className="px-3 py-1 text-xs bg-success/20 text-success border border-success/30 rounded-lg hover:bg-success/30 transition-colors"
                >
                  打开目录
                </button>
              </div>
              <p className="text-xs text-surface-400 mb-2">输出路径: {result.path}</p>
              {result.content && (
                <pre className="text-xs text-surface-400 bg-surface-900 p-3 rounded-lg max-h-40 overflow-y-auto whitespace-pre-wrap">
                  {result.content.slice(0, 500)}
                  {result.content.length > 500 ? "..." : ""}
                </pre>
              )}
            </Card>
          </div>
        )}
      </Card>

      <Modal open={showAiReview} onClose={() => setShowAiReview(false)} title="AI 导出审阅" width="lg">
        <div className="space-y-4">
          <Textarea
            label="附加要求（可选）"
            value={aiPrompt}
            onChange={(e) => setAiPrompt(e.target.value)}
            placeholder="例如：优先检查章节衔接断层与术语一致性"
            className="min-h-[90px]"
          />
          <Button
            variant="primary"
            loading={aiLoading}
            onClick={async () => {
              if (!projectRoot) return;
              setAiLoading(true);
              setAiError(null);
              setAiResult(null);
              try {
                const result = await runModuleAiTask({
                  projectRoot,
                  taskType: "export.review",
                  uiAction: "export.ai.review",
                  userInstruction: aiPrompt,
                  persistMode: "derived_review",
                  automationTier: "auto",
                });
                setAiResult(result || "AI 未返回内容。");
              } catch (err) {
                setAiError(err instanceof Error ? err.message : "AI 审阅失败");
              } finally {
                setAiLoading(false);
              }
            }}
            disabled={!projectRoot}
          >
            {aiLoading ? "审阅中..." : "生成导出前审阅"}
          </Button>
          {aiError && (
            <div className="p-3 rounded-lg bg-error/10 border border-error/30 text-sm text-error">
              {aiError}
            </div>
          )}
          {aiResult && (
            <div className="p-4 rounded-xl bg-primary/5 border border-primary/20">
              <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed max-h-80 overflow-y-auto">{aiResult}</pre>
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
}
