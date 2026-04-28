import { useProjectStore } from "../../stores/projectStore.js";
import { useUiStore } from "../../stores/uiStore.js";
import { Card } from "../../components/cards/Card.js";
import { Badge } from "../../components/ui/Badge.js";
import { useDashboardStats } from "../../hooks/useApi.js";

export function DashboardPage() {
  const project = useProjectStore((s) => s.currentProject);
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const { data: stats, isLoading } = useDashboardStats(projectRoot);

  const statCards = [
    { label: "总字数", value: stats?.totalWords.toLocaleString() ?? "0", color: "text-info" },
    { label: "章节数", value: stats?.chapterCount ?? 0, color: "text-success" },
    { label: "角色数", value: stats?.characterCount ?? 0, color: "text-primary" },
    { label: "设定数", value: stats?.worldRuleCount ?? 0, color: "text-warning" },
    { label: "剧情节点", value: stats?.plotNodeCount ?? 0, color: "text-surface-200" },
    { label: "未解决问题", value: stats?.openIssueCount ?? 0, color: "text-error" },
  ];

  const shortcuts = [
    { label: "继续写作", route: "chapters" as const, icon: "✍️" },
    { label: "完成蓝图", route: "blueprint" as const, icon: "📐" },
    { label: "创建角色", route: "characters" as const, icon: "👤" },
    { label: "创建章节", route: "chapters" as const, icon: "📑" },
    { label: "运行检查", route: "consistency" as const, icon: "🔍" },
    { label: "导出作品", route: "export" as const, icon: "📤" },
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

      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8 stagger-children">
        {statCards.map((card) => (
          <Card key={card.label} padding="md" className="text-center">
            <div className={`text-2xl font-bold ${card.color}`}>{isLoading ? "..." : card.value}</div>
            <div className="text-xs text-surface-400 mt-1">{card.label}</div>
          </Card>
        ))}
      </div>

      <Card padding="lg" className="mb-6">
        <h2 className="text-sm font-semibold text-surface-200 mb-3">创作进度</h2>
        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span className="text-surface-400">蓝图完成度</span>
            <span className="text-surface-200">{stats?.blueprintProgress ?? 0}%</span>
          </div>
          <div className="w-full bg-surface-700 rounded-full h-2">
            <div
              className="bg-primary h-2 rounded-full transition-all duration-500"
              style={{ width: `${stats?.blueprintProgress ?? 0}%` }}
            />
          </div>
        </div>
        <div className="grid grid-cols-3 gap-4 mt-4 text-sm">
          <div>
            <span className="text-surface-400">已完成章节</span>
            <p className="text-success font-semibold">{stats?.completedChapterCount ?? stats?.chapterCount ?? 0}</p>
          </div>
          <div>
            <span className="text-surface-400">未解决问题</span>
            <p className="text-error font-semibold">{stats?.openIssueCount ?? 0}</p>
          </div>
          <div>
            <span className="text-surface-400">目标字数</span>
            <p className="text-surface-200 font-semibold">{project?.targetWords?.toLocaleString() ?? 0}</p>
          </div>
        </div>
      </Card>

      <h2 className="text-sm font-semibold text-surface-200 mb-3">快捷操作</h2>
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        {shortcuts.map((s) => (
          <button
            key={s.label}
            onClick={() => setActiveRoute(s.route)}
            className="flex flex-col items-center gap-2 p-4 bg-surface-800 border border-surface-700 rounded-xl hover:border-surface-500 hover:bg-surface-700 transition-colors"
          >
            <span className="text-2xl">{s.icon}</span>
            <span className="text-xs text-surface-300">{s.label}</span>
          </button>
        ))}
      </div>

      {stats?.recentChapters && stats.recentChapters.length > 0 && (
        <Card padding="lg" className="mt-6">
          <h2 className="text-sm font-semibold text-surface-200 mb-3">最近编辑</h2>
          <div className="space-y-2">
            {stats.recentChapters.map((ch) => (
              <div key={ch.id} className="flex items-center justify-between py-1.5">
                <span className="text-sm text-surface-200">{ch.title}</span>
                <span className="text-xs text-surface-400">{ch.updatedAt.slice(0, 10)}</span>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
