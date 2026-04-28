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
  listTaskRoutes,
  saveTaskRoute,
  deleteTaskRoute,
  loadEditorSettings,
  saveEditorSettings,
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
import {
  checkProjectIntegrity,
  createBackup,
  listBackups,
  restoreBackup,
  type BackupResult,
  type IntegrityReport,
} from "../../api/chapterApi";
import { useProjectStore } from "../../stores/projectStore";
import { VENDOR_PRESETS, type VendorInfo } from "../../types/ai";

type TabKey = "model" | "routing" | "editor" | "backup" | "about";

interface VendorFormState {
  config: LlmProviderConfig;
  apiKeyInput: string;
  betaHeadersInput: string;
  customHeadersInput: string;
  saving: boolean;
  testing: boolean;
  refreshing: boolean;
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

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState<TabKey>("model");
  const [vendors, setVendors] = useState<Record<string, VendorFormState>>({});
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

  const TASK_ROUTE_OPTIONS = [
    { value: "chapter_draft", label: "章节草稿" },
    { value: "chapter_continue", label: "章节续写" },
    { value: "chapter_rewrite", label: "局部改写" },
    { value: "prose_naturalize", label: "去 AI 味" },
    { value: "character.create", label: "角色生成" },
    { value: "world.generate", label: "世界观生成" },
    { value: "consistency.scan", label: "一致性检查" },
    { value: "blueprint.generate_step", label: "蓝图生成" },
    { value: "plot.generate", label: "剧情生成" },
  ] as const;

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
      if (!config.defaultModel?.trim()) return "自定义 Provider 默认模型不能为空";
      if (!config.protocol) return "请选择自定义 Provider 协议";
      if (config.authMode === "custom" && !config.authHeaderName?.trim()) {
        return "自定义认证模式必须填写 Auth Header 名称";
      }
      if (config.modelsPath && !config.modelsPath.startsWith("/")) {
        return "模型列表路径必须以 / 开头";
      }
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

  useEffect(() => {
    (async () => {
      setLoading(true);
      setTaskRoutesLoading(true);
      try {
        const [configs, routes] = await Promise.all([
          listProviders(),
          listTaskRoutes(),
        ]);
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
            betaHeadersInput: mapToInput(existing?.betaHeaders),
            customHeadersInput: mapToInput(existing?.customHeaders),
            saving: false,
            testing: false,
            refreshing: false,
            refreshResult: null,
            testResult: null,
            validationError: null,
            expanded: false,
          };
        }

        const routeMap: Record<string, TaskRouteFormState> = {};
        for (const task of TASK_ROUTE_OPTIONS) {
          const existingRoute = routes.find((route) => route.taskType === task.value);
          routeMap[task.value] = {
            route: existingRoute
              ? normalizeRoute(existingRoute)
              : {
                  id: "",
                  taskType: task.value,
                  providerId: "",
                  modelId: "",
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

    if (preset.id === "custom" && normalizedConfig.protocol === "custom_openai_compatible" && !normalizedConfig.endpointPath) {
      normalizedConfig.endpointPath = "/chat/completions";
    }
    if (preset.id === "custom" && normalizedConfig.protocol === "custom_anthropic_compatible" && !normalizedConfig.endpointPath) {
      normalizedConfig.endpointPath = "/messages";
    }

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
      const saved = await saveProvider(normalizedConfig, v.apiKeyInput || undefined);
      setVendors((prev) => ({
        ...prev,
        [preset.id]: {
          ...prev[preset.id],
          config: saved,
          apiKeyInput: "",
          betaHeadersInput: mapToInput(saved.betaHeaders),
          customHeadersInput: mapToInput(saved.customHeaders),
          saving: false,
          validationError: null,
        },
      }));
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
    setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], testing: true, testResult: null } }));
    try {
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
    setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], refreshing: true, refreshResult: null } }));
    try {
      const result = await refreshProviderModels(preset.id);
      setVendors((prev) => ({
        ...prev,
        [preset.id]: {
          ...prev[preset.id],
          refreshing: false,
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
        betaHeadersInput: "",
        customHeadersInput: "",
        validationError: null,
      },
    }));
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
    { key: "editor", label: "编辑器" },
    { key: "backup", label: "数据与备份" },
    { key: "about", label: "关于" },
  ];

  const providerOptions = VENDOR_PRESETS.map((preset) => ({
    value: preset.id,
    label: vendors[preset.id]?.config.displayName || preset.displayName,
  }));

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
                        {preset.id === "custom" && (
                          <Input
                            label="Endpoint Path"
                            value={config.endpointPath || ""}
                            onChange={(e) => updateVendor(preset.id, { endpointPath: e.target.value })}
                            placeholder={config.protocol === "custom_anthropic_compatible" ? "/messages" : "/chat/completions"}
                          />
                        )}
                        <ApiKeyInput label="API Key" value={v.apiKeyInput} onChange={(val) => setVendors((prev) => ({ ...prev, [preset.id]: { ...prev[preset.id], apiKeyInput: val } }))} maskedValue={config.apiKey && !v.apiKeyInput ? config.apiKey : undefined} />
                        {preset.id === "custom" && (
                          <Select
                            label="认证模式"
                            value={config.authMode}
                            onChange={(e) => updateVendor(preset.id, { authMode: e.target.value })}
                            options={[
                              { value: "bearer", label: "Bearer" },
                              { value: "x-api-key", label: "x-api-key" },
                              { value: "custom", label: "Custom Header" },
                            ]}
                          />
                        )}
                        {preset.id === "custom" && config.authMode === "custom" && (
                          <Input
                            label="Auth Header 名称"
                            value={config.authHeaderName || ""}
                            onChange={(e) => updateVendor(preset.id, { authHeaderName: e.target.value })}
                            placeholder="X-Auth-Token"
                          />
                        )}
                        {preset.id === "custom" && config.protocol === "custom_anthropic_compatible" && (
                          <Input
                            label="Anthropic Version"
                            value={config.anthropicVersion || ""}
                            onChange={(e) => updateVendor(preset.id, { anthropicVersion: e.target.value })}
                            placeholder="2023-06-01"
                          />
                        )}
                        <Input label="默认模型" value={config.defaultModel || ""} onChange={(e) => updateVendor(preset.id, { defaultModel: e.target.value })} placeholder={preset.defaultModel} />
                        {preset.id === "custom" && (
                          <Input
                            label="模型列表路径"
                            value={config.modelsPath || ""}
                            onChange={(e) => updateVendor(preset.id, { modelsPath: e.target.value })}
                            placeholder="/models"
                          />
                        )}
                        {preset.id === "custom" && (
                          <Input
                            label="Beta Headers（key:value，每行一条）"
                            value={v.betaHeadersInput}
                            onChange={(e) =>
                              setVendors((prev) => ({
                                ...prev,
                                [preset.id]: { ...prev[preset.id], betaHeadersInput: e.target.value, validationError: null },
                              }))
                            }
                            placeholder="context-1m-2025-08-07:true"
                          />
                        )}
                        {preset.id === "custom" && (
                          <Input
                            label="自定义 Headers（key:value，每行一条）"
                            value={v.customHeadersInput}
                            onChange={(e) =>
                              setVendors((prev) => ({
                                ...prev,
                                [preset.id]: { ...prev[preset.id], customHeadersInput: e.target.value, validationError: null },
                              }))
                            }
                            placeholder="x-tenant-id:novelforge"
                          />
                        )}
                        <div className="grid grid-cols-2 gap-3">
                          <Input label="超时(ms)" type="number" value={String(config.timeoutMs)} onChange={(e) => updateVendor(preset.id, { timeoutMs: parseInt(e.target.value) || 120000 })} />
                          <Input label="重试次数" type="number" value={String(config.maxRetries)} onChange={(e) => updateVendor(preset.id, { maxRetries: parseInt(e.target.value) || 2 })} />
                        </div>
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
                      </div>
                    )}
                  </Card>
                );
              })}
            </div>
          )}
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
                        onChange={(e) => updateTaskRoute(task.value, { providerId: e.target.value })}
                        options={providerOptions}
                        placeholder="选择 Provider"
                      />
                      <Input
                        label="模型 ID"
                        value={route.modelId || ""}
                        onChange={(e) => updateTaskRoute(task.value, { modelId: e.target.value })}
                        placeholder="例如 deepseek-v4-flash"
                      />
                      <Select
                        label="Fallback Provider"
                        value={route.fallbackProviderId || ""}
                        onChange={(e) => updateTaskRoute(task.value, { fallbackProviderId: e.target.value })}
                        options={[{ value: "", label: "不使用 fallback" }, ...providerOptions]}
                      />
                      <Input
                        label="Fallback 模型 ID"
                        value={route.fallbackModelId || ""}
                        onChange={(e) => updateTaskRoute(task.value, { fallbackModelId: e.target.value })}
                        placeholder="可留空"
                      />
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
