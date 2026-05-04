import { useState, useEffect } from "react";
import {
  listAssetChangeHistory,
  getAssetHistorySummary,
  type AssetChangeRecord,
  type AssetHistorySummary,
  type AssetHistoryFilter,
} from "../../api/assetHistoryApi";
import { Button } from "../../components/ui/Button";
import { Select } from "../../components/forms/Select";
import { Input } from "../../components/forms/Input";
import { Badge } from "../../components/ui/Badge";
import { Spinner } from "../../components/ui/Spinner";
import { useProjectStore } from "../../stores/projectStore";

export function AssetHistoryPage() {
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const [filter, setFilter] = useState<AssetHistoryFilter>({});
  const [selectedAsset, setSelectedAsset] = useState<{
    type: string;
    id: string;
  } | null>(null);

  const [history, setHistory] = useState<AssetChangeRecord[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [historyError, setHistoryError] = useState<Error | null>(null);

  const [summary, setSummary] = useState<AssetHistorySummary | null>(null);
  const [summaryLoading, setSummaryLoading] = useState(false);


  useEffect(() => {
    if (projectRoot) {
      loadHistory();
      loadSummary();
    }
  }, [projectRoot, filter]);

  const loadHistory = async () => {
    if (!projectRoot) return;
    setHistoryLoading(true);
    setHistoryError(null);
    try {
      const data = await listAssetChangeHistory(projectRoot, filter);
      setHistory(data);
    } catch (err) {
      setHistoryError(err as Error);
    } finally {
      setHistoryLoading(false);
    }
  };

  const loadSummary = async () => {
    if (!projectRoot) return;
    setSummaryLoading(true);
    try {
      const data = await getAssetHistorySummary(projectRoot);
      setSummary(data);
    } catch (err) {
      console.error("Failed to load summary:", err);
    } finally {
      setSummaryLoading(false);
    }
  };

  const handleFilterChange = (key: keyof AssetHistoryFilter, value: any) => {
    setFilter((prev) => ({ ...prev, [key]: value }));
  };

  const clearFilters = () => {
    setFilter({});
    setSelectedAsset(null);
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  const getChangeTypeColor = (type: string) => {
    switch (type) {
      case "create":
        return "bg-green-100 text-green-800";
      case "update":
        return "bg-blue-100 text-blue-800";
      case "delete":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const getChangedByColor = (by: string) => {
    switch (by) {
      case "ai":
        return "bg-purple-100 text-purple-800";
      case "user":
        return "bg-indigo-100 text-indigo-800";
      case "system":
        return "bg-gray-100 text-gray-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  return (
    <div className="h-full flex flex-col bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b px-6 py-4">
        <h1 className="text-2xl font-bold text-gray-900">资产变更历史</h1>
        <p className="text-sm text-gray-600 mt-1">
          追踪角色、世界设定等资产的所有变更记录
        </p>
      </div>

      <div className="flex-1 overflow-auto p-6">
        <div className="max-w-7xl mx-auto space-y-6">
          {/* Summary Cards */}
          {summary && !summaryLoading && (
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="bg-white rounded-lg shadow p-4">
                <div className="text-sm text-gray-600">总变更数</div>
                <div className="text-3xl font-bold text-gray-900 mt-1">
                  {summary.totalChanges}
                </div>
              </div>
              <div className="bg-white rounded-lg shadow p-4">
                <div className="text-sm text-gray-600">变更类型分布</div>
                <div className="mt-2 space-y-1">
                  {Object.entries(summary.changesByType).map(([type, count]) => (
                    <div key={type} className="flex justify-between text-sm">
                      <span className="text-gray-700 capitalize">{type}</span>
                      <span className="font-medium">{count}</span>
                    </div>
                  ))}
                </div>
              </div>
              <div className="bg-white rounded-lg shadow p-4">
                <div className="text-sm text-gray-600">变更来源分布</div>
                <div className="mt-2 space-y-1">
                  {Object.entries(summary.changesBySource).map(([source, count]) => (
                    <div key={source} className="flex justify-between text-sm">
                      <span className="text-gray-700 capitalize">{source}</span>
                      <span className="font-medium">{count}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Filters */}
          <div className="bg-white rounded-lg shadow p-4">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">筛选条件</h2>
              <Button variant="ghost" size="sm" onClick={clearFilters}>
                清除筛选
              </Button>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <Select
                label="资产类型"
                value={filter.assetType ?? ""}
                onChange={(e) => handleFilterChange("assetType", e.target.value || undefined)}
                options={[
                  { value: "", label: "全部" },
                  { value: "character", label: "角色" },
                  { value: "world_rule", label: "世界设定" },
                  { value: "plot_node", label: "剧情节点" },
                  { value: "glossary_term", label: "术语" }
                ]}
              />

              <Select
                label="变更来源"
                value={filter.changedBy ?? ""}
                onChange={(e) => handleFilterChange("changedBy", e.target.value || undefined)}
                options={[
                  { value: "", label: "全部" },
                  { value: "user", label: "用户" },
                  { value: "ai", label: "AI" },
                  { value: "system", label: "系统" }
                ]}
              />

              <Input
                label="开始日期"
                type="date"
                value={filter.startDate ?? ""}
                onChange={(e) => handleFilterChange("startDate", e.target.value || undefined)}
              />

              <Input
                label="结束日期"
                type="date"
                value={filter.endDate ?? ""}
                onChange={(e) => handleFilterChange("endDate", e.target.value || undefined)}
              />
            </div>
          </div>

          {/* History List */}
          <div className="bg-white rounded-lg shadow">
            <div className="px-4 py-3 border-b">
              <h2 className="text-lg font-semibold text-gray-900">变更记录</h2>
            </div>

            {historyLoading && (
              <div className="flex items-center justify-center py-12">
                <Spinner size="lg" />
              </div>
            )}

            {historyError && (
              <div className="p-4 text-red-600">
                加载失败: {historyError.message}
              </div>
            )}

            {history && history.length === 0 && (
              <div className="p-8 text-center text-gray-500">
                暂无变更记录
              </div>
            )}

            {history && history.length > 0 && (
              <div className="divide-y">
                {history.map((record) => (
                  <div
                    key={record.id}
                    className="p-4 hover:bg-gray-50 transition-colors"
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-2">
                          <Badge className={getChangeTypeColor(record.changeType)}>
                            {record.changeType}
                          </Badge>
                          <Badge className={getChangedByColor(record.changedBy)}>
                            {record.changedBy}
                          </Badge>
                          <span className="text-sm text-gray-600">
                            {record.assetType}
                          </span>
                          <span className="text-sm font-medium text-gray-900">
                            {record.assetName}
                          </span>
                        </div>

                        {record.fieldName && (
                          <div className="text-sm text-gray-700 mb-2">
                            <span className="font-medium">字段:</span> {record.fieldName}
                          </div>
                        )}

                        {record.changeType === "update" && (
                          <div className="grid grid-cols-2 gap-4 text-sm">
                            <div>
                              <div className="text-gray-600 mb-1">修改前:</div>
                              <div className="bg-red-50 border border-red-200 rounded p-2 text-gray-800">
                                {record.oldValue || "(空)"}
                              </div>
                            </div>
                            <div>
                              <div className="text-gray-600 mb-1">修改后:</div>
                              <div className="bg-green-50 border border-green-200 rounded p-2 text-gray-800">
                                {record.newValue || "(空)"}
                              </div>
                            </div>
                          </div>
                        )}

                        {record.changeReason && (
                          <div className="mt-2 text-sm text-gray-600">
                            <span className="font-medium">原因:</span> {record.changeReason}
                          </div>
                        )}

                        {record.aiTaskType && (
                          <div className="mt-2 text-sm text-purple-600">
                            <span className="font-medium">AI 任务:</span> {record.aiTaskType}
                          </div>
                        )}
                      </div>

                      <div className="text-sm text-gray-500 ml-4">
                        {formatDate(record.createdAt)}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
