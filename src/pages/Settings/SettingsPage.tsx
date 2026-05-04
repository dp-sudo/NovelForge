import { useEffect, useState } from "react";
import { Card } from "../../components/cards/Card";
import { Input } from "../../components/forms/Input";
import { Select } from "../../components/forms/Select";
import { ApiKeyInput } from "../../components/forms/ApiKeyInput";
import { Button } from "../../components/ui/Button";
import {
  listProviders,
  saveProvider,
  deleteProvider,
  testProviderConnection,
  refreshProviderModels,
  getProviderModels,
  checkRemoteRegistry,
  applyRegistryUpdate,
  listTaskRoutes,
  saveTaskRoute,
  deleteTaskRoute,
  loadEditorSettings,
  saveEditorSettings,
  saveWritingStyle,
  getWritingStyle,
  initProjectRepository,
  getProjectRepositoryStatus,
  commitProjectSnapshot,
  listProjectHistory,
  getLicenseStatus,
  activateLicense,
  checkAppUpdate,
  installAppUpdate,
  type LlmProviderConfig,
  type EditorSettingsData,
  type GitRepositoryStatus,
  type GitCommitRecord,
  type LicenseStatus,
  type AppUpdateInfo,
  type TaskRoute,
} from "../../api/settingsApi";
import { checkProjectIntegrity, type IntegrityReport } from "../../api/integrityApi.js";
import { createBackup, listBackups, restoreBackup, type BackupResult } from "../../api/backupApi.js";
import { useProjectStore } from "../../stores/projectStore";
import {
  VENDOR_PRESETS,
  defaultWritingStyle,
  type WritingStyle,
  type VendorInfo,
  type ModelRecord,
  type CapabilityReport,
} from "../../types/ai";
import { SkillsManager } from "../../components/skills/SkillsManager.js";
import { TASK_ROUTE_OPTIONS } from "../../utils/taskRouting.js";

type TabKey = "model" | "routing" | "skills" | "editor" | "writing" | "backup" | "about";

interface VendorFormState {
  config: LlmProviderConfig;
  apiKeyInput: string;
  clearApiKeyRequested: boolean;
  betaHeadersInput: string;
  customHeadersInput: string;
  saving: boolean;
  testing: boolean;
  refreshing: boolean;
  models: ModelRecord[];
  capabilities: CapabilityReport | null;
  refreshResult: string | null;
  testResult: string | null;
  validationError: string | null;
  expanded: boolean;
}

interface TaskRouteFormState {
  route: TaskRoute;
  saving: boolean;
  deleting: boolean;
  error: string | null;
}

interface SliderControlProps {
  label: string;
  value: number;
  minLabel: string;
  maxLabel: string;
  min?: number;
  max?: number;
  onChange: (val: number) => void;
}

function SliderControl({
  label,
  value,
  minLabel,
  maxLabel,
  min = 1,
  max = 7,
  onChange,
}: SliderControlProps) {
  return (
    <div>
      <label className="text-sm text-surface-200 block mb-2">{label}</label>
      <div className="flex items-center gap-3">
        <span className="text-xs text-surface-400 w-16 text-right shrink-0">{minLabel}</span>
        <div className="flex gap-1.5 flex-1 justify-center">
          {Array.from({ length: max - min + 1 }, (_, i) => min + i).map((n) => (
            <button
              key={n}
              type="button"
              onClick={() => onChange(n)}
              className={`w-8 h-8 rounded-full text-xs font-medium transition-colors ${
                n === value
                  ? "bg-primary text-white"
                  : n < value
                    ? "bg-primary/20 text-primary border border-primary/30"
                    : "bg-surface-800 text-surface-500 border border-surface-600"
              }`}
            >
              {n}
            </button>
          ))}
        </div>
        <span className="text-xs text-surface-400 w-16 shrink-0">{maxLabel}</span>
      </div>
    </div>
  );
}

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState<TabKey>("model");
  const [vendors, setVendors] = useState<Record<string, VendorFormState>>({});
  const [configuredProviderIds, setConfiguredProviderIds] = useState<string[]>([]);
  const [taskRoutes, setTaskRoutes] = useState<Record<string, TaskRouteFormState>>({});
  const [taskRouteMessage, setTaskRouteMessage] = useState<string | null>(null);
  const [taskRoutesLoading, setTaskRoutesLoading] = useState(true);
  const [loading, setLoading] = useState(true);

  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const [backupList, setBackupList] = useState<BackupResult[]>([]);
  const [backupCreating, setBackupCreating] = useState(false);
  const [backupRestoring, setBackupRestoring] = useState(false);
  const [backupMessage, setBackupMessage] = useState<string | null>(null);
  const [integrityChecking, setIntegrityChecking] = useState(false);
  const [integrityReport, setIntegrityReport] = useState<IntegrityReport | null>(null);
  const [gitStatus, setGitStatus] = useState<GitRepositoryStatus | null>(null);
  const [gitHistory, setGitHistory] = useState<GitCommitRecord[]>([]);
  const [gitBusy, setGitBusy] = useState(false);
  const [gitMessage, setGitMessage] = useState<string | null>(null);
  const [snapshotMessage, setSnapshotMessage] = useState("");
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus | null>(null);
  const [licenseKeyInput, setLicenseKeyInput] = useState("");
  const [licenseBusy, setLicenseBusy] = useState(false);
  const [licenseMessage, setLicenseMessage] = useState<string | null>(null);
  const [updateInfo, setUpdateInfo] = useState<AppUpdateInfo | null>(null);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [updateMessage, setUpdateMessage] = useState<string | null>(null);
  const [registryUrl, setRegistryUrl] = useState("https://updates.novelforge.app/llm-model-registry.json");
  const [registryStatus, setRegistryStatus] = useState<string | null>(null);
  const [registryChecking, setRegistryChecking] = useState(false);
  const [registryApplying, setRegistryApplying] = useState(false);
  const [registryUpdateAvailable, setRegistryUpdateAvailable] = useState(false);

  async function handleCreateBackup() {
    if (!projectRoot) { setBackupMessage("请先打开项目"); return; }
    setBackupCreating(true); setBackupMessage(null);
    try {
      const result = await createBackup(projectRoot);
      setBackupMessage(`备份成功：${result.filePath} (${(result.fileSize / 1024).toFixed(0)} KB)`);
      const list = await listBackups(projectRoot);
      setBackupList(list);
    } catch (e: unknown) {
      setBackupMessage(`备份失败：${typeof e === 'object' && e && 'message' in e ? String((e as {message:string}).message) : String(e)}`);
    } finally { setBackupCreating(false); }
  }

  async function handleRestoreBackup(backupPath: string) {
    if (!projectRoot) return;
    if (!window.confirm("恢复备份将覆盖当前项目数据，确定继续？")) return;
    setBackupRestoring(true); setBackupMessage(null);
    try {
      const result = await restoreBackup(projectRoot, backupPath);
      setBackupMessage(`恢复成功：${result.filesRestored} 个文件`);
    } catch (e: unknown) {
      setBackupMessage(`恢复失败：${typeof e === 'object' && e && 'message' in e ? String((e as {message:string}).message) : String(e)}`);
    } finally { setBackupRestoring(false); }
  }

  async function handleCheckIntegrity() {
    if (!projectRoot) {
      setBackupMessage("请先打开项目");
      return;
    }
    setIntegrityChecking(true);
    setBackupMessage(null);
    try {
      const report = await checkProjectIntegrity(projectRoot);
      setIntegrityReport(report);
      if (report.status === "healthy") {
        setBackupMessage("完整性检查通过：项目健康");
      } else {
        setBackupMessage(`完整性检查完成：发现 ${report.issues.length} 个问题`);
      }
    } catch (e: unknown) {
      setBackupMessage(`完整性检查失败：${typeof e === "object" && e && "message" in e ? String((e as {message:string}).message) : String(e)}`);
    } finally {
      setIntegrityChecking(false);
    }
  }

  useEffect(() => {
    if (projectRoot) {
      void listBackups(projectRoot).then(setBackupList).catch(() => {});
      void getProjectRepositoryStatus(projectRoot)
        .then((status) => setGitStatus(status))
        .catch(() => setGitStatus(null));
      void listProjectHistory(projectRoot, 10)
        .then((rows) => setGitHistory(rows))
        .catch(() => setGitHistory([]));
    } else {
      setIntegrityReport(null);
      setGitStatus(null);
      setGitHistory([]);
    }
  }, [projectRoot]);

  useEffect(() => {
    let canceled = false;
    if (!projectRoot) {
      setWritingStyle(defaultWritingStyle());
      setWritingStyleLoaded(false);
      setWritingStyleMessage(null);
      return () => {
        canceled = true;
      };
    }

    setWritingStyleLoaded(false);
    setWritingStyleMessage(null);
    setWritingStyleSaved(false);

    (async () => {
      try {
        const loaded = await getWritingStyle(projectRoot);
        if (!canceled) {
          setWritingStyle(loaded);
        }
      } catch (err: unknown) {
        if (canceled) return;
        setWritingStyle(defaultWritingStyle());
        setWritingStyleMessage(
          typeof err === "object" && err && "message" in err
            ? `加载写作风格失败：${String((err as { message: string }).message)}`
            : "加载写作风格失败"
        );
      } finally {
        if (!canceled) {
          setWritingStyleLoaded(true);
        }
      }
    })();

    return () => {
      canceled = true;
    };
  }, [projectRoot]);

  async function refreshGitData() {
    if (!projectRoot) {
      setGitMessage("请先打开项目");
      return;
    }
    const [status, history] = await Promise.all([
      getProjectRepositoryStatus(projectRoot),
      listProjectHistory(projectRoot, 10),
    ]);
    setGitStatus(status);
    setGitHistory(history);
  }

  async function handleInitGitRepo() {
    if (!projectRoot) {
      setGitMessage("请先打开项目");
      return;
    }
    setGitBusy(true);
    setGitMessage(null);
    try {
      const status = await initProjectRepository(projectRoot);
      setGitStatus(status);
      const history = await listProjectHistory(projectRoot, 10);
      setGitHistory(history);
      setGitMessage("Git 仓库初始化完成");
    } catch (err: unknown) {
      setGitMessage(
        typeof err === "object" && err && "message" in err
          ? String((err as { message: string }).message)
          : "初始化 Git 仓库失败"
      );
    } finally {
      setGitBusy(false);
    }
  }

  async function handleCommitSnapshot() {
    if (!projectRoot) {
      setGitMessage("请先打开项目");
      return;
    }
    setGitBusy(true);
    setGitMessage(null);
    try {
      const result = await commitProjectSnapshot(projectRoot, snapshotMessage.trim() || undefined);
      if (result.noChanges) {
        setGitMessage("没有检测到文件变更，未创建新提交");
      } else if (result.commit) {
        setGitMessage(`已创建快照提交：${result.commit.commitId.slice(0, 8)} ${result.commit.summary}`);
      } else {
        setGitMessage("快照提交完成");
      }
      setSnapshotMessage("");
      await refreshGitData();
    } catch (err: unknown) {
      setGitMessage(
        typeof err === "object" && err && "message" in err
          ? String((err as { message: string }).message)
          : "提交快照失败"
      );
    } finally {
      setGitBusy(false);
    }
  }

  function updateWritingStyle(patch: Partial<WritingStyle>) {
    setWritingStyle((prev) => ({ ...prev, ...patch }));
    setWritingStyleSaved(false);
  }

  async function handleSaveWritingStyle() {
    if (!projectRoot) {
      setWritingStyleMessage("请先打开项目以设置写作风格");
      return;
    }

    setWritingStyleSaving(true);
    setWritingStyleMessage(null);
    try {
      await saveWritingStyle(projectRoot, writingStyle);
      setWritingStyleSaved(true);
      setWritingStyleMessage("写作风格已保存");
      setTimeout(() => setWritingStyleSaved(false), 2000);
    } catch (err: unknown) {
      setWritingStyleSaved(false);
      setWritingStyleMessage(
        typeof err === "object" && err && "message" in err
          ? `保存写作风格失败：${String((err as { message: string }).message)}`
          : "保存写作风格失败"
      );
    } finally {
      setWritingStyleSaving(false);
    }
  }

  async function handleActivateLicense() {
    const key = licenseKeyInput.trim();
    if (!key) {
      setLicenseMessage("请输入授权码");
      return;
    }
    setLicenseBusy(true);
    setLicenseMessage(null);
    try {
      const status = await activateLicense(key);
      setLicenseStatus(status);
      setLicenseKeyInput("");
      setLicenseMessage(`授权成功，当前等级：${status.tier}`);
    } catch (err: unknown) {
      setLicenseMessage(
        typeof err === "object" && err && "message" in err
          ? String((err as { message: string }).message)
          : "授权失败"
      );
    } finally {
      setLicenseBusy(false);
    }
  }

  async function handleCheckUpdate() {
    setUpdateBusy(true);
    setUpdateMessage(null);
    try {
      const info = await checkAppUpdate();
      setUpdateInfo(info);
      if (info.available) {
        setUpdateMessage(`发现可用更新：${info.currentVersion} -> ${info.targetVersion || "unknown"}`);
      } else {
        setUpdateMessage(`当前已是最新版本：${info.currentVersion}`);
      }
    } catch (err: unknown) {
      setUpdateMessage(
        typeof err === "object" && err && "message" in err
          ? String((err as { message: string }).message)
          : "检查更新失败"
      );
    } finally {
      setUpdateBusy(false);
    }
  }

  async function handleInstallUpdate() {
    setUpdateBusy(true);
    setUpdateMessage(null);
    try {
      const info = await installAppUpdate();
      setUpdateInfo(info);
      if (info.available) {
        setUpdateMessage(`更新包已安装：${info.currentVersion} -> ${info.targetVersion || "unknown"}，请重启应用生效`);
      } else {
        setUpdateMessage(`没有可安装的更新，当前版本 ${info.currentVersion}`);
      }
    } catch (err: unknown) {
      setUpdateMessage(
        typeof err === "object" && err && "message" in err
          ? String((err as { message: string }).message)
          : "安装更新失败"
      );
    } finally {
      setUpdateBusy(false);
    }
  }

  const [editor, setEditor] = useState<EditorSettingsData>({
    fontSize: 16, lineHeight: 1.75, autosaveInterval: 5, narrativePov: "third_limited",
  });
  const [editorSaved, setEditorSaved] = useState(false);
  const [writingStyle, setWritingStyle] = useState<WritingStyle>(defaultWritingStyle());
  const [writingStyleLoaded, setWritingStyleLoaded] = useState(false);
  const [writingStyleSaving, setWritingStyleSaving] = useState(false);
  const [writingStyleSaved, setWritingStyleSaved] = useState(false);
  const [writingStyleMessage, setWritingStyleMessage] = useState<string | null>(null);

  function mapToInput(value?: Record<string, string>): string {
    if (!value) return "";
    return Object.entries(value).map(([k, v]) => `${k}:${v}`).join("\n");
  }

  function parseHeadersInput(value: string): Record<string, string> | undefined {
    const pairs = value
      .split(/\r?\n|,/)
      .map((item) => item.trim())
      .filter(Boolean)
      .map((item) => {
        const idx = item.indexOf(":");
        if (idx <= 0) return null;
        const key = item.slice(0, idx).trim();
        const val = item.slice(idx + 1).trim();
        if (!key || !val) return null;
        return [key, val] as const;
      })
      .filter((item): item is readonly [string, string] => item !== null);

    if (pairs.length === 0) return undefined;
    return Object.fromEntries(pairs);
  }

  function validateProviderInput(config: LlmProviderConfig): string | null {
    if (!config.baseUrl.trim()) return "Base URL 不能为空";
    try {
      const parsed = new URL(config.baseUrl);
      if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
        return "Base URL 必须是 http:// 或 https://";
      }
    } catch {
      return "Base URL 格式不合法";
    }

    if (config.id === "custom") {
      if (!config.displayName.trim()) return "自定义 Provider 名称不能为空";
      if (!config.protocol) return "请选择自定义 Provider 协议";
    }

    return null;
  }

  function normalizeRoute(route: TaskRoute): TaskRoute {
    return {
      ...route,
      fallbackProviderId: route.fallbackProviderId || "",
      fallbackModelId: route.fallbackModelId || "",
      maxRetries: route.maxRetries || 1,
    };
  }

  function pickPrimaryRouteSeed(configs: LlmProviderConfig[]): { providerId: string; modelId: string } | null {
    for (const preset of VENDOR_PRESETS) {
      const existing = configs.find((config) => config.id === preset.id);
      const modelId = existing?.defaultModel?.trim() || "";
      if (existing && modelId) {
        return { providerId: existing.id, modelId };
      }
    }
    for (const config of configs) {
      const modelId = config.defaultModel?.trim() || "";
      if (modelId) {
        return { providerId: config.id, modelId };
      }
    }
    return null;
  }

  function collectProviderModelChoices(providerId: string): string[] {
    const provider = vendors[providerId];
    if (!provider) return [];
    const values = [
      provider.config.defaultModel?.trim() || "",
      ...provider.models.map((model) => model.modelName.trim()),
    ].filter(Boolean);
    return Array.from(new Set(values));
  }

  useEffect(() => {
    (async () => {
      setLoading(true);
      setTaskRoutesLoading(true);
      try {
        const [configs, routes] = await Promise.all([
          listProviders(),
          listTaskRoutes(),
        ]);
        const configuredIds = configs.map((config) => config.id);
        setConfiguredProviderIds(configuredIds);
        const modelsByProvider = new Map<string, ModelRecord[]>(
          await Promise.all(
            configuredIds.map(async (providerId) => [
              providerId,
              await getProviderModels(providerId).catch(() => [] as ModelRecord[]),
            ] as const)
          )
        );
        const primaryRouteSeed = pickPrimaryRouteSeed(configs);
        const currentLicense = await getLicenseStatus().catch(() => null);
        setLicenseStatus(currentLicense);

        const map: Record<string, VendorFormState> = {};
        for (const preset of VENDOR_PRESETS) {
          const existing = configs.find((c) => c.id === preset.id);
          map[preset.id] = {
            config: {
              id: preset.id,
              displayName: existing?.displayName || preset.displayName,
              vendor: preset.vendor,
              protocol: existing?.protocol || preset.defaultProtocol,
              baseUrl: existing?.baseUrl || preset.defaultBaseUrl,
              endpointPath: existing?.endpointPath,
              defaultModel: existing?.defaultModel || preset.defaultModel,
              apiKey: existing?.apiKey,
              authMode: existing?.authMode || (preset.id === "custom" ? "bearer" : "bearer"),
              authHeaderName: existing?.authHeaderName,
              anthropicVersion: existing?.anthropicVersion,
              betaHeaders: existing?.betaHeaders,
              customHeaders: existing?.customHeaders,
              timeoutMs: existing?.timeoutMs || 120000,
              connectTimeoutMs: existing?.connectTimeoutMs || 15000,
              maxRetries: existing?.maxRetries || 2,
              modelRefreshMode: existing?.modelRefreshMode || "registry",
              modelsPath: existing?.modelsPath,
              lastModelRefreshAt: existing?.lastModelRefreshAt,
            },
            apiKeyInput: "",
            clearApiKeyRequested: false,
            betaHeadersInput: mapToInput(existing?.betaHeaders),
            customHeadersInput: mapToInput(existing?.customHeaders),
            saving: false,
            testing: false,
            refreshing: false,
            models: modelsByProvider.get(preset.id) || [],
            capabilities: null,
            refreshResult: null,
            testResult: null,
            validationError: null,
            expanded: false,
          };
        }

        const routeMap: Record<string, TaskRouteFormState> = {};
        for (const task of TASK_ROUTE_OPTIONS) {
          const existingRoute = routes.find(
            (route) => route.taskType.trim() === task.value
          );
          routeMap[task.value] = {
            route: existingRoute
              ? normalizeRoute({ ...existingRoute, taskType: task.value })
              : {
                  id: "",
                  taskType: task.value,
                  providerId: primaryRouteSeed?.providerId || "",
                  modelId: primaryRouteSeed?.modelId || "",
                  fallbackProviderId: "",
                  fallbackModelId: "",
                  maxRetries: 1,
                },
            saving: false,
            deleting: false,
            error: null,
          };
        }

        setVendors(map);
        setTaskRoutes(routeMap);
      } finally {
        setLoading(false);
        setTaskRoutesLoading(false);
      }
    })();
    (async () => {
      setEditor(await loadEditorSettings());
    })();
  }, []);

  function updateVendor(id: string, patch: Partial<LlmProviderConfig>) {
    setVendors((prev) => ({
      ...prev,
      [id]: { ...prev[id], config: { ...prev[id].config, ...patch }, validationError: null },
    }));
  }

  async function handleSave(preset: VendorInfo) {
    const v = vendors[preset.id];
    if (!v) return;

    const normalizedConfig: LlmProviderConfig = {
      ...v.config,
      displayName: v.config.displayName.trim(),
      baseUrl: v.config.baseUrl.trim(),
      endpointPath: v.config.endpointPath?.trim() || undefined,
      defaultModel: v.config.defaultModel?.trim() || undefined,
      authHeaderName: v.config.authHeaderName?.trim() || undefined,
      modelsPath: v.config.modelsPath?.trim() || undefined,
      betaHeaders: parseHeadersInput(v.betaHeadersInput),
      customHeaders: parseHeadersInput(v.customHeadersInput),
    };

    const validationError = validateProviderInput(normalizedConfig);
    if (validationError) {
      setVendors((prev) => ({
        ...prev,
        [preset.id]: { ...prev[preset.id], validationError },
      }));
      return;
    }

    setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], saving: true, validationError: null } }));
    try {
      const apiKeyPayload = resolveApiKeyPayload(v);
      const saved = await saveProvider(normalizedConfig, apiKeyPayload);
      const models = await getProviderModels(preset.id).catch(() => [] as ModelRecord[]);
      setVendors((prev) => ({
        ...prev,
        [preset.id]: {
          ...prev[preset.id],
          config: saved,
          apiKeyInput: "",
          clearApiKeyRequested: false,
          betaHeadersInput: mapToInput(saved.betaHeaders),
          customHeadersInput: mapToInput(saved.customHeaders),
          models,
          saving: false,
          validationError: null,
        },
      }));
      setConfiguredProviderIds((prev) => (prev.includes(saved.id) ? prev : [...prev, saved.id]));
      const defaultModelId = saved.defaultModel?.trim() || "";
      if (defaultModelId) {
        setTaskRoutes((prev) => {
          const next: Record<string, TaskRouteFormState> = {};
          for (const [taskType, state] of Object.entries(prev)) {
            if (state.route.providerId.trim() || state.route.modelId.trim()) {
              next[taskType] = state;
            } else {
              next[taskType] = {
                ...state,
                route: {
                  ...state.route,
                  providerId: saved.id,
                  modelId: defaultModelId,
                },
              };
            }
          }
          return next;
        });
      }
    } catch (err: unknown) {
      const msg = typeof err === "object" && err && "message" in err
        ? String((err as { message: string }).message)
        : "保存失败";
      setVendors((prev) => ({
        ...prev,
        [preset.id]: { ...prev[preset.id], saving: false, validationError: msg },
      }));
    }
  }

  async function handleTest(preset: VendorInfo) {
    const v = vendors[preset.id];
    if (!v) return;
    setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], testing: true, testResult: null, validationError: null } }));
    try {
      // Save the provider config first so the backend can find it when testing
      if (v.apiKeyInput || !v.config.apiKey || v.clearApiKeyRequested) {
        // Only save if there's new input or no existing key (otherwise skip to avoid unnecessary DB write)
        const normalizedConfig: LlmProviderConfig = {
          ...v.config,
          displayName: v.config.displayName.trim(),
          baseUrl: v.config.baseUrl.trim(),
          endpointPath: v.config.endpointPath?.trim() || undefined,
          defaultModel: v.config.defaultModel?.trim() || undefined,
          authHeaderName: v.config.authHeaderName?.trim() || undefined,
          modelsPath: v.config.modelsPath?.trim() || undefined,
        };
        await saveProvider(normalizedConfig, resolveApiKeyPayload(v));
      }
      const result = await testProviderConnection(preset.id);
      setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], testing: false, testResult: result } }));
    } catch (err: unknown) {
      const msg = typeof err === "object" && err && "message" in err
        ? String((err as { message: string }).message)
        : "连接失败";
      setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], testing: false, testResult: msg } }));
    }
  }

  async function handleRefresh(preset: VendorInfo) {
    setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], refreshing: true, refreshResult: null, models: [], capabilities: null } }));
    try {
      const result = await refreshProviderModels(preset.id);
      const [models] = await Promise.all([
        getProviderModels(preset.id).catch(() => []),
      ]);
      setVendors((prev) => ({
        ...prev,
        [preset.id]: {
          ...prev[preset.id],
          refreshing: false,
          models,
          capabilities: result.capabilities,
          refreshResult: `✓ 新增 ${result.added} / 更新 ${result.updated} / 弃用 ${result.removed}`,
        },
      }));
    } catch (err: unknown) {
      const msg = typeof err === "object" && err && "message" in err
        ? String((err as { message: string }).message)
        : "刷新失败";
      setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], refreshing: false, refreshResult: `✗ ${msg}` } }));
    }
  }

  async function handleDelete(preset: VendorInfo) {
    await deleteProvider(preset.id);
    setConfiguredProviderIds((prev) => prev.filter((providerId) => providerId !== preset.id));
    setVendors((prev) => ({
      ...prev,
      [preset.id]: {
        ...prev[preset.id],
        config: {
          ...prev[preset.id].config,
          displayName: preset.displayName,
          protocol: preset.defaultProtocol,
          baseUrl: preset.defaultBaseUrl,
          endpointPath: undefined,
          defaultModel: preset.defaultModel,
          apiKey: undefined,
          authMode: "bearer",
          authHeaderName: undefined,
          anthropicVersion: undefined,
          betaHeaders: undefined,
          customHeaders: undefined,
          modelsPath: undefined,
        },
        apiKeyInput: "",
        clearApiKeyRequested: false,
        betaHeadersInput: "",
        customHeadersInput: "",
        validationError: null,
      },
    }));
    setTaskRoutes((prev) => {
      const next: Record<string, TaskRouteFormState> = {};
      for (const [taskType, state] of Object.entries(prev)) {
        if (state.route.providerId !== preset.id && state.route.fallbackProviderId !== preset.id) {
          next[taskType] = state;
          continue;
        }
        next[taskType] = {
          ...state,
          route: {
            ...state.route,
            providerId: state.route.providerId === preset.id ? "" : state.route.providerId,
            modelId: state.route.providerId === preset.id ? "" : state.route.modelId,
            fallbackProviderId: state.route.fallbackProviderId === preset.id ? "" : state.route.fallbackProviderId,
            fallbackModelId: state.route.fallbackProviderId === preset.id ? "" : state.route.fallbackModelId,
          },
        };
      }
      return next;
    });
  }

  function resolveApiKeyPayload(vendor: VendorFormState): string | undefined {
    if (vendor.clearApiKeyRequested) {
      return "";
    }
    const trimmed = vendor.apiKeyInput.trim();
    return trimmed ? trimmed : undefined;
  }

  async function handleCheckRegistry() {
    if (!registryUrl.trim()) { setRegistryStatus("✗ 注册表 URL 不能为空"); return; }
    setRegistryChecking(true);
    setRegistryStatus(null);
    setRegistryUpdateAvailable(false);
    try {
      const result = await checkRemoteRegistry(registryUrl.trim());
      if (result.hasUpdate) {
        setRegistryStatus(`✓ 发现新版本: ${result.remoteVersion} (当前: ${result.currentVersion})`);
        setRegistryUpdateAvailable(true);
      } else {
        setRegistryStatus(`✓ 已是最新版本: ${result.currentVersion}`);
        setRegistryUpdateAvailable(false);
      }
    } catch (err: unknown) {
      const msg = typeof err === "object" && err && "message" in err
        ? String((err as { message: string }).message)
        : "检查失败";
      setRegistryStatus(`✗ ${msg}`);
      setRegistryUpdateAvailable(false);
    } finally {
      setRegistryChecking(false);
    }
  }

  async function handleApplyRegistry() {
    if (!registryUrl.trim() || !registryUpdateAvailable) return;
    setRegistryApplying(true);
    try {
      const result = await applyRegistryUpdate(registryUrl.trim());
      setRegistryStatus(`✓ 更新已应用: 新增 ${result.added} / 更新 ${result.updated} (版本: ${result.version})`);
      setRegistryUpdateAvailable(false);
    } catch (err: unknown) {
      const msg = typeof err === "object" && err && "message" in err
        ? String((err as { message: string }).message)
        : "应用失败";
      setRegistryStatus(`✗ ${msg}`);
    } finally {
      setRegistryApplying(false);
    }
  }

  function updateTaskRoute(taskType: string, patch: Partial<TaskRoute>) {
    setTaskRoutes((prev) => ({
      ...prev,
      [taskType]: {
        ...prev[taskType],
        route: { ...prev[taskType].route, ...patch },
        error: null,
      },
    }));
    setTaskRouteMessage(null);
  }

  function handleTaskRouteProviderChange(taskType: string, providerId: string) {
    const nextModelId = collectProviderModelChoices(providerId)[0] || "";
    updateTaskRoute(taskType, {
      providerId,
      modelId: nextModelId,
    });
  }

  function handleTaskRouteFallbackProviderChange(taskType: string, fallbackProviderId: string) {
    const fallbackModelId = fallbackProviderId
      ? collectProviderModelChoices(fallbackProviderId)[0] || ""
      : "";
    updateTaskRoute(taskType, {
      fallbackProviderId,
      fallbackModelId,
    });
  }

  async function handleSaveTaskRoute(taskType: string) {
    const state = taskRoutes[taskType];
    if (!state) return;
    const route = state.route;

    if (!route.providerId.trim()) {
      setTaskRoutes((prev) => ({
        ...prev,
        [taskType]: { ...prev[taskType], error: "请选择 Provider" },
      }));
      return;
    }

    if (!route.modelId.trim()) {
      setTaskRoutes((prev) => ({
        ...prev,
        [taskType]: { ...prev[taskType], error: "请输入模型 ID" },
      }));
      return;
    }

    setTaskRoutes((prev) => ({
      ...prev,
      [taskType]: { ...prev[taskType], saving: true, error: null },
    }));

    try {
      const payload: TaskRoute = {
        ...route,
        id: route.id || "",
        fallbackProviderId: route.fallbackProviderId?.trim() || undefined,
        fallbackModelId: route.fallbackModelId?.trim() || undefined,
        maxRetries: Math.max(1, Number(route.maxRetries) || 1),
      };
      const saved = await saveTaskRoute(payload);
      setTaskRoutes((prev) => ({
        ...prev,
        [taskType]: {
          ...prev[taskType],
          route: normalizeRoute(saved),
          saving: false,
          error: null,
        },
      }));
      setTaskRouteMessage(`已保存任务路由：${taskType}`);
    } catch (err: unknown) {
      const msg = typeof err === "object" && err && "message" in err
        ? String((err as { message: string }).message)
        : "保存任务路由失败";
      setTaskRoutes((prev) => ({
        ...prev,
        [taskType]: { ...prev[taskType], saving: false, error: msg },
      }));
    }
  }

  async function handleDeleteTaskRoute(taskType: string) {
    const state = taskRoutes[taskType];
    if (!state || !state.route.id) return;
    setTaskRoutes((prev) => ({
      ...prev,
      [taskType]: { ...prev[taskType], deleting: true, error: null },
    }));

    try {
      await deleteTaskRoute(state.route.id);
      setTaskRoutes((prev) => ({
        ...prev,
        [taskType]: {
          ...prev[taskType],
          route: {
            id: "",
            taskType,
            providerId: "",
            modelId: "",
            fallbackProviderId: "",
            fallbackModelId: "",
            maxRetries: 1,
          },
          deleting: false,
          error: null,
        },
      }));
      setTaskRouteMessage(`已删除任务路由：${taskType}`);
    } catch (err: unknown) {
      const msg = typeof err === "object" && err && "message" in err
        ? String((err as { message: string }).message)
        : "删除任务路由失败";
      setTaskRoutes((prev) => ({
        ...prev,
        [taskType]: { ...prev[taskType], deleting: false, error: msg },
      }));
    }
  }

  function capabilityBadge(label: string) {
    return (
      <span key={label} className="inline-flex items-center px-2 py-0.5 text-xs rounded-full bg-primary/10 text-primary border border-primary/20">
        {label}
      </span>
    );
  }

  const tabs: { key: TabKey; label: string }[] = [
    { key: "model", label: "模型配置" },
    { key: "routing", label: "任务路由" },
    { key: "skills", label: "技能管理" },
    { key: "editor", label: "编辑器" },
    { key: "writing", label: "写作风格" },
    { key: "backup", label: "数据与备份" },
    { key: "about", label: "关于" },
  ];

  const configuredProviderIdSet = new Set(configuredProviderIds);
  const providerIdsForRouting = configuredProviderIds.length > 0
    ? VENDOR_PRESETS
      .map((preset) => preset.id)
      .filter((providerId) => configuredProviderIdSet.has(providerId))
    : VENDOR_PRESETS.map((preset) => preset.id);

  function toProviderLabel(providerId: string): string {
    const preset = VENDOR_PRESETS.find((item) => item.id === providerId);
    return vendors[providerId]?.config.displayName || preset?.displayName || providerId;
  }

  function buildRouteProviderOptions(currentProviderId: string): { value: string; label: string }[] {
    const ids = providerIdsForRouting.includes(currentProviderId) || !currentProviderId
      ? providerIdsForRouting
      : [...providerIdsForRouting, currentProviderId];
    return ids.map((providerId) => ({
      value: providerId,
      label: toProviderLabel(providerId),
    }));
  }

  function buildRouteModelOptions(providerId: string, currentModelId: string): { value: string; label: string }[] {
    const modelIds = collectProviderModelChoices(providerId);
    const merged = currentModelId && !modelIds.includes(currentModelId)
      ? [currentModelId, ...modelIds]
      : modelIds;
    return merged.map((modelId) => ({ value: modelId, label: modelId }));
  }

  return (
    <div className="max-w-4xl mx-auto">
      <h1 className="text-2xl font-bold text-surface-100 mb-6">设置</h1>

      <div className="flex gap-1 mb-6 p-1 bg-surface-800 rounded-lg border border-surface-700 w-fit">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`px-4 py-2 text-sm rounded-md transition-colors ${
              activeTab === tab.key ? "bg-primary text-white" : "text-surface-300 hover:text-surface-100"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {activeTab === "model" && (
        <div className="space-y-6">
          {loading ? (
            <p className="text-surface-400 text-sm">加载中...</p>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {VENDOR_PRESETS.map((preset) => {
                const v = vendors[preset.id];
                if (!v) return null;
                const { config } = v;
                return (
                  <Card key={preset.id} padding="md" className="space-y-3">
                    <div className="flex items-center justify-between">
                      <div>
                        <h3 className="text-sm font-semibold text-surface-100">{preset.displayName}</h3>
                        <p className="text-xs text-surface-400 mt-0.5">{config.defaultModel || "未配置"}</p>
                      </div>
                      <div className="flex gap-1 flex-wrap justify-end max-w-[140px]">
                        {preset.supports.map(capabilityBadge)}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`inline-block w-2 h-2 rounded-full ${config.apiKey ? "bg-success" : "bg-surface-500"}`} />
                      <span className="text-xs text-surface-400">{config.apiKey ? "API Key 已配置" : "未配置 API Key"}</span>
                    </div>
                    <button
                      onClick={() => setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], expanded: !prev[preset.id].expanded } }))}
                      className="text-xs text-primary hover:text-primary-light"
                    >
                      {v.expanded ? "收起配置 ▲" : "展开配置 ▼"}
                    </button>

                    {v.expanded && (
                      <div className="space-y-3 pt-2 border-t border-surface-700">
                        {preset.id === "custom" && (
                          <Input
                            label="Provider 名称"
                            value={config.displayName}
                            onChange={(e) => updateVendor(preset.id, { displayName: e.target.value })}
                            placeholder="My Custom Provider"
                          />
                        )}
                        {preset.id === "custom" && (
                          <Select
                            label="协议"
                            value={config.protocol}
                            onChange={(e) => updateVendor(preset.id, { protocol: e.target.value })}
                            options={[
                              { value: "custom_openai_compatible", label: "Custom OpenAI Compatible" },
                              { value: "custom_anthropic_compatible", label: "Custom Anthropic Compatible" },
                            ]}
                          />
                        )}
                        <Input label="Base URL" value={config.baseUrl} onChange={(e) => updateVendor(preset.id, { baseUrl: e.target.value })} placeholder={preset.defaultBaseUrl} />
                        <ApiKeyInput
                          label="API Key"
                          value={v.apiKeyInput}
                          onChange={(val) =>
                            setVendors((prev) => ({
                              ...prev,
                              [preset.id]: {
                                ...prev[preset.id],
                                apiKeyInput: val,
                                clearApiKeyRequested: false,
                              },
                            }))
                          }
                          onClearMasked={() =>
                            setVendors((prev) => ({
                              ...prev,
                              [preset.id]: {
                                ...prev[preset.id],
                                apiKeyInput: "",
                                clearApiKeyRequested: true,
                                validationError: null,
                              },
                            }))
                          }
                          maskedValue={config.apiKey && !v.apiKeyInput ? config.apiKey : undefined}
                        />
                        <Input label="默认模型" value={config.defaultModel || ""} onChange={(e) => updateVendor(preset.id, { defaultModel: e.target.value })} placeholder={preset.defaultModel} />
                        {v.validationError && (
                          <div className="px-3 py-2 rounded-lg text-sm bg-error/10 text-error border border-error/20">
                            {v.validationError}
                          </div>
                        )}
                        {v.testResult && (
                          <div className={`px-3 py-2 rounded-lg text-sm ${v.testResult.includes("成功") ? "bg-success/10 text-success border border-success/20" : "bg-error/10 text-error border border-error/20"}`}>
                            {v.testResult}
                          </div>
                        )}
                        {v.refreshResult && (
                          <div className={`px-3 py-2 rounded-lg text-sm ${v.refreshResult.startsWith("✓") ? "bg-success/10 text-success border border-success/20" : "bg-error/10 text-error border border-error/20"}`}>
                            {v.refreshResult}
                          </div>
                        )}
                        <div className="flex justify-between pt-2">
                          <Button variant="danger" size="sm" onClick={() => handleDelete(preset)}>重置</Button>
                          <div className="flex gap-2">
                            <Button variant="secondary" size="sm" onClick={() => handleRefresh(preset)} disabled={v.refreshing}>{v.refreshing ? "刷新中..." : "刷新模型"}</Button>
                            <Button variant="secondary" size="sm" onClick={() => handleTest(preset)} disabled={v.testing}>{v.testing ? "测试中..." : "测试连接"}</Button>
                            <Button variant="primary" size="sm" onClick={() => handleSave(preset)} disabled={v.saving}>{v.saving ? "保存中..." : "保存"}</Button>
                          </div>
                        </div>

                        {v.capabilities && (
                          <div className="pt-2 border-t border-surface-700">
                            <p className="text-xs font-medium text-surface-300 mb-2">能力检测报告</p>
                            <div className="flex flex-wrap gap-1.5">
                              {[
                                { key: "textResponse" as const, label: "文本生成" },
                                { key: "streaming" as const, label: "流式" },
                                { key: "jsonObject" as const, label: "JSON Object" },
                                { key: "tools" as const, label: "Tools" },
                                { key: "thinking" as const, label: "Thinking" },
                              ].map(({ key, label }) => (
                                <span
                                  key={key}
                                  className={`inline-flex items-center px-2 py-0.5 text-xs rounded-full border ${
                                    v.capabilities![key]
                                      ? "bg-success/10 text-success border-success/30"
                                      : "bg-surface-800 text-surface-500 border-surface-700"
                                  }`}
                                >
                                  {label} {v.capabilities![key] ? "✓" : "—"}
                                </span>
                              ))}
                            </div>
                          </div>
                        )}

                        {v.models.length > 0 && (
                          <div className="pt-2 border-t border-surface-700">
                            <p className="text-xs font-medium text-surface-300 mb-2">模型列表 ({v.models.length})</p>
                            <div className="max-h-40 overflow-y-auto space-y-1">
                              {v.models.map((m) => (
                                <div key={m.id} className="flex items-center justify-between px-2 py-1 rounded bg-surface-800/60 text-xs">
                                  <span className="text-surface-200 font-mono truncate">{m.modelName}</span>
                                  <div className="flex gap-1 shrink-0 ml-2">
                                    {m.supportsStreaming && <span className="text-success">S</span>}
                                    {m.supportsTools && <span className="text-info">T</span>}
                                    {m.supportsThinking && <span className="text-warning">R</span>}
                                    {m.supportsJsonObject && <span className="text-primary">J</span>}
                                  </div>
                                </div>
                              ))}
                            </div>
                          </div>
                        )}
                      </div>
                    )}
                  </Card>
                );
              })}
            </div>
          )}

          {/* ── Remote Registry Update ── */}
          <Card padding="md" className="mt-6 space-y-3">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold text-surface-100">远程模型注册表</h3>
            </div>
            <p className="text-xs text-surface-400">从远程服务器拉取最新模型元数据</p>
            <div className="flex gap-2">
              <Input
                label="注册表 URL"
                value={registryUrl}
                onChange={(e) => setRegistryUrl(e.target.value)}
                placeholder="https://updates.novelforge.app/llm-model-registry.json"
                containerClassName="flex-1"
              />
            </div>
            {registryStatus && (
              <div className={`px-3 py-2 rounded-lg text-sm ${
                registryStatus.startsWith("✓") ? "bg-success/10 text-success border border-success/20" :
                registryStatus.startsWith("✗") ? "bg-error/10 text-error border border-error/20" :
                "bg-info/10 text-info border border-info/20"
              }`}>
                {registryStatus}
              </div>
            )}
            <div className="flex justify-end gap-2">
              <Button variant="secondary" size="sm" onClick={handleCheckRegistry} disabled={registryChecking}>
                {registryChecking ? "检查中..." : "检查更新"}
              </Button>
              <Button variant="primary" size="sm" onClick={handleApplyRegistry} disabled={!registryUpdateAvailable || registryApplying}>
                {registryApplying ? "应用中..." : "应用更新"}
              </Button>
            </div>
          </Card>
        </div>
      )}

      {activeTab === "routing" && (
        <Card padding="lg" className="space-y-4">
          <h2 className="text-base font-semibold text-surface-100">任务路由配置</h2>
          <p className="text-sm text-surface-400">按任务类型指定 Provider / Model，保存后 AI 调用会按路由命中。</p>
          {taskRouteMessage && (
            <div className="px-3 py-2 rounded-lg text-sm bg-success/10 text-success border border-success/20">
              {taskRouteMessage}
            </div>
          )}
          {taskRoutesLoading ? (
            <p className="text-sm text-surface-400">路由加载中...</p>
          ) : (
            <div className="space-y-4">
              {TASK_ROUTE_OPTIONS.map((task) => {
                const state = taskRoutes[task.value];
                if (!state) return null;
                const route = state.route;
                const routeProviderOptions = buildRouteProviderOptions(route.providerId || "");
                const fallbackProviderOptions = [
                  { value: "", label: "不使用 fallback" },
                  ...buildRouteProviderOptions(route.fallbackProviderId || ""),
                ];
                const modelOptions = buildRouteModelOptions(route.providerId || "", route.modelId || "");
                const fallbackModelOptions = buildRouteModelOptions(
                  route.fallbackProviderId || "",
                  route.fallbackModelId || "",
                );
                return (
                  <div key={task.value} className="border border-surface-700 rounded-lg p-4 space-y-3 bg-surface-800/40">
                    <div className="flex items-center justify-between">
                      <div className="text-sm font-medium text-surface-100">{task.label}</div>
                      <div className="text-xs text-surface-500">{task.value}</div>
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                      <Select
                        label="Provider"
                        value={route.providerId || ""}
                        onChange={(e) => handleTaskRouteProviderChange(task.value, e.target.value)}
                        options={routeProviderOptions}
                        placeholder="选择 Provider"
                      />
                      {modelOptions.length > 0 ? (
                        <Select
                          label="模型 ID"
                          value={route.modelId || ""}
                          onChange={(e) => updateTaskRoute(task.value, { modelId: e.target.value })}
                          options={modelOptions}
                          placeholder="选择已配置模型"
                        />
                      ) : (
                        <Input
                          label="模型 ID"
                          value={route.modelId || ""}
                          onChange={(e) => updateTaskRoute(task.value, { modelId: e.target.value })}
                          placeholder="例如 deepseek-v4-flash"
                        />
                      )}
                      <Select
                        label="Fallback Provider"
                        value={route.fallbackProviderId || ""}
                        onChange={(e) => handleTaskRouteFallbackProviderChange(task.value, e.target.value)}
                        options={fallbackProviderOptions}
                      />
                      {fallbackModelOptions.length > 0 ? (
                        <Select
                          label="Fallback 模型 ID"
                          value={route.fallbackModelId || ""}
                          onChange={(e) => updateTaskRoute(task.value, { fallbackModelId: e.target.value })}
                          options={fallbackModelOptions}
                          placeholder="选择已配置模型"
                        />
                      ) : (
                        <Input
                          label="Fallback 模型 ID"
                          value={route.fallbackModelId || ""}
                          onChange={(e) => updateTaskRoute(task.value, { fallbackModelId: e.target.value })}
                          placeholder="可留空"
                        />
                      )}
                      <Input
                        label="最大重试次数"
                        type="number"
                        min={1}
                        max={8}
                        value={String(route.maxRetries || 1)}
                        onChange={(e) => updateTaskRoute(task.value, { maxRetries: parseInt(e.target.value) || 1 })}
                      />
                    </div>
                    {state.error && (
                      <div className="px-3 py-2 rounded-lg text-sm bg-error/10 text-error border border-error/20">
                        {state.error}
                      </div>
                    )}
                    <div className="flex justify-end gap-2 pt-1">
                      {route.id && (
                        <Button
                          variant="danger"
                          size="sm"
                          onClick={() => void handleDeleteTaskRoute(task.value)}
                          disabled={state.deleting}
                        >
                          {state.deleting ? "删除中..." : "删除"}
                        </Button>
                      )}
                      <Button
                        variant="primary"
                        size="sm"
                        onClick={() => void handleSaveTaskRoute(task.value)}
                        disabled={state.saving}
                      >
                        {state.saving ? "保存中..." : "保存路由"}
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </Card>
      )}

      {activeTab === "skills" && (
        <Card padding="lg" className="min-h-[560px]">
          <h2 className="text-base font-semibold text-surface-100 mb-4">技能管理</h2>
          <p className="text-xs text-surface-400 mb-4">
            技能是基于 Markdown 文件的 AI 提示词模板。您可以浏览内置技能、导入自定义 .md 文件、编辑现有技能内容。
          </p>
          <div className="h-[70vh] min-h-[480px]">
            <SkillsManager />
          </div>
        </Card>
      )}

      {activeTab === "editor" && (
        <Card padding="lg" className="space-y-4">
          <h2 className="text-base font-semibold text-surface-100">编辑器设置</h2>
          <div className="grid grid-cols-2 gap-4">
            <Input label="字号" type="number" value={String(editor.fontSize)} onChange={(e) => setEditor({ ...editor, fontSize: parseInt(e.target.value) || 16 })} min={12} max={24} />
            <Input label="行高" type="number" value={String(editor.lineHeight)} onChange={(e) => setEditor({ ...editor, lineHeight: parseFloat(e.target.value) || 1.75 })} step={0.05} min={1} max={3} />
          </div>
          <Input label="自动保存间隔（秒）" type="number" value={String(editor.autosaveInterval)} onChange={(e) => setEditor({ ...editor, autosaveInterval: parseInt(e.target.value) || 5 })} min={1} max={60} />
          <Select label="默认叙事视角" value={editor.narrativePov} onChange={(e) => setEditor({ ...editor, narrativePov: e.target.value })} options={[
            { value: "first", label: "第一人称" },
            { value: "third_limited", label: "第三人称限制" },
            { value: "third_omniscient", label: "第三人称全知" },
          ]} />
          <div className="pt-3 border-t border-surface-700 flex justify-end">
            <Button variant="primary" onClick={async () => { await saveEditorSettings(editor); setEditorSaved(true); setTimeout(() => setEditorSaved(false), 2000); }}>
              {editorSaved ? "已保存 ✓" : "保存"}
            </Button>
          </div>
        </Card>
      )}

      {activeTab === "writing" && (
        <Card padding="lg" className="space-y-6">
          <h2 className="text-base font-semibold text-surface-100">写作风格</h2>
          <p className="text-sm text-surface-400">
            设定作品的默认写作风格参数。AI 生成时将遵循这些参数输出相应风格文本。
          </p>

          {projectRoot && !writingStyleLoaded && (
            <p className="text-xs text-surface-500">写作风格加载中...</p>
          )}

          <div>
            <label className="text-sm text-surface-200 block mb-2">语言风格</label>
            <div className="flex flex-wrap gap-2">
              {(["plain", "balanced", "ornate", "colloquial"] as const).map((opt) => (
                <button
                  key={opt}
                  type="button"
                  onClick={() => updateWritingStyle({ languageStyle: opt })}
                  className={`px-4 py-2 text-sm rounded-lg border transition-colors ${
                    writingStyle.languageStyle === opt
                      ? "bg-primary text-white border-primary"
                      : "bg-surface-800 text-surface-300 border-surface-600 hover:border-surface-500"
                  }`}
                >
                  {{ plain: "平实", balanced: "适中", ornate: "华丽", colloquial: "口语化" }[opt]}
                </button>
              ))}
            </div>
          </div>

          <SliderControl
            label="描写密度"
            value={writingStyle.descriptionDensity}
            minLabel="点到为止"
            maxLabel="详细刻画"
            onChange={(val) => updateWritingStyle({ descriptionDensity: val })}
          />

          <SliderControl
            label="对话比例"
            value={writingStyle.dialogueRatio}
            minLabel="偏叙述"
            maxLabel="偏对话"
            onChange={(val) => updateWritingStyle({ dialogueRatio: val })}
          />

          <div>
            <label className="text-sm text-surface-200 block mb-2">句子节奏</label>
            <div className="flex flex-wrap gap-2">
              {(["short", "mixed", "long"] as const).map((opt) => (
                <button
                  key={opt}
                  type="button"
                  onClick={() => updateWritingStyle({ sentenceRhythm: opt })}
                  className={`px-4 py-2 text-sm rounded-lg border transition-colors ${
                    writingStyle.sentenceRhythm === opt
                      ? "bg-primary text-white border-primary"
                      : "bg-surface-800 text-surface-300 border-surface-600 hover:border-surface-500"
                  }`}
                >
                  {{ short: "短句为主", mixed: "混合", long: "长句为主" }[opt]}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="text-sm text-surface-200 block mb-2">氛围基调</label>
            <div className="flex flex-wrap gap-2">
              {(["warm", "cold", "humorous", "serious", "suspenseful", "neutral"] as const).map((opt) => (
                <button
                  key={opt}
                  type="button"
                  onClick={() => updateWritingStyle({ atmosphere: opt })}
                  className={`px-4 py-2 text-sm rounded-lg border transition-colors ${
                    writingStyle.atmosphere === opt
                      ? "bg-primary text-white border-primary"
                      : "bg-surface-800 text-surface-300 border-surface-600 hover:border-surface-500"
                  }`}
                >
                  {{
                    warm: "温暖",
                    cold: "冷峻",
                    humorous: "幽默",
                    serious: "严肃",
                    suspenseful: "悬疑",
                    neutral: "中性",
                  }[opt]}
                </button>
              ))}
            </div>
          </div>

          <SliderControl
            label="心理描写深度"
            value={writingStyle.psychologicalDepth}
            minLabel="仅外部行为"
            maxLabel="深入内心"
            onChange={(val) => updateWritingStyle({ psychologicalDepth: val })}
          />

          {writingStyleMessage && (
            <div className={`px-3 py-2 rounded-lg text-sm ${
              writingStyleSaved
                ? "bg-success/10 text-success border border-success/20"
                : "bg-info/10 text-info border border-info/20"
            }`}>
              {writingStyleMessage}
            </div>
          )}

          <div className="flex items-center gap-3 pt-3 border-t border-surface-700">
            <Button
              variant="primary"
              onClick={() => void handleSaveWritingStyle()}
              disabled={writingStyleSaving || !projectRoot || !writingStyleLoaded}
            >
              {writingStyleSaving ? "保存中..." : writingStyleSaved ? "已保存 ✓" : "保存风格设置"}
            </Button>
            {!projectRoot && (
              <span className="text-xs text-warning">请先打开项目以设置写作风格</span>
            )}
          </div>
        </Card>
      )}

      {activeTab === "backup" && (
        <Card padding="lg" className="space-y-4">
          <h2 className="text-base font-semibold text-surface-100">数据与备份</h2>
          <p className="text-sm text-surface-400">项目数据默认保存在本地。支持手动备份和恢复。</p>
          <p className="text-xs text-surface-500">备份包含 project.json、数据库、章节正文和蓝图文件。API Key 不会进入备份包。</p>
          <div className="flex gap-3">
            <Button variant="secondary" loading={backupCreating} onClick={() => void handleCreateBackup()}>
              {backupCreating ? "备份中..." : "创建备份"}
            </Button>
            <Button variant="secondary" loading={integrityChecking} onClick={() => void handleCheckIntegrity()}>
              {integrityChecking ? "检查中..." : "完整性检查"}
            </Button>
          </div>
          {backupMessage && (
            <div className={`px-3 py-2 rounded-lg text-sm ${
              backupMessage.startsWith("备份成功") || backupMessage.startsWith("恢复成功")
                ? "bg-success/10 text-success border border-success/20"
                : backupMessage.startsWith("备份失败") || backupMessage.startsWith("恢复失败")
                ? "bg-error/10 text-error border border-error/20"
                : "bg-info/10 text-info border border-info/20"
            }`}>
              {backupMessage}
            </div>
          )}
          {backupList.length > 0 && (
            <div>
              <h3 className="text-sm font-semibold text-surface-200 mb-2">历史备份</h3>
              <div className="space-y-2 max-h-48 overflow-y-auto">
                {backupList.map((b, i) => (
                  <div key={i} className="flex items-center justify-between p-2 bg-surface-800 rounded-lg">
                    <div className="text-xs text-surface-300 truncate flex-1">
                      {b.filePath.split(/[/\\]/).pop()}
                      <span className="text-surface-500 ml-2">({(b.fileSize / 1024).toFixed(0)} KB)</span>
                    </div>
                    <Button variant="ghost" size="sm" loading={backupRestoring}
                      onClick={() => void handleRestoreBackup(b.filePath)}>
                      恢复
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          )}
          {integrityReport && (
            <div className="space-y-2">
              <h3 className="text-sm font-semibold text-surface-200">完整性报告</h3>
              <div className="text-xs text-surface-400">
                状态:{" "}
                <span className={
                  integrityReport.status === "healthy"
                    ? "text-success"
                    : integrityReport.status === "issues_found"
                      ? "text-warning"
                      : "text-error"
                }>
                  {integrityReport.status}
                </span>
                {" · "}
                schema: {integrityReport.summary.schemaVersion}
                {" · "}
                章节正常: {integrityReport.summary.chaptersOk}
                {" · "}
                缺失: {integrityReport.summary.chaptersMissing}
                {" · "}
                孤立草稿: {integrityReport.summary.orphanDrafts}
              </div>
              {integrityReport.issues.length > 0 && (
                <div className="max-h-40 overflow-y-auto space-y-2 pr-1">
                  {integrityReport.issues.map((issue, index) => (
                    <div key={`${issue.category}-${index}`} className="text-xs px-3 py-2 rounded-lg bg-surface-800 border border-surface-700">
                      <div className={
                        issue.severity === "error"
                          ? "text-error"
                          : issue.severity === "warning"
                            ? "text-warning"
                            : "text-info"
                      }>
                        [{issue.severity}] {issue.message}
                      </div>
                      {issue.detail && <div className="text-surface-500 mt-1">{issue.detail}</div>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
          {!projectRoot && (
            <p className="text-xs text-warning">请先打开项目以使用备份功能</p>
          )}
          <div className="pt-4 border-t border-surface-700 space-y-3">
            <h3 className="text-sm font-semibold text-surface-200">Git 快照</h3>
            <p className="text-xs text-surface-400">
              支持初始化仓库、提交项目快照并查看最近历史记录。
            </p>
            <div className="text-xs text-surface-400">
              状态：{gitStatus?.initialized ? `已初始化（${gitStatus.branch}）` : "未初始化"} / {gitStatus?.hasChanges ? "有未提交变更" : "工作区干净"}
            </div>
            <Input
              label="提交说明（可选）"
              value={snapshotMessage}
              onChange={(e) => setSnapshotMessage(e.target.value)}
              placeholder="例如：完成第 10 章初稿"
            />
            <div className="flex gap-2">
              <Button variant="secondary" onClick={() => void handleInitGitRepo()} disabled={gitBusy}>
                {gitBusy ? "处理中..." : "初始化仓库"}
              </Button>
              <Button variant="primary" onClick={() => void handleCommitSnapshot()} disabled={gitBusy || !projectRoot}>
                {gitBusy ? "处理中..." : "提交快照"}
              </Button>
              <Button variant="ghost" onClick={() => void refreshGitData()} disabled={gitBusy || !projectRoot}>
                刷新历史
              </Button>
            </div>
            {gitMessage && (
              <div className="px-3 py-2 rounded-lg text-xs bg-info/10 text-info border border-info/20">
                {gitMessage}
              </div>
            )}
            {gitHistory.length > 0 && (
              <div className="space-y-2 max-h-48 overflow-y-auto pr-1">
                {gitHistory.map((row) => (
                  <div key={row.commitId} className="text-xs px-3 py-2 rounded-lg bg-surface-800 border border-surface-700">
                    <div className="text-surface-200 break-all">{row.commitId.slice(0, 10)} · {row.summary}</div>
                    <div className="text-surface-500 mt-1">{new Date(row.committedAt).toLocaleString("zh-CN")}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </Card>
      )}

      {activeTab === "about" && (
        <Card padding="lg" className="space-y-4">
          <h2 className="text-base font-semibold text-surface-100">关于 NovelForge</h2>
          <div className="space-y-2 text-sm">
            <p className="text-surface-200"><span className="text-surface-400">版本: </span>0.1.0 (MVP)</p>
            <p className="text-surface-200"><span className="text-surface-400">技术栈: </span>Tauri + React + TypeScript + Rust + SQLite</p>
            <p className="text-surface-400 italic">让灵感成为工程，让故事稳定完稿。</p>
          </div>
          <div className="pt-3 border-t border-surface-700 space-y-3">
            <h3 className="text-sm font-semibold text-surface-100">授权</h3>
            <div className="text-xs text-surface-400">
              当前状态：{licenseStatus?.activated ? `已激活（${licenseStatus.tier}）` : "未激活"} / {licenseStatus?.offlineAvailable ? "支持离线使用" : "离线不可用"}
            </div>
            {licenseStatus?.licenseKeyMasked && (
              <div className="text-xs text-surface-500">授权码：{licenseStatus.licenseKeyMasked}</div>
            )}
            <Input
              label="输入授权码"
              value={licenseKeyInput}
              onChange={(e) => setLicenseKeyInput(e.target.value)}
              placeholder="NF-XXXX-XXXX-XXXX-XXXX"
            />
            <div className="flex gap-2">
              <Button variant="primary" onClick={() => void handleActivateLicense()} disabled={licenseBusy}>
                {licenseBusy ? "激活中..." : "激活授权"}
              </Button>
              <Button
                variant="ghost"
                onClick={() => void getLicenseStatus().then(setLicenseStatus).catch(() => {})}
                disabled={licenseBusy}
              >
                刷新状态
              </Button>
            </div>
            {licenseMessage && (
              <div className="px-3 py-2 rounded-lg text-xs bg-info/10 text-info border border-info/20">
                {licenseMessage}
              </div>
            )}
          </div>
          <div className="pt-3 border-t border-surface-700 space-y-3">
            <h3 className="text-sm font-semibold text-surface-100">应用更新</h3>
            <div className="text-xs text-surface-400">
              当前版本：{updateInfo?.currentVersion || "0.1.0"}
              {updateInfo?.available ? ` / 可更新至 ${updateInfo.targetVersion || "unknown"}` : ""}
            </div>
            <div className="flex gap-2">
              <Button variant="secondary" onClick={() => void handleCheckUpdate()} disabled={updateBusy}>
                {updateBusy ? "处理中..." : "检查更新"}
              </Button>
              <Button variant="primary" onClick={() => void handleInstallUpdate()} disabled={updateBusy}>
                {updateBusy ? "处理中..." : "下载并安装"}
              </Button>
            </div>
            {updateInfo?.body && (
              <div className="text-xs text-surface-500 whitespace-pre-wrap">{updateInfo.body}</div>
            )}
            {updateMessage && (
              <div className="px-3 py-2 rounded-lg text-xs bg-info/10 text-info border border-info/20">
                {updateMessage}
              </div>
            )}
          </div>
          <div className="pt-3 border-t border-surface-700">
            <h3 className="text-sm font-semibold text-surface-100 mb-3">更新日志</h3>
            <div className="space-y-3 text-xs">
              <div>
                <div className="text-surface-200 font-medium">v0.1.0 (2026-04-27)</div>
                <ul className="text-surface-400 list-disc list-inside mt-1 space-y-0.5">
                  <li>Sprint 5: API Key 安全存储 + 多供应商配置</li>
                  <li>Windows Credential Manager 密钥存储</li>
                  <li>8 供应商卡片配置界面</li>
                </ul>
              </div>
            </div>
          </div>
        </Card>
      )}
    </div>
  );
}
