import type { PropsWithChildren } from "react";
import { TopBar } from "./TopBar.js";
import { Sidebar } from "./Sidebar.js";
import { StatusBar } from "./StatusBar.js";
import { useUiStore } from "../../stores/uiStore.js";

export function AppShell({ children }: PropsWithChildren) {
  const sidebarCollapsed = useUiStore((s) => s.sidebarCollapsed);

  return (
    <div className="flex flex-col h-screen bg-surface-900 text-surface-100 overflow-hidden">
      <TopBar />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <main
          className={`flex-1 overflow-y-auto transition-all duration-200 ${
            sidebarCollapsed ? "ml-0" : ""
          }`}
        >
          <div className="p-6">{children}</div>
        </main>
      </div>
      <StatusBar />
    </div>
  );
}
