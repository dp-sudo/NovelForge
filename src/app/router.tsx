import { useUiStore, type AppRoute } from "../stores/uiStore";
import { useProjectStore } from "../stores/projectStore";
import { Suspense, lazy, type ComponentType } from "react";
import { AppShell } from "../components/layout/AppShell";
import { PageTransition } from "../components/ui/PageTransition.js";
import { ProjectCenterPage } from "../pages/ProjectCenter/ProjectCenterPage";
import { CommandCenterPage } from "../pages/CommandCenter/CommandCenterPage.js";

const CharactersPage = lazy(async () => {
  const mod = await import("../pages/Characters/CharactersPage");
  return { default: mod.CharactersPage };
});
const WorldPage = lazy(async () => {
  const mod = await import("../pages/World/WorldPage");
  return { default: mod.WorldPage };
});
const GlossaryPage = lazy(async () => {
  const mod = await import("../pages/Glossary/GlossaryPage");
  return { default: mod.GlossaryPage };
});
const PlotPage = lazy(async () => {
  const mod = await import("../pages/Plot/PlotPage");
  return { default: mod.PlotPage };
});
const TimelinePage = lazy(async () => {
  const mod = await import("../pages/Timeline/TimelinePage");
  return { default: mod.TimelinePage };
});
const RelationshipsPage = lazy(async () => {
  const mod = await import("../pages/Relationships/RelationshipsPage");
  return { default: mod.RelationshipsPage };
});
const ExportPage = lazy(async () => {
  const mod = await import("../pages/Export/ExportPage");
  return { default: mod.ExportPage };
});
const SettingsPage = lazy(async () => {
  const mod = await import("../pages/Settings/SettingsPage");
  return { default: mod.SettingsPage };
});

const routeMap: Record<AppRoute, ComponentType> = {
  "project-center": ProjectCenterPage,
  "command-center": CommandCenterPage,
  blueprint: CommandCenterPage,
  characters: CharactersPage,
  world: WorldPage,
  glossary: GlossaryPage,
  plot: PlotPage,
  narrative: CommandCenterPage,
  timeline: TimelinePage,
  relationships: RelationshipsPage,
  chapters: CommandCenterPage,
  consistency: CommandCenterPage,
  export: ExportPage,
  settings: SettingsPage
};

function PageContent() {
  const activeRoute = useUiStore((s) => s.activeRoute);
  const Page = routeMap[activeRoute] ?? CommandCenterPage;
  return (
    <PageTransition routeKey={activeRoute}>
      <Suspense fallback={<div className="px-4 py-3 text-xs text-surface-400">页面加载中...</div>}>
        <Page />
      </Suspense>
    </PageTransition>
  );
}

export function AppRouter() {
  const activeRoute = useUiStore((s) => s.activeRoute);
  const currentProject = useProjectStore((s) => s.currentProject);
  const isInProject =
    activeRoute !== "project-center" && currentProject !== null;

  if (!isInProject) {
    return <ProjectCenterPage />;
  }

  return (
    <AppShell>
      <PageContent />
    </AppShell>
  );
}
