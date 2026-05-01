import { Card } from "../../../components/cards/Card";
import { Input } from "../../../components/forms/Input";
import { Select } from "../../../components/forms/Select";
import { Button } from "../../../components/ui/Button";
import { TASK_ROUTE_OPTIONS } from "../../../utils/taskRouting.js";
import type { TaskRoute } from "../../../types/ai.js";

interface SelectOption {
  value: string;
  label: string;
}

interface TaskRouteUiState {
  route: TaskRoute;
  saving: boolean;
  deleting: boolean;
  error: string | null;
}

interface ModelRoutingPanelProps {
  hasConfiguredProvidersForRouting: boolean;
  hasConfiguredModelPools: boolean;
  taskRouteMessage: string | null;
  taskRoutesLoading: boolean;
  taskRoutes: Record<string, TaskRouteUiState>;
  modelPoolOptions: SelectOption[];
  buildFallbackPoolOptions: (currentPoolId: string) => SelectOption[];
  buildRouteProviderOptions: (currentProviderId: string) => SelectOption[];
  buildRouteModelOptions: (providerId: string, currentModelId: string) => SelectOption[];
  onTaskRouteProviderChange: (taskType: string, providerId: string) => void;
  onTaskRouteFallbackProviderChange: (taskType: string, providerId: string) => void;
  onUpdateTaskRoute: (taskType: string, patch: Partial<TaskRoute>) => void;
  onSaveTaskRoute: (taskType: string) => Promise<void>;
  onDeleteTaskRoute: (taskType: string) => Promise<void>;
}

function parsePostTaskList(raw: string): string[] {
  const values = raw
    .split(/[\n,;，；]/)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
  return Array.from(new Set(values));
}

export function ModelRoutingPanel(props: ModelRoutingPanelProps) {
  const {
    hasConfiguredProvidersForRouting,
    hasConfiguredModelPools,
    taskRouteMessage,
    taskRoutesLoading,
    taskRoutes,
    modelPoolOptions,
    buildFallbackPoolOptions,
    buildRouteProviderOptions,
    buildRouteModelOptions,
    onTaskRouteProviderChange,
    onTaskRouteFallbackProviderChange,
    onUpdateTaskRoute,
    onSaveTaskRoute,
    onDeleteTaskRoute,
  } = props;

  return (
    <Card padding="lg" className="space-y-4">
      <h2 className="text-base font-semibold text-surface-100">任务路由配置</h2>
      <p className="text-sm text-surface-400">按任务类型配置模型池路由，必要时可切换为直接 provider/model 兼容模式。</p>
      {!hasConfiguredProvidersForRouting && (
        <div className="px-3 py-2 rounded-lg text-sm bg-warning/10 text-warning border border-warning/30">
          请先在“模型设置”中保存至少一个供应商，再配置任务路由。
        </div>
      )}
      {hasConfiguredProvidersForRouting && !hasConfiguredModelPools && (
        <div className="px-3 py-2 rounded-lg text-sm bg-info/10 text-info border border-info/20">
          尚未创建模型池，当前默认使用直接路由模式。
        </div>
      )}
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
            const isPoolMode = Boolean(route.modelPoolId && route.modelPoolId.trim());
            const routeProviderOptions = buildRouteProviderOptions(route.providerId || "");
            const fallbackProviderOptions = [
              { value: "", label: "不使用兜底" },
              ...buildRouteProviderOptions(route.fallbackProviderId || ""),
            ];
            const fallbackPoolOptions = buildFallbackPoolOptions(route.modelPoolId || "");
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
                    label="路由模式"
                    value={isPoolMode ? "pool" : "direct"}
                    onChange={(e) => {
                      if (e.target.value === "pool") {
                        onUpdateTaskRoute(task.value, {
                          modelPoolId: route.modelPoolId || modelPoolOptions[0]?.value || "",
                        });
                      } else {
                        onUpdateTaskRoute(task.value, {
                          modelPoolId: undefined,
                          fallbackModelPoolId: undefined,
                        });
                      }
                    }}
                    options={[
                      { value: "pool", label: "模型池路由" },
                      { value: "direct", label: "直接 provider/model（兼容）" },
                    ]}
                  />
                  {isPoolMode ? (
                    <Select
                      label="主模型池"
                      value={route.modelPoolId || ""}
                      onChange={(e) => onUpdateTaskRoute(task.value, { modelPoolId: e.target.value })}
                      options={modelPoolOptions}
                      placeholder="选择模型池"
                    />
                  ) : (
                    <Select
                      label="供应商"
                      value={route.providerId || ""}
                      onChange={(e) => onTaskRouteProviderChange(task.value, e.target.value)}
                      options={routeProviderOptions}
                      placeholder="选择供应商"
                    />
                  )}
                  {isPoolMode ? (
                    <Select
                      label="兜底模型池"
                      value={route.fallbackModelPoolId || ""}
                      onChange={(e) =>
                        onUpdateTaskRoute(task.value, { fallbackModelPoolId: e.target.value || undefined })
                      }
                      options={fallbackPoolOptions}
                    />
                  ) : modelOptions.length > 0 ? (
                    <Select
                      label="模型 ID"
                      value={route.modelId || ""}
                      onChange={(e) => onUpdateTaskRoute(task.value, { modelId: e.target.value })}
                      options={modelOptions}
                      placeholder="选择已配置模型"
                    />
                  ) : (
                    <Input
                      label="模型 ID"
                      value={route.modelId || ""}
                      onChange={(e) => onUpdateTaskRoute(task.value, { modelId: e.target.value })}
                      placeholder="例如 deepseek-v4-flash"
                    />
                  )}
                  {!isPoolMode && (
                    <>
                      <Select
                        label="兜底供应商"
                        value={route.fallbackProviderId || ""}
                        onChange={(e) => onTaskRouteFallbackProviderChange(task.value, e.target.value)}
                        options={fallbackProviderOptions}
                      />
                      {fallbackModelOptions.length > 0 ? (
                        <Select
                          label="兜底模型ID"
                          value={route.fallbackModelId || ""}
                          onChange={(e) => onUpdateTaskRoute(task.value, { fallbackModelId: e.target.value })}
                          options={fallbackModelOptions}
                          placeholder="选择已配置模型"
                        />
                      ) : (
                        <Input
                          label="兜底模型ID"
                          value={route.fallbackModelId || ""}
                          onChange={(e) => onUpdateTaskRoute(task.value, { fallbackModelId: e.target.value })}
                          placeholder="可留空"
                        />
                      )}
                    </>
                  )}
                  <Input
                    label="最大重试次数"
                    type="number"
                    min={1}
                    max={8}
                    value={String(route.maxRetries || 1)}
                    onChange={(e) => onUpdateTaskRoute(task.value, { maxRetries: parseInt(e.target.value, 10) || 1 })}
                  />
                  <Input
                    label="后置任务（逗号分隔）"
                    value={(route.postTasks || []).join(", ")}
                    onChange={(e) =>
                      onUpdateTaskRoute(task.value, {
                        postTasks: parsePostTaskList(e.target.value),
                      })
                    }
                    placeholder="review_continuity, extract_state, extract_assets"
                    helperText="默认映射：dialogue→extract_state；combat→review_continuity+extract_state；exposition→extract_assets"
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
                      onClick={() => void onDeleteTaskRoute(task.value)}
                      disabled={state.deleting}
                    >
                      {state.deleting ? "删除中..." : "删除"}
                    </Button>
                  )}
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => void onSaveTaskRoute(task.value)}
                    disabled={state.saving || !hasConfiguredProvidersForRouting}
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
  );
}
