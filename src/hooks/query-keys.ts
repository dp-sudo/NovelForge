export const queryKeys = {
  project: {
    all: ["project"] as const,
    recent: () => [...queryKeys.project.all, "recent"] as const,
    stats: (root: string) => [...queryKeys.project.all, "stats", root] as const,
  },
  blueprint: {
    all: (root: string) => ["blueprint", root] as const,
  },
  chapter: {
    all: (root: string) => ["chapter", root] as const,
    list: (root: string) => [...queryKeys.chapter.all(root), "list"] as const,
    content: (root: string, id: string) => [...queryKeys.chapter.all(root), "content", id] as const,
    snapshots: (root: string, id: string) => [...queryKeys.chapter.all(root), "snapshots", id] as const,
    volumes: (root: string) => [...queryKeys.chapter.all(root), "volumes"] as const,
  },
  character: {
    all: (root: string) => ["character", root] as const,
    relationships: (root: string, id: string) => [...queryKeys.character.all(root), "relationships", id] as const,
  },
  world: {
    all: (root: string) => ["world", root] as const,
  },
  plot: {
    all: (root: string) => ["plot", root] as const,
  },
  consistency: {
    all: (root: string) => ["consistency", root] as const,
  },
  settings: {
    providers: () => ["settings", "providers"] as const,
    models: (id: string) => ["settings", "models", id] as const,
    taskRoutes: () => ["settings", "taskRoutes"] as const,
    editor: () => ["settings", "editor"] as const,
    license: () => ["settings", "license"] as const,
    update: () => ["settings", "update"] as const,
  },
};
