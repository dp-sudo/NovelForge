import { useUiStore, type AppRoute } from "../stores/uiStore";
import { useProjectStore } from "../stores/projectStore";
import type { ComponentType } from "react";
import { AppShell } from "../components/layout/AppShell";
import { PageTransition } from "../components/ui/PageTransition.js";
import { ProjectCenterPage } from "../pages/ProjectCenter/ProjectCenterPage";
import { DashboardPage } from "../pages/Dashboard/DashboardPage";
import { BlueprintPage } from "../pages/Blueprint/BlueprintPage";
import { CharactersPage } from "../pages/Characters/CharactersPage";
import { WorldPage } from "../pages/World/WorldPage";
import { GlossaryPage } from "../pages/Glossary/GlossaryPage";
import { PlotPage } from "../pages/Plot/PlotPage";
import { NarrativePage } from "../pages/Narrative/NarrativePage";
import { TimelinePage } from "../pages/Timeline/TimelinePage";
import { RelationshipsPage } from "../pages/Relationships/RelationshipsPage";
import { ChaptersPage } from "../pages/Chapters/ChaptersPage";
import { EditorPage } from "../pages/Editor/EditorPage";
import { ConsistencyPage } from "../pages/Consistency/ConsistencyPage";
import { ExportPage } from "../pages/Export/ExportPage";
import { SettingsPage } from "../pages/Settings/SettingsPage";

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
      <Page />
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
