import { useUiStore, type AppRoute } from "../../stores/uiStore.js";
import { useProjectStore } from "../../stores/projectStore.js";

interface NavItem {
  route: AppRoute;
  label: string;
  icon: string;
}

const navItems: NavItem[] = [
  { route: "dashboard", label: "仪表盘", icon: "📊" },
  { route: "blueprint", label: "蓝图", icon: "📐" },
  { route: "characters", label: "角色", icon: "👤" },
  { route: "world", label: "世界", icon: "🌍" },
  { route: "glossary", label: "名词库", icon: "📖" },
  { route: "plot", label: "剧情", icon: "📈" },
  { route: "narrative", label: "叙事义务", icon: "🧭" },
  { route: "timeline", label: "时间线", icon: "🕒" },
  { route: "relationships", label: "关系图", icon: "🕸️" },
  { route: "chapters", label: "章节", icon: "📑" },
  { route: "constitution", label: "宪法", icon: "📜" },
  { route: "state-tracker", label: "状态", icon: "📸" },
  { route: "review-board", label: "审查", icon: "✅" },
  { route: "consistency", label: "检查", icon: "🔍" },
  { route: "export", label: "导出", icon: "📤" },
  { route: "settings", label: "设置", icon: "⚙️" }
];

export function Sidebar() {
  const activeRoute = useUiStore((s) => s.activeRoute);
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const clearProject = useProjectStore((s) => s.clearCurrentProject);
  const setRoute = useUiStore((s) => s.setActiveRoute);

  function handleNavigate(route: AppRoute) {
    setActiveRoute(route);
  }

  function handleBackToProjects() {
    clearProject();
    setRoute("project-center");
  }

  return (
    <nav
      className={`flex flex-col border-r border-surface-700 bg-surface-800 transition-all duration-200 shrink-0 ${
        collapsed ? "w-14" : "w-48"
      }`}
    >
      <div className="flex-1 overflow-y-auto py-2">
        {navItems.map((item) => {
          const isActive = activeRoute === item.route;
          return (
            <button
              key={item.route}
              onClick={() => handleNavigate(item.route)}
              className={`w-full flex items-center gap-3 px-3 py-2.5 text-sm transition-colors border-l-2 ${
                isActive
                  ? "bg-primary/10 text-primary border-l-primary"
                  : "text-surface-300 border-l-transparent hover:bg-surface-700 hover:text-surface-100"
              }`}
              title={collapsed ? item.label : undefined}
            >
              <span className="text-base shrink-0">{item.icon}</span>
              {!collapsed && (
                <span className="truncate">{item.label}</span>
              )}
            </button>
          );
        })}
      </div>

      <div className="border-t border-surface-700 p-2">
        <button
          onClick={toggleSidebar}
          className="w-full flex items-center justify-center gap-2 px-2 py-2 text-xs text-surface-400 hover:text-surface-200 transition-colors"
          title={collapsed ? "展开侧栏" : "折叠侧栏"}
        >
          {collapsed ? "▶" : "◀ 折叠"}
        </button>
        <button
          onClick={handleBackToProjects}
          className="w-full flex items-center justify-center gap-2 px-2 py-2 text-xs text-surface-400 hover:text-surface-200 transition-colors"
          title="返回项目中心"
        >
          {collapsed ? "🏠" : "🏠 项目中心"}
        </button>
      </div>
    </nav>
  );
}
