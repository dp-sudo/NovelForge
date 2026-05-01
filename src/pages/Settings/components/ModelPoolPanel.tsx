import { useEffect, useMemo, useState } from "react";
import { Card } from "../../../components/cards/Card";
import { Input } from "../../../components/forms/Input";
import { Select } from "../../../components/forms/Select";
import { Button } from "../../../components/ui/Button";
import type { CreateModelPoolInput } from "../../../api/modelPoolApi.js";
import type { ModelPool, ModelPoolEntry, ModelPoolRole } from "../../../types/modelPool.js";

interface SelectOption {
  value: string;
  label: string;
}

interface ModelPoolPanelProps {
  modelPoolsLoading: boolean;
  modelPoolMessage: string | null;
  modelPools: ModelPool[];
  providerOptions: SelectOption[];
  buildModelOptions: (providerId: string, currentModelId: string) => SelectOption[];
  onCreateModelPool: (input: CreateModelPoolInput) => Promise<void>;
  onUpdateModelPool: (poolId: string, config: ModelPool) => Promise<void>;
  onDeleteModelPool: (poolId: string) => Promise<void>;
}

interface PoolDraft {
  id: string;
  role: ModelPoolRole;
  displayName: string;
  enabled: boolean;
  fallbackPoolId: string;
  entries: ModelPoolEntry[];
  isNew: boolean;
  saving: boolean;
  deleting: boolean;
  error: string | null;
}

const MODEL_POOL_ROLES: { value: ModelPoolRole; label: string }[] = [
  { value: "planner", label: "Planner 池" },
  { value: "drafter", label: "Drafter 池" },
  { value: "reviewer", label: "Reviewer 池" },
  { value: "extractor", label: "Extractor 池" },
  { value: "state", label: "State 池" },
];

function createEmptyEntry(): ModelPoolEntry {
  return { providerId: "", modelId: "" };
}

function createDraft(role: ModelPoolRole, existing?: ModelPool): PoolDraft {
  return {
    id: existing?.id || role,
    role,
    displayName: existing?.displayName || `${MODEL_POOL_ROLES.find((item) => item.value === role)?.label || role}`,
    enabled: existing?.enabled ?? true,
    fallbackPoolId: existing?.fallbackPoolId || "",
    entries: existing?.entries.length ? existing.entries.map((entry) => ({ ...entry })) : [createEmptyEntry()],
    isNew: !existing,
    saving: false,
    deleting: false,
    error: null,
  };
}

function sanitizeEntries(entries: ModelPoolEntry[]): ModelPoolEntry[] {
  return entries
    .map((entry) => ({
      providerId: entry.providerId.trim(),
      modelId: entry.modelId.trim(),
    }))
    .filter((entry) => entry.providerId && entry.modelId);
}

function buildFallbackOptions(role: ModelPoolRole): SelectOption[] {
  const options: SelectOption[] = [{ value: "", label: "不使用兜底池" }];
  for (const item of MODEL_POOL_ROLES) {
    if (item.value === role) continue;
    options.push({ value: item.value, label: item.label });
  }
  return options;
}

export function ModelPoolPanel(props: ModelPoolPanelProps) {
  const {
    modelPoolsLoading,
    modelPoolMessage,
    modelPools,
    providerOptions,
    buildModelOptions,
    onCreateModelPool,
    onUpdateModelPool,
    onDeleteModelPool,
  } = props;

  const [drafts, setDrafts] = useState<Record<ModelPoolRole, PoolDraft>>({
    planner: createDraft("planner"),
    drafter: createDraft("drafter"),
    reviewer: createDraft("reviewer"),
    extractor: createDraft("extractor"),
    state: createDraft("state"),
  });

  useEffect(() => {
    setDrafts({
      planner: createDraft("planner", modelPools.find((pool) => pool.role === "planner")),
      drafter: createDraft("drafter", modelPools.find((pool) => pool.role === "drafter")),
      reviewer: createDraft("reviewer", modelPools.find((pool) => pool.role === "reviewer")),
      extractor: createDraft("extractor", modelPools.find((pool) => pool.role === "extractor")),
      state: createDraft("state", modelPools.find((pool) => pool.role === "state")),
    });
  }, [modelPools]);

  const roleOrder = useMemo(() => MODEL_POOL_ROLES.map((item) => item.value), []);

  function updateDraft(role: ModelPoolRole, patch: Partial<PoolDraft>) {
    setDrafts((prev) => ({
      ...prev,
      [role]: {
        ...prev[role],
        ...patch,
      },
    }));
  }

  function updateEntry(role: ModelPoolRole, index: number, patch: Partial<ModelPoolEntry>) {
    const current = drafts[role];
    if (!current) return;
    const nextEntries = current.entries.map((entry, i) => (i === index ? { ...entry, ...patch } : entry));
    updateDraft(role, { entries: nextEntries, error: null });
  }

  function addEntry(role: ModelPoolRole) {
    const current = drafts[role];
    if (!current) return;
    updateDraft(role, { entries: [...current.entries, createEmptyEntry()], error: null });
  }

  function removeEntry(role: ModelPoolRole, index: number) {
    const current = drafts[role];
    if (!current) return;
    const nextEntries = current.entries.filter((_, i) => i !== index);
    updateDraft(role, {
      entries: nextEntries.length ? nextEntries : [createEmptyEntry()],
      error: null,
    });
  }

  async function handleSave(role: ModelPoolRole) {
    const draft = drafts[role];
    if (!draft) return;
    const displayName = draft.displayName.trim();
    if (!displayName) {
      updateDraft(role, { error: "模型池名称不能为空" });
      return;
    }
    const entries = sanitizeEntries(draft.entries);
    if (entries.length === 0) {
      updateDraft(role, { error: "至少配置一个 provider/model" });
      return;
    }

    updateDraft(role, { saving: true, error: null });
    try {
      if (draft.isNew) {
        await onCreateModelPool({
          name: displayName,
          poolType: role,
          models: entries,
        });
      } else {
        await onUpdateModelPool(draft.id, {
          id: draft.id,
          displayName,
          role,
          enabled: draft.enabled,
          entries,
          fallbackPoolId: draft.fallbackPoolId || undefined,
        });
      }
    } catch (error: unknown) {
      const message = typeof error === "object" && error && "message" in error
        ? String((error as { message: string }).message)
        : "保存模型池失败";
      updateDraft(role, { error: message });
    } finally {
      updateDraft(role, { saving: false });
    }
  }

  async function handleDelete(role: ModelPoolRole) {
    const draft = drafts[role];
    if (!draft || draft.isNew) return;
    updateDraft(role, { deleting: true, error: null });
    try {
      await onDeleteModelPool(draft.id);
    } catch (error: unknown) {
      const message = typeof error === "object" && error && "message" in error
        ? String((error as { message: string }).message)
        : "删除模型池失败";
      updateDraft(role, { error: message });
    } finally {
      updateDraft(role, { deleting: false });
    }
  }

  return (
    <Card padding="lg" className="space-y-4">
      <h2 className="text-base font-semibold text-surface-100">模型池管理</h2>
      <p className="text-sm text-surface-400">
        按角色维护 `planner/drafter/reviewer/extractor/state` 池。任务路由优先引用模型池。
      </p>
      {modelPoolMessage && (
        <div className="px-3 py-2 rounded-lg text-sm bg-info/10 text-info border border-info/20">
          {modelPoolMessage}
        </div>
      )}
      {providerOptions.length === 0 && (
        <div className="px-3 py-2 rounded-lg text-sm bg-warning/10 text-warning border border-warning/30">
          请先在“模型配置”里保存至少一个供应商，再配置模型池。
        </div>
      )}
      {modelPoolsLoading ? (
        <p className="text-sm text-surface-400">模型池加载中...</p>
      ) : (
        <div className="space-y-4">
          {roleOrder.map((role) => {
            const draft = drafts[role];
            if (!draft) return null;
            const fallbackOptions = buildFallbackOptions(role);
            return (
              <div key={role} className="border border-surface-700 rounded-lg p-4 space-y-3 bg-surface-800/40">
                <div className="flex items-center justify-between gap-2">
                  <div>
                    <div className="text-sm font-medium text-surface-100">
                      {MODEL_POOL_ROLES.find((item) => item.value === role)?.label}
                    </div>
                    <div className="text-xs text-surface-500">{draft.id}</div>
                  </div>
                  <div className="text-xs text-surface-500">{draft.isNew ? "未创建" : "已创建"}</div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                  <Input
                    label="池名称"
                    value={draft.displayName}
                    onChange={(event) => updateDraft(role, { displayName: event.target.value, error: null })}
                    placeholder="例如 Drafter Pool"
                  />
                  <Select
                    label="状态"
                    value={draft.enabled ? "enabled" : "disabled"}
                    onChange={(event) => updateDraft(role, { enabled: event.target.value === "enabled", error: null })}
                    options={[
                      { value: "enabled", label: "启用" },
                      { value: "disabled", label: "禁用" },
                    ]}
                  />
                  <Select
                    label="兜底池"
                    value={draft.fallbackPoolId}
                    onChange={(event) => updateDraft(role, { fallbackPoolId: event.target.value, error: null })}
                    options={fallbackOptions}
                  />
                </div>

                <div className="space-y-2">
                  <div className="text-xs font-semibold text-surface-400 uppercase tracking-wider">池内模型</div>
                  <div className="space-y-2">
                    {draft.entries.map((entry, index) => {
                      const modelOptions = buildModelOptions(entry.providerId, entry.modelId);
                      return (
                        <div key={`${role}-${index}`} className="grid grid-cols-1 md:grid-cols-[1fr_1fr_auto] gap-2 items-end">
                          <Select
                            label={index === 0 ? "供应商" : undefined}
                            value={entry.providerId}
                            onChange={(event) => {
                              const providerId = event.target.value;
                              const firstModel = buildModelOptions(providerId, "")[0]?.value || "";
                              updateEntry(role, index, { providerId, modelId: firstModel });
                            }}
                            options={providerOptions}
                          />
                          {modelOptions.length > 0 ? (
                            <Select
                              label={index === 0 ? "模型 ID" : undefined}
                              value={entry.modelId}
                              onChange={(event) => updateEntry(role, index, { modelId: event.target.value })}
                              options={modelOptions}
                            />
                          ) : (
                            <Input
                              label={index === 0 ? "模型 ID" : undefined}
                              value={entry.modelId}
                              onChange={(event) => updateEntry(role, index, { modelId: event.target.value })}
                              placeholder="例如 gpt-5.5"
                            />
                          )}
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => removeEntry(role, index)}
                            disabled={draft.entries.length <= 1}
                          >
                            删除
                          </Button>
                        </div>
                      );
                    })}
                  </div>
                  <div>
                    <Button variant="secondary" size="sm" onClick={() => addEntry(role)}>
                      新增模型
                    </Button>
                  </div>
                </div>

                {draft.error && (
                  <div className="px-3 py-2 rounded-lg text-sm bg-error/10 text-error border border-error/20">
                    {draft.error}
                  </div>
                )}

                <div className="flex justify-end gap-2">
                  {!draft.isNew && (
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => void handleDelete(role)}
                      disabled={draft.deleting}
                    >
                      {draft.deleting ? "删除中..." : "删除池"}
                    </Button>
                  )}
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => void handleSave(role)}
                    disabled={draft.saving || providerOptions.length === 0}
                  >
                    {draft.saving ? "保存中..." : draft.isNew ? "创建池" : "保存池"}
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
