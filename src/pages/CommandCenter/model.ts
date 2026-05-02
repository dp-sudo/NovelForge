export const SYSTEM_NAV_ROUTES = ["command-center", "export", "settings"] as const;

export type SystemNavRoute = (typeof SYSTEM_NAV_ROUTES)[number];

export interface CommandCenterChapterLike {
  id: string;
  chapterIndex: number;
  status: string;
}

function compareByChapterIndex(
  left: CommandCenterChapterLike,
  right: CommandCenterChapterLike,
): number {
  return left.chapterIndex - right.chapterIndex;
}

export function selectCommandCenterChapter<T extends CommandCenterChapterLike>(
  chapters: T[],
  preferredChapterId?: string | null,
): T | null {
  if (chapters.length === 0) {
    return null;
  }

  const explicit = preferredChapterId
    ? chapters.find((chapter) => chapter.id === preferredChapterId)
    : undefined;
  if (explicit) {
    return explicit;
  }

  const sorted = [...chapters].sort(compareByChapterIndex);
  return (
    sorted.find((chapter) => chapter.status === "drafting")
    ?? sorted.find((chapter) => chapter.status === "revising")
    ?? sorted.find((chapter) => chapter.status === "planned")
    ?? sorted.find((chapter) => chapter.status !== "archived")
    ?? sorted[0]
    ?? null
  );
}
