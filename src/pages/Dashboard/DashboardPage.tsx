import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore.js";
import { useUiStore } from "../../stores/uiStore.js";
import { Card } from "../../components/cards/Card.js";
import { Badge } from "../../components/ui/Badge.js";
import { getDashboardStats, type DashboardStats } from "../../api/statsApi.js";

export function DashboardPage() {
  const project = useProjectStore((s) => s.currentProject);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);

  useEffect(() => {
    if (!projectRoot) {
      setStats(null);
      return;
    }
    getDashboardStats(projectRoot).then(setStats).catch(() => {
      setStats(null);
    });
  }, [projectRoot]);

  const statCards = [
    { label: "总字数", value: stats?.totalWords.toLocaleString() ?? "0", color: "text-info" },
    { label: "章节数", value: stats?.chapterCount ?? 0, color: "text-success" },
    { label: "角色数", value: stats?.characterCount ?? 0, color: "text-primary" },
    { label: "设定数", value: stats?.worldRuleCount ?? 0, color: "text-warning" },
    { label: "剧情节点", value: stats?.plotNodeCount ?? 0, color: "text-surface-200" },
    { label: "未解决问题", value: stats?.openIssueCount ?? 0, color: "text-error" }
  ];

  const shortcuts = [
    { label: "继续写作", route: "chapters" as const, icon: "✍️" },
    { label: "完成蓝图", route: "blueprint" as const, icon: "📐" },
    { label: "创建角色", route: "characters" as const, icon: "👤" },
    { label: "创建章节", route: "chapters" as const, icon: "📑" },
    { label: "运行检查", route: "consistency" as const, icon: "🔍" },
    { label: "导出作品", route: "export" as const, icon: "📤" }
  ];

  return (
    <div className="max-w-5xl mx-auto">
      <h1 className="text-2xl font-bold text-surface-100 mb-1">
        {project?.name ?? "项目仪表盘"}
      </h1>
      <p className="text-sm text-surface-400 mb-6">
        {project?.genre ? `类型: ${project.genre}` : ""}
        {project?.targetWords
          ? ` · 目标: ${project.targetWords.toLocaleString()} 字`
          : ""}
      </p>

      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8">
        {statCards.map((card) => (
          <Card key={card.label} padding="md" className="text-center">
            <div className={`text-2xl font-bold ${card.color}`}>{card.value}</div>
            <div className="text-xs text-surface-400 mt-1">{card.label}</div>
          </Card>
        ))}
      </div>

      <Card padding="lg" className="mb-6">
        <h2 className="text-sm font-semibold text-surface-200 mb-3">创作进度</h2>
        <div className="flex items-center gap-3">
          <div className="flex-1 h-2 bg-surface-700 rounded-full overflow-hidden">
            <div
              className="h-full bg-primary rounded-full transition-all duration-500"
              style={{ width: `${Math.min(stats?.blueprintProgress ?? 0, 100)}%` }}
            />
          </div>
          <span className="text-sm text-surface-300 shrink-0">
            {stats?.blueprintProgress ?? 0}%
          </span>
        </div>
        {stats && stats.openIssueCount > 0 && (
          <div className="mt-3">
            <Badge variant="error">{stats.openIssueCount} 个未解决问题</Badge>
          </div>
        )}
      </Card>

      <h2 className="text-sm font-semibold text-surface-200 mb-3">快捷操作</h2>
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
        {shortcuts.map((item) => (
          <button
            key={item.label}
            onClick={() => setActiveRoute(item.route)}
            className="flex flex-col items-center gap-2 p-4 bg-surface-800 border border-surface-700 rounded-xl hover:border-surface-500 transition-colors"
          >
            <span className="text-xl">{item.icon}</span>
            <span className="text-xs text-surface-300">{item.label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
