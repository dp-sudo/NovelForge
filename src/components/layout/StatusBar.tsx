import { useProjectStore } from "../../stores/projectStore.js";
import { useEditorStore } from "../../stores/editorStore.js";
import { useUiStore } from "../../stores/uiStore.js";

export function StatusBar() {
  const stats = useProjectStore((s) => s.stats);
  const saveStatus = useEditorStore((s) => s.saveStatus);
  const lastSavedAt = useEditorStore((s) => s.lastSavedAt);
  const activeRoute = useUiStore((s) => s.activeRoute);
  const globalError = useUiStore((s) => s.globalError);

  const isInProject = activeRoute !== "project-center";

  return (
    <footer className="flex items-center justify-between h-6 px-4 border-t border-surface-700 bg-surface-800 text-xs text-surface-400 shrink-0 select-none">
      <div className="flex items-center gap-4">
        {isInProject && stats && (
          <>
            <span>
              总字数: <span className="text-surface-200">{stats.totalWords.toLocaleString()}</span>
            </span>
            <span className="text-surface-600">|</span>
            <span>
              章节: <span className="text-surface-200">{stats.chapterCount}</span>
            </span>
            <span className="text-surface-600">|</span>
            <span>
              角色: <span className="text-surface-200">{stats.characterCount}</span>
            </span>
          </>
        )}
        {!isInProject && <span>就绪</span>}
      </div>

      <div className="flex items-center gap-4">
        {globalError && (
          <span className="text-error font-medium">{globalError}</span>
        )}
        {lastSavedAt && (
          <span>
            上次保存: <span className="text-surface-300">{lastSavedAt}</span>
          </span>
        )}
        <span className="text-surface-500">NovelForge v0.1.0</span>
      </div>
    </footer>
  );
}
