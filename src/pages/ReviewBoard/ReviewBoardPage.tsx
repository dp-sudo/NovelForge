import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Badge } from "../../components/ui/Badge.js";
import { Select } from "../../components/forms/Select.js";
import {
  listReviewWorkItems,
  updateReviewQueueItemStatus,
} from "../../api/contextApi.js";
import type { ChapterContext } from "../../api/contextApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

type ReviewItem = ChapterContext["reviewQueue"][number];
type ReviewStatus = "pending" | "resolved" | "rejected";

const STATUS_OPTIONS = [
  { value: "all", label: "全部" },
  { value: "pending", label: "待审查" },
  { value: "resolved", label: "已通过" },
  { value: "rejected", label: "已驳回" },
];

const SEVERITY_VARIANT: Record<string, "error" | "warning" | "info" | "default"> = {
  critical: "error",
  high: "error",
  medium: "warning",
  low: "info",
};

export function ReviewBoardPage() {
  const [items, setItems] = useState<ReviewItem[]>([]);
  const [statusFilter, setStatusFilter] = useState("all");
  const [updating, setUpdating] = useState<string | null>(null);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setItems([]);
      return;
    }
    const data = await listReviewWorkItems(projectRoot, {
      status:
        statusFilter === "all"
          ? undefined
          : (statusFilter as ReviewStatus),
      limit: 200,
    });
    setItems(data);
  }, [projectRoot, statusFilter]);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleUpdateStatus(
    itemId: string,
    status: ReviewStatus
  ) {
    if (!projectRoot) return;
    setUpdating(itemId);
    try {
      await updateReviewQueueItemStatus(projectRoot, itemId, status);
      await load();
    } finally {
      setUpdating(null);
    }
  }

  const pendingCount = items.filter((i) => i.status === "pending").length;
  const resolvedCount = items.filter((i) => i.status === "resolved").length;
  const rejectedCount = items.filter((i) => i.status === "rejected").length;

  // When filter is applied on backend, we show all returned items
  const displayed = items;

  return (
    <div className="max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-surface-100">审查看板</h1>
          <p className="text-sm text-surface-400 mt-1">
            AI 生成结果的质量审查闭环：审查 → 通过/驳回 → 进入下一阶段
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={() => void load()}>
          刷新
        </Button>
      </div>

      {/* Stats row */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <Card padding="md">
          <p className="text-xs text-surface-400">待审查</p>
          <p className="text-2xl font-bold text-warning mt-1">
            {pendingCount}
          </p>
        </Card>
        <Card padding="md">
          <p className="text-xs text-surface-400">已通过</p>
          <p className="text-2xl font-bold text-success mt-1">
            {resolvedCount}
          </p>
        </Card>
        <Card padding="md">
          <p className="text-xs text-surface-400">已驳回</p>
          <p className="text-2xl font-bold text-error mt-1">
            {rejectedCount}
          </p>
        </Card>
      </div>

      {/* Filter */}
      <div className="mb-4 w-48">
        <Select
          label="状态筛选"
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
          options={STATUS_OPTIONS}
        />
      </div>

      {/* Items list */}
      {displayed.length === 0 ? (
        <Card padding="lg" className="text-center">
          <p className="text-surface-400 text-sm">暂无审查项</p>
        </Card>
      ) : (
        <div className="space-y-2">
          {displayed.map((item) => (
            <Card
              key={item.id}
              padding="md"
              className="flex items-start justify-between gap-4"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <Badge
                    variant={
                      SEVERITY_VARIANT[item.severity] ?? "default"
                    }
                  >
                    {item.severity}
                  </Badge>
                  <Badge
                    variant={
                      item.status === "pending"
                        ? "warning"
                        : item.status === "resolved"
                          ? "success"
                          : "error"
                    }
                  >
                    {item.status === "pending"
                      ? "待审"
                      : item.status === "resolved"
                        ? "已通过"
                        : "已驳回"}
                  </Badge>
                  <span className="text-xs text-surface-500">
                    {item.taskType}
                  </span>
                </div>
                <p className="text-sm font-medium text-surface-100">
                  {item.title}
                </p>
                <p className="text-xs text-surface-300 mt-1">
                  {item.message}
                </p>
                <p className="text-xs text-surface-500 mt-1">
                  {item.createdAt}
                </p>
              </div>

              {item.status === "pending" && (
                <div className="flex gap-2 shrink-0">
                  <Button
                    variant="primary"
                    size="sm"
                    loading={updating === item.id}
                    onClick={() =>
                      void handleUpdateStatus(item.id, "resolved")
                    }
                  >
                    通过
                  </Button>
                  <Button
                    variant="danger"
                    size="sm"
                    loading={updating === item.id}
                    onClick={() =>
                      void handleUpdateStatus(item.id, "rejected")
                    }
                  >
                    驳回
                  </Button>
                </div>
              )}
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
