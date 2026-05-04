import { useState, useEffect } from "react";
import {
  listConstitutionConflicts,
  detectConstitutionConflicts,
  updateConflictResolution,
  getConflictSummary,
  type ConstitutionConflictWithRules,
} from "../../api/constitutionConflictApi";
import { Button } from "../../components/ui/Button";
import { Badge } from "../../components/ui/Badge";
import { Spinner } from "../../components/ui/Spinner";
import { Modal } from "../../components/dialogs/Modal";
import { Textarea } from "../../components/forms/Textarea";
import { Select } from "../../components/forms/Select";
import { useProjectStore } from "../../stores/projectStore";

export function ConstitutionConflictsPage() {
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const [statusFilter, setStatusFilter] = useState<string>("");
  const [selectedConflict, setSelectedConflict] =
    useState<ConstitutionConflictWithRules | null>(null);
  const [resolutionNote, setResolutionNote] = useState("");
  const [resolutionStatus, setResolutionStatus] = useState<
    "acknowledged" | "resolved" | "false_positive"
  >("acknowledged");

  const [conflicts, setConflicts] = useState<ConstitutionConflictWithRules[]>([]);
  const [conflictsLoading, setConflictsLoading] = useState(false);
  const [conflictsError, setConflictsError] = useState<Error | null>(null);

  const [summary, setSummary] = useState<{
    totalConflicts: number;
    openConflicts: number;
    conflictsBySeverity: Record<string, number>;
    conflictsByType: Record<string, number>;
  } | null>(null);
  const [summaryLoading, setSummaryLoading] = useState(false);

  const [detectingConflicts, setDetectingConflicts] = useState(false);


  useEffect(() => {
    if (projectRoot) {
      loadConflicts();
      loadSummary();
    }
  }, [projectRoot, statusFilter]);

  const loadConflicts = async () => {
    if (!projectRoot) return;
    setConflictsLoading(true);
    setConflictsError(null);
    try {
      const data = await listConstitutionConflicts(
        projectRoot,
        statusFilter as any || undefined
      );
      setConflicts(data);
    } catch (err) {
      setConflictsError(err as Error);
    } finally {
      setConflictsLoading(false);
    }
  };

  const loadSummary = async () => {
    if (!projectRoot) return;
    setSummaryLoading(true);
    try {
      const data = await getConflictSummary(projectRoot);
      setSummary(data);
    } catch (err) {
      console.error("Failed to load summary:", err);
    } finally {
      setSummaryLoading(false);
    }
  };

  const handleDetectConflicts = async () => {
    if (!projectRoot) return;
    setDetectingConflicts(true);
    try {
      const result = await detectConstitutionConflicts(projectRoot);
      if (result) {
        alert(
          `检测完成！\n检查了 ${result.totalRulesChecked} 条规则\n发现 ${result.conflictsFound} 个冲突`
        );
        loadConflicts();
        loadSummary();
      }
    } catch (err) {
      alert(`检测失败: ${(err as Error).message}`);
    } finally {
      setDetectingConflicts(false);
    }
  };

  const handleResolveConflict = async () => {
    if (!selectedConflict || !projectRoot) return;
    try {
      await updateConflictResolution(
        projectRoot,
        selectedConflict.id,
        resolutionStatus,
        resolutionNote
      );
      setSelectedConflict(null);
      setResolutionNote("");
      loadConflicts();
      loadSummary();
    } catch (err) {
      alert(`更新失败: ${(err as Error).message}`);
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case "high":
        return "bg-red-100 text-red-800";
      case "medium":
        return "bg-yellow-100 text-yellow-800";
      case "low":
        return "bg-blue-100 text-blue-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "open":
        return "bg-red-100 text-red-800";
      case "acknowledged":
        return "bg-yellow-100 text-yellow-800";
      case "resolved":
        return "bg-green-100 text-green-800";
      case "false_positive":
        return "bg-gray-100 text-gray-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const getConflictTypeLabel = (type: string) => {
    switch (type) {
      case "direct_contradiction":
        return "直接矛盾";
      case "logical_inconsistency":
        return "逻辑不一致";
      case "temporal_conflict":
        return "时间冲突";
      default:
        return type;
    }
  };

  return (
    <div className="h-full flex flex-col bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">宪法冲突检测</h1>
            <p className="text-sm text-gray-600 mt-1">
              检测并管理宪法规则之间的潜在矛盾
            </p>
          </div>
          <Button
            onClick={handleDetectConflicts}
            disabled={detectingConflicts}
          >
            {detectingConflicts ? (
              <>
                <Spinner size="sm" className="mr-2" />
                检测中...
              </>
            ) : (
              "运行冲突检测"
            )}
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-6">
        <div className="max-w-7xl mx-auto space-y-6">
          {/* Summary Cards */}
          {summary && !summaryLoading && (
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div className="bg-white rounded-lg shadow p-4">
                <div className="text-sm text-gray-600">总冲突数</div>
                <div className="text-3xl font-bold text-gray-900 mt-1">
                  {summary.totalConflicts}
                </div>
              </div>
              <div className="bg-white rounded-lg shadow p-4">
                <div className="text-sm text-gray-600">待处理</div>
                <div className="text-3xl font-bold text-red-600 mt-1">
                  {summary.openConflicts}
                </div>
              </div>
              <div className="bg-white rounded-lg shadow p-4">
                <div className="text-sm text-gray-600">严重程度分布</div>
                <div className="mt-2 space-y-1">
                  {Object.entries(summary.conflictsBySeverity).map(
                    ([severity, count]) => (
                      <div key={severity} className="flex justify-between text-sm">
                        <span className="text-gray-700 capitalize">{severity}</span>
                        <span className="font-medium">{count}</span>
                      </div>
                    )
                  )}
                </div>
              </div>
              <div className="bg-white rounded-lg shadow p-4">
                <div className="text-sm text-gray-600">冲突类型分布</div>
                <div className="mt-2 space-y-1">
                  {Object.entries(summary.conflictsByType).map(([type, count]) => (
                    <div key={type} className="flex justify-between text-sm">
                      <span className="text-gray-700 text-xs">
                        {getConflictTypeLabel(type)}
                      </span>
                      <span className="font-medium">{count}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Filters */}
          <div className="bg-white rounded-lg shadow p-4">
            <div className="flex items-center gap-4">
              <Select
                label="状态筛选"
                value={statusFilter}
                onChange={(e) => setStatusFilter(e.target.value)}
                options={[
                  { value: "", label: "全部" },
                  { value: "open", label: "待处理" },
                  { value: "acknowledged", label: "已确认" },
                  { value: "resolved", label: "已解决" },
                  { value: "false_positive", label: "误报" }
                ]}
              />
            </div>
          </div>

          {/* Conflicts List */}
          <div className="bg-white rounded-lg shadow">
            <div className="px-4 py-3 border-b">
              <h2 className="text-lg font-semibold text-gray-900">冲突列表</h2>
            </div>

            {conflictsLoading && (
              <div className="flex items-center justify-center py-12">
                <Spinner size="lg" />
              </div>
            )}

            {conflictsError && (
              <div className="p-4 text-red-600">
                加载失败: {conflictsError.message}
              </div>
            )}

            {conflicts && conflicts.length === 0 && (
              <div className="p-8 text-center text-gray-500">
                {statusFilter ? "没有符合条件的冲突" : "暂无冲突记录"}
              </div>
            )}

            {conflicts && conflicts.length > 0 && (
              <div className="divide-y">
                {conflicts.map((conflict) => (
                  <div
                    key={conflict.id}
                    className="p-4 hover:bg-gray-50 transition-colors"
                  >
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <Badge className={getSeverityColor(conflict.severity)}>
                          {conflict.severity}
                        </Badge>
                        <Badge className={getStatusColor(conflict.resolutionStatus)}>
                          {conflict.resolutionStatus}
                        </Badge>
                        <span className="text-sm text-gray-600">
                          {getConflictTypeLabel(conflict.conflictType)}
                        </span>
                        {conflict.aiDetected && (
                          <Badge className="bg-purple-100 text-purple-800">
                            AI 检测
                          </Badge>
                        )}
                      </div>
                      <span className="text-sm text-gray-500">
                        {new Date(conflict.detectedAt).toLocaleDateString("zh-CN")}
                      </span>
                    </div>

                    <div className="space-y-3">
                      <div className="bg-blue-50 border border-blue-200 rounded p-3">
                        <div className="text-xs text-blue-600 font-medium mb-1">
                          规则 A ({conflict.ruleA.ruleType})
                        </div>
                        <div className="text-sm text-gray-800">
                          {conflict.ruleA.content}
                        </div>
                      </div>

                      <div className="bg-orange-50 border border-orange-200 rounded p-3">
                        <div className="text-xs text-orange-600 font-medium mb-1">
                          规则 B ({conflict.ruleB.ruleType})
                        </div>
                        <div className="text-sm text-gray-800">
                          {conflict.ruleB.content}
                        </div>
                      </div>

                      <div className="bg-yellow-50 border border-yellow-200 rounded p-3">
                        <div className="text-xs text-yellow-700 font-medium mb-1">
                          冲突说明
                        </div>
                        <div className="text-sm text-gray-800">
                          {conflict.explanation}
                        </div>
                      </div>

                      {conflict.resolutionNote && (
                        <div className="bg-green-50 border border-green-200 rounded p-3">
                          <div className="text-xs text-green-700 font-medium mb-1">
                            解决方案
                          </div>
                          <div className="text-sm text-gray-800">
                            {conflict.resolutionNote}
                          </div>
                        </div>
                      )}
                    </div>

                    {conflict.resolutionStatus === "open" && (
                      <div className="mt-3 flex justify-end">
                        <Button
                          size="sm"
                          onClick={() => {
                            setSelectedConflict(conflict);
                            setResolutionNote(conflict.resolutionNote || "");
                          }}
                        >
                          处理冲突
                        </Button>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Resolution Modal */}
      {selectedConflict && (
        <Modal
          open={true}
          onClose={() => setSelectedConflict(null)}
          title="处理冲突"
        >
          <div className="space-y-4">
            <Select
              label="处理状态"
              value={resolutionStatus}
              onChange={(e) =>
                setResolutionStatus(
                  e.target.value as "acknowledged" | "resolved" | "false_positive"
                )
              }
              options={[
                { value: "acknowledged", label: "已确认（待解决）" },
                { value: "resolved", label: "已解决" },
                { value: "false_positive", label: "误报" }
              ]}
            />

            <Textarea
              label="解决方案或说明"
              value={resolutionNote}
              onChange={(e) => setResolutionNote(e.target.value)}
              rows={4}
              placeholder="描述如何解决此冲突，或说明为什么这不是真正的冲突..."
            />

            <div className="flex justify-end gap-2">
              <Button
                variant="ghost"
                onClick={() => setSelectedConflict(null)}
              >
                取消
              </Button>
              <Button onClick={handleResolveConflict}>确认</Button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
