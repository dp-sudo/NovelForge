import { create } from "zustand";

export type AppRoute =
  | "project-center"
  | "dashboard"
  | "blueprint"
  | "characters"
  | "world"
  | "glossary"
  | "plot"
  | "narrative"
  | "timeline"
  | "relationships"
  | "chapters"
  | "editor"
  | "consistency"
  | "constitution"
  | "constitution-conflicts"
  | "asset-history"
  | "state-tracker"
  | "review-board"
  | "export"
  | "settings";

export interface UiState {
  sidebarCollapsed: boolean;
  activeRoute: AppRoute;
  theme: "dark" | "light";
  previousRoute: AppRoute | null;
  globalError: string | null;
  toggleSidebar: () => void;
  setActiveRoute: (route: AppRoute) => void;
  setTheme: (theme: "dark" | "light") => void;
  setGlobalError: (error: string | null) => void;
}

export const useUiStore = create<UiState>((set) => ({
  sidebarCollapsed: false,
  activeRoute: "project-center",
  theme: "dark",
  previousRoute: null,
  globalError: null,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setActiveRoute: (route) =>
    set((s) => ({ previousRoute: s.activeRoute, activeRoute: route })),
  setTheme: (theme) => set({ theme }),
  setGlobalError: (error) => set({ globalError: error })
}));
