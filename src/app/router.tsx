import { useUiStore, type AppRoute } from "../stores/uiStore";
import { useProjectStore } from "../stores/projectStore";
import { Suspense, lazy, type ComponentType } from "react";
import { AppShell } from "../components/layout/AppShell";
import { PageTransition } from "../components/ui/PageTransition.js";
import { ProjectCenterPage } from "../pages/ProjectCenter/ProjectCenterPage";

const DashboardPage = lazy(async () => {
  const mod = await import("../pages/Dashboard/DashboardPage");
  return { default: mod.DashboardPage };
});
const BlueprintPage = lazy(async () => {
  const mod = await import("../pages/Blueprint/BlueprintPage");
  return { default: mod.BlueprintPage };
});
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
const NarrativePage = lazy(async () => {
  const mod = await import("../pages/Narrative/NarrativePage");
  return { default: mod.NarrativePage };
});
const TimelinePage = lazy(async () => {
  const mod = await import("../pages/Timeline/TimelinePage");
  return { default: mod.TimelinePage };
});
const RelationshipsPage = lazy(async () => {
  const mod = await import("../pages/Relationships/RelationshipsPage");
  return { default: mod.RelationshipsPage };
});
const ChaptersPage = lazy(async () => {
  const mod = await import("../pages/Chapters/ChaptersPage");
  return { default: mod.ChaptersPage };
});
const EditorPage = lazy(async () => {
  const mod = await import("../pages/Editor/EditorPage");
  return { default: mod.EditorPage };
});
const ConsistencyPage = lazy(async () => {
  const mod = await import("../pages/Consistency/ConsistencyPage");
  return { default: mod.ConsistencyPage };
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
  dashboard: DashboardPage,
  blueprint: BlueprintPage,
  characters: CharactersPage,
  world: WorldPage,
  glossary: GlossaryPage,
  plot: PlotPage,
  narrative: NarrativePage,
  timeline: TimelinePage,
  relationships: RelationshipsPage,
  chapters: ChaptersPage,
  editor: EditorPage,
  consistency: ConsistencyPage,
  export: ExportPage,
  settings: SettingsPage
};

function PageContent() {
  const activeRoute = useUiStore((s) => s.activeRoute);
  const Page = routeMap[activeRoute] ?? DashboardPage;
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
