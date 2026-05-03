import { useEffect, useState, type PropsWithChildren } from "react";
import { TopBar } from "./TopBar.js";
import { Sidebar } from "./Sidebar.js";
import { StatusBar } from "./StatusBar.js";
import { CommandPalette } from "./CommandPalette.js";
import { useUiStore } from "../../stores/uiStore.js";

/** Maps command palette paths to AppRoute names. */
const PATH_TO_ROUTE: Record<string, string> = {
  "/": "project-center",
  "/dashboard": "dashboard",
  "/blueprint": "blueprint",
  "/characters": "characters",
  "/world": "world",
  "/glossary": "glossary",
  "/plot": "plot",
  "/chapters": "chapters",
  "/editor": "editor",
  "/consistency": "consistency",
  "/constitution": "constitution",
  "/state-tracker": "state-tracker",
  "/review-board": "review-board",
  "/export": "export",
  "/settings": "settings",
  "/narrative": "narrative",
  "/timeline": "timeline",
  "/relationships": "relationships",
};

export function AppShell({ children }: PropsWithChildren) {
  const sidebarCollapsed = useUiStore((s) => s.sidebarCollapsed);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const [paletteOpen, setPaletteOpen] = useState(false);

  // Global keyboard shortcuts: Ctrl+P (command palette), Ctrl+F (find in page)
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "p") {
        e.preventDefault();
        setPaletteOpen((o) => !o);
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, []);

  function handleNavigate(path: string) {
    const route = PATH_TO_ROUTE[path];
    if (route) {
      (setActiveRoute as (r: string) => void)(route);
    }
  }

  return (
    <div className="flex flex-col h-screen bg-background text-foreground overflow-hidden">
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

      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onNavigate={handleNavigate}
      />
    </div>
  );
}
