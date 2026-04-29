import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card";
import { Button } from "../../components/ui/Button";
import { Badge } from "../../components/ui/Badge";
import { useEditorStore } from "../../stores/editorStore";
import { readChapterContent } from "../../api/chapterApi";
import { aiScanConsistency, scanChapterConsistency, scanFullConsistency, updateIssueStatus, type ConsistencyIssueRow } from "../../api/consistencyApi";
import { useProjectStore } from "../../stores/projectStore";

const severityColors: Record<string, "error" | "warning" | "info" | "default"> = {
  blocker: "error",
  high: "error",
  medium: "warning",
  low: "info",
  info: "default"
};

const severityLabels: Record<string, string> = {
  blocker: "阻断",
  high: "高",
  medium: "中",
  low: "低",
  info: "信息"
};

const typeLabels: Record<string, string> = {
  glossary: "名词",
  character: "角色",
  world_rule: "世界规则",
  timeline: "时间线",
  prose_style: "文风"
};

const statusLabels: Record<string, string> = {
  open: "未处理",
  ignored: "已忽略",
  fixed: "已修复",
  false_positive: "误报"
};

export function ConsistencyPage() {
  const [issues, setIssues] = useState<ConsistencyIssueRow[]>([]);
  const [selected, setSelected] = useState<ConsistencyIssueRow | null>(null);
  const [scanning, setScanning] = useState(false);
  const [scope, setScope] = useState<"chapter" | "full">("full");
  const [filterSeverity, setFilterSeverity] = useState<string>("all");
  const activeChapterId = useEditorStore((s) => s.activeChapterId);
  const chapterContent = useEditorStore((s) => s.content);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setIssues([]);
      return;
    }
    const data = await scanFullConsistency(projectRoot);
    setIssues(data);
  }, [projectRoot]);

  useEffect(() => { void load(); }, [load]);

  async function handleScan() {
    if (!projectRoot) return;
    setScanning(true);
    try {
      let data: ConsistencyIssueRow[];
      if (scope === "chapter" && activeChapterId) {
        let contentForAi = chapterContent.trim();
        if (!contentForAi) {
          try {
            contentForAi = (await readChapterContent(activeChapterId, projectRoot)).trim();
          } catch {
            contentForAi = "";
          }
        }
        if (contentForAi) {
          await aiScanConsistency({
            projectRoot,
            chapterId: activeChapterId,
            chapterContent: contentForAi,
          });
          data = await scanFullConsistency(projectRoot);
        } else {
          data = await scanChapterConsistency(activeChapterId, projectRoot);
        }
      } else {
        data = await scanFullConsistency(projectRoot);
      }
      setIssues(data);
    } finally {
      setScanning(false);
    }
  }

  function handleStatusChange(issueId: string, status: string) {
    if (!projectRoot) return;
    void updateIssueStatus(issueId, status, projectRoot);
    setIssues((prev) =>
      prev.map((i) => (i.id === issueId ? { ...i, status } : i))
    );
    if (selected?.id === issueId) {
      setSelected((prev) => (prev ? { ...prev, status } : null));
    }
  }

  const filteredIssues = filterSeverity === "all"
    ? issues
    : issues.filter((i) => i.severity === filterSeverity);

  const summary = {
    total: issues.length,
    open: issues.filter((i) => i.status === "open").length,
    high: issues.filter((i) => i.severity === "high" || i.severity === "blocker").length
  };

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-surface-100">一致性检查</h1>
        <div className="flex items-center gap-3">
          <div className="flex bg-surface-800 rounded-lg border border-surface-700 p-0.5">
            <button
              onClick={() => setScope("chapter")}
              className={`px-3 py-1.5 text-xs rounded-md transition-colors ${
                scope === "chapter" ? "bg-surface-700 text-surface-200" : "text-surface-400 hover:text-surface-200"
              }`}
            >
              当前章
            </button>
            <button
              onClick={() => setScope("full")}
              className={`px-3 py-1.5 text-xs rounded-md transition-colors ${
                scope === "full" ? "bg-surface-700 text-surface-200" : "text-surface-400 hover:text-surface-200"
              }`}
            >
              全书
            </button>
          </div>
          <Button
            variant="primary"
            size="sm"
            onClick={() => void handleScan()}
            disabled={scanning}
          >
            {scanning ? "检查中..." : `检查${scope === "chapter" ? "当前章" : "全书"}`}
          </Button>
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <Card padding="md">
          <div className="text-2xl font-bold text-surface-100">{summary.total}</div>
          <div className="text-xs text-surface-400 mt-1">总问题数</div>
        </Card>
        <Card padding="md">
          <div className="text-2xl font-bold text-warning">{summary.open}</div>
          <div className="text-xs text-surface-400 mt-1">未处理</div>
        </Card>
        <Card padding="md">
          <div className="text-2xl font-bold text-error">{summary.high}</div>
          <div className="text-xs text-surface-400 mt-1">高优先级</div>
        </Card>
      </div>

      <div className="flex gap-6">
        {/* Issue List */}
        <div className="flex-1 min-w-0">
          {/* Filter */}
          <div className="flex gap-2 mb-3">
            {[
              { value: "all", label: "全部" },
              { value: "high", label: "高优先级" },
              { value: "medium", label: "中优先级" },
              { value: "low", label: "低" }
            ].map((f) => (
              <button
                key={f.value}
                onClick={() => setFilterSeverity(f.value)}
                className={`px-2.5 py-1 text-xs rounded-lg transition-colors ${
                  filterSeverity === f.value
                    ? "bg-primary/10 text-primary border border-primary/20"
                    : "bg-surface-800 text-surface-400 border border-surface-700 hover:text-surface-200"
                }`}
              >
                {f.label}
              </button>
            ))}
          </div>

          {filteredIssues.length === 0 ? (
            <Card padding="lg">
              <div className="flex items-center justify-center min-h-[200px] text-surface-400">
                <div className="text-center">
                  <p className="text-lg mb-2">暂无问题</p>
                  <p className="text-sm text-surface-500">点击"检查全书"开始扫描</p>
                </div>
              </div>
            </Card>
          ) : (
            <div className="space-y-2">
              {filteredIssues.map((issue) => (
                <button
                  key={issue.id}
                  onClick={() => setSelected(issue)}
                  className={`w-full text-left p-4 rounded-lg border transition-colors ${
                    selected?.id === issue.id
                      ? "bg-primary/10 border-primary/30"
                      : "bg-surface-800 border-surface-700 hover:border-surface-500"
                  }`}
                >
                  <div className="flex items-start gap-3">
                    <Badge variant={severityColors[issue.severity] ?? "default"}>
                      {severityLabels[issue.severity] ?? issue.severity}
                    </Badge>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-xs text-surface-400">
                          {typeLabels[issue.issueType] ?? issue.issueType}
                        </span>
                        <span className="text-xs text-surface-500">
                          {statusLabels[issue.status] ?? issue.status}
                        </span>
                      </div>
                      <p className="text-sm text-surface-200 mt-1 truncate">
                        {issue.explanation}
                      </p>
                      {issue.sourceText && (
                        <p className="text-xs text-surface-400 mt-0.5 truncate">
                          "{issue.sourceText.slice(0, 60)}"
                        </p>
                      )}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Issue Detail */}
        <div className="w-80 shrink-0">
          {selected ? (
            <Card padding="md" className="space-y-4">
              <div className="flex items-center gap-2">
                <Badge variant={severityColors[selected.severity] ?? "default"}>
                  {severityLabels[selected.severity] ?? selected.severity}
                </Badge>
                <span className="text-xs text-surface-400">
                  {typeLabels[selected.issueType] ?? selected.issueType}
                </span>
              </div>

              <div>
                <label className="text-xs text-surface-400">问题说明</label>
                <p className="text-sm text-surface-200 mt-1">{selected.explanation}</p>
              </div>

              {selected.sourceText && (
                <div>
                  <label className="text-xs text-surface-400">原文片段</label>
                  <blockquote className="text-sm text-surface-300 mt-1 pl-3 border-l-2 border-surface-600 italic">
                    "{selected.sourceText}"
                  </blockquote>
                </div>
              )}

              {selected.suggestedFix && (
                <div>
                  <label className="text-xs text-surface-400">修复建议</label>
                  <p className="text-sm text-surface-200 mt-1">{selected.suggestedFix}</p>
                </div>
              )}

              <div className="pt-3 border-t border-surface-700 flex flex-wrap gap-2">
                {selected.status === "open" && (
                  <>
                    <Button variant="primary" size="sm" onClick={() => handleStatusChange(selected.id, "fixed")}>
                      标记已修复
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => handleStatusChange(selected.id, "ignored")}>
                      忽略
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => handleStatusChange(selected.id, "false_positive")}>
                      误报
                    </Button>
                  </>
                )}
                {selected.status !== "open" && (
                  <Button variant="ghost" size="sm" onClick={() => handleStatusChange(selected.id, "open")}>
                    重新打开
                  </Button>
                )}
              </div>
            </Card>
          ) : (
            <Card padding="md">
              <div className="text-center text-surface-500 text-sm py-8">
                选择一个问题查看详情
              </div>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
