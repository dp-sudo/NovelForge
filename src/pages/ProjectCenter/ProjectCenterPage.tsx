import { useEffect, useState } from "react";
import { useUiStore } from "../../stores/uiStore.js";
import { useProjectStore } from "../../stores/projectStore.js";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Select } from "../../components/forms/Select.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { clearRecentProjects, createProject, listRecentProjects, openProject, validateProjectName } from "../../api/projectApi.js";
import type { AppErrorDto } from "../../types/error.js";

const GENRES = ["玄幻", "都市", "科幻", "悬疑", "言情", "历史", "奇幻", "轻小说", "剧本", "其他"];
const WINDOWS_INVALID_PATH_CHARS = /[<>:"|?*]/;

function validateWindowsDirectoryPath(path: string): string | null {
  const value = path.trim();
  if (!value) {
    return "保存目录不能为空";
  }
  const isDrivePath = /^[a-zA-Z]:[\\/]/.test(value);
  const isUncPath = /^\\\\[^\\\/]+[\\\/][^\\\/]+/.test(value);
  if (!isDrivePath && !isUncPath) {
    return "保存目录需为 Windows 绝对路径";
  }
  const valueWithoutDrive = value.replace(/^[a-zA-Z]:/, "");
  if (WINDOWS_INVALID_PATH_CHARS.test(valueWithoutDrive)) {
    return "保存目录包含非法字符";
  }
  return null;
}

function getErrorMessage(error: unknown, fallback: string): string {
  if (typeof error === "object" && error !== null) {
    const maybeError = error as { message?: unknown; suggestedAction?: unknown };
    if (typeof maybeError.message === "string" && maybeError.message.trim()) {
      const suggestion = typeof maybeError.suggestedAction === "string" && maybeError.suggestedAction.trim()
        ? `（${maybeError.suggestedAction}）`
        : "";
      return `${maybeError.message}${suggestion}`;
    }
  }
  return fallback;
}

function asAppError(error: unknown): AppErrorDto | null {
  if (typeof error !== "object" || error === null) {
    return null;
  }
  const maybeError = error as Partial<AppErrorDto>;
  if (typeof maybeError.code !== "string" || typeof maybeError.message !== "string") {
    return null;
  }
  return {
    code: maybeError.code,
    message: maybeError.message,
    detail: maybeError.detail,
    recoverable: Boolean(maybeError.recoverable),
    suggestedAction: maybeError.suggestedAction,
  };
}

export function ProjectCenterPage() {
  const [showNew, setShowNew] = useState(false);
  const [name, setName] = useState("");
  const [author, setAuthor] = useState("");
  const [genre, setGenre] = useState("玄幻");
  const [targetWords, setTargetWords] = useState(300000);
  const [saveDirectory, setSaveDirectory] = useState("");
  const [openDirectory, setOpenDirectory] = useState("");
  const [creating, setCreating] = useState(false);
  const [opening, setOpening] = useState(false);
  const [showOpen, setShowOpen] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [recentProjects, setRecentProjects] = useState<Array<{ path: string; name: string; openedAt: string }>>([]);

  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const setCurrentProject = useProjectStore((s) => s.setCurrentProject);

  useEffect(() => {
    void (async () => {
      try {
        const items = await listRecentProjects();
        setActionError(null);
        setRecentProjects(items.map((item) => ({
          path: item.projectPath,
          name: item.projectPath.split(/[\\/]/).pop() || item.projectPath,
          openedAt: item.openedAt,
        })));
      } catch {
        setRecentProjects([]);
      }
    })();
  }, []);

  async function handleCreate() {
    const pathError = validateWindowsDirectoryPath(saveDirectory);
    if (pathError) {
      setActionError(pathError);
      return;
    }
    if (!name.trim()) {
      setActionError("作品名称不能为空");
      return;
    }
    setCreating(true);
    setActionError(null);
    try {
      const validated = await validateProjectName({ name: name.trim() });
      const result = await createProject({
        name: validated.normalizedName,
        author: author.trim() || undefined,
        genre,
        targetWords,
        saveDirectory: saveDirectory.trim()
      });
      setCurrentProject(result.projectRoot, result.project);
      setShowNew(false);
      setActiveRoute("dashboard");
      setSaveDirectory("");
      setName("");
      setAuthor("");
    } catch (err) {
      const appError = asAppError(err);
      if (appError?.code === "PROJECT_PATH_EXISTS" && appError.detail) {
        try {
          const existing = await openProject(appError.detail);
          setCurrentProject(existing.projectRoot, existing.project);
          setShowNew(false);
          setActiveRoute("dashboard");
          setActionError(null);
          return;
        } catch {
          // Keep original create error when existing directory is not a valid project.
        }
      }
      console.error("Create project failed", err);
      setActionError(getErrorMessage(err, "项目创建失败，请检查保存目录和权限"));
    } finally {
      setCreating(false);
    }
  }

  async function handleOpenExisting(projectPath?: string) {
    // If a specific path is given (from recent projects list), open it directly
    if (projectPath) {
      try {
        const result = await openProject(projectPath);
        setCurrentProject(result.projectRoot, result.project);
        setActiveRoute("dashboard");
        setActionError(null);
      } catch (err) {
        setActionError(getErrorMessage(err, "打开项目失败，请确认路径有效"));
      }
      return;
    }
    // No path: use explicit absolute-directory input for reliable Tauri path handling.
    setOpenDirectory("");
    setShowOpen(true);
  }

  async function handleOpenByDirectoryPath() {
    const pathError = validateWindowsDirectoryPath(openDirectory);
    if (pathError) {
      setActionError(pathError);
      return;
    }
    setOpening(true);
    setActionError(null);
    try {
      const result = await openProject(openDirectory.trim());
      setCurrentProject(result.projectRoot, result.project);
      setActiveRoute("dashboard");
      setShowOpen(false);
      setOpenDirectory("");
    } catch (err) {
      setActionError(getErrorMessage(err, "打开项目失败，请确认所选目录是有效的 NovelForge 项目"));
    } finally {
      setOpening(false);
    }
  }

  async function handleClearProject() {
    try {
      await clearRecentProjects();
      setRecentProjects([]);
      setActionError(null);
    } catch (err) {
      setActionError(getErrorMessage(err, "清除最近项目失败"));
    }
  }

  return (
    <div className="min-h-screen bg-surface-900 flex items-center justify-center p-6">
      <div className="w-full max-w-4xl">
        <div className="text-center mb-10">
          <h1 className="text-3xl font-bold text-primary mb-2">NovelForge</h1>
          <p className="text-surface-400 text-sm">本地优先 AI 长篇小说创作平台</p>
        </div>

        <div className="grid md:grid-cols-2 gap-6">
          <Card padding="lg" className="space-y-4">
            <h2 className="text-lg font-semibold text-surface-100">开始创作</h2>
            <div className="space-y-3">
              <Button variant="primary" className="w-full justify-center" onClick={() => setShowNew(true)}>
                新建作品工程
              </Button>
              <Button variant="secondary" className="w-full justify-center" onClick={() => void handleOpenExisting()}>
                打开本地项目
              </Button>
            </div>
          </Card>

          <Card padding="lg">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-surface-100">最近项目</h2>
              {recentProjects.length > 0 && (
                <button onClick={() => void handleClearProject()} className="text-xs text-surface-400 hover:text-error transition-colors">
                  清除
                </button>
              )}
            </div>
            {recentProjects.length === 0 ? (
              <p className="text-sm text-surface-500 text-center py-8">暂无最近项目</p>
            ) : (
              <div className="space-y-2">
                {recentProjects.map((p) => (
                  <div key={p.path} className="w-full p-3 bg-surface-700 rounded-lg space-y-2">
                    <button
                      onClick={() => void handleOpenExisting(p.path)}
                      className="w-full text-left hover:bg-surface-600/40 rounded transition-colors"
                    >
                      <div className="text-sm text-surface-100">{p.name}</div>
                      <div className="text-xs text-surface-400 mt-1">打开于 {p.openedAt}</div>
                    </button>
                  </div>
                ))}
              </div>
            )}
          </Card>
        </div>

        <p className="mt-8 text-center text-xs text-surface-500">
          让灵感成为工程，让故事稳定完稿。
        </p>
        {actionError && (
          <p className="mt-3 text-center text-xs text-error">{actionError}</p>
        )}
      </div>

      <Modal open={showNew} onClose={() => setShowNew(false)} title="新建作品工程" width="md">
        <div className="space-y-4">
          <Input label="作品名称 *" value={name} onChange={(e) => setName(e.target.value)} placeholder="输入作品名称" />
          <Input label="作者名" value={author} onChange={(e) => setAuthor(e.target.value)} placeholder="可选" />
          <Input
            label="保存目录 *"
            value={saveDirectory}
            onChange={(e) => setSaveDirectory(e.target.value)}
            placeholder="例如：D:\\NovelProjects"
          />
          <Select label="类型" value={genre} onChange={(e) => setGenre(e.target.value)} options={GENRES.map((g) => ({ value: g, label: g }))} />
          <Input label="目标字数" type="number" value={targetWords} onChange={(e) => setTargetWords(Number(e.target.value))} min={10000} step={10000} />
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleCreate()} disabled={!name.trim() || creating}>
              {creating ? "创建中..." : "创建项目"}
            </Button>
          </div>
        </div>
      </Modal>

      <Modal open={showOpen} onClose={() => setShowOpen(false)} title="打开本地项目" width="md">
        <div className="space-y-4">
          <Input
            label="项目目录 *"
            value={openDirectory}
            onChange={(e) => setOpenDirectory(e.target.value)}
            placeholder="例如：F:\\NovelProjects\\夜潮计划"
          />
          <p className="text-xs text-surface-500">
            请选择包含 <code>project.json</code> 与 <code>database\\project.sqlite</code> 的项目根目录。
          </p>
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowOpen(false)}>取消</Button>
            <Button variant="primary" onClick={() => void handleOpenByDirectoryPath()} disabled={!openDirectory.trim() || opening}>
              {opening ? "打开中..." : "打开项目"}
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
