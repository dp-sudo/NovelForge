import { useEffect, useState, useCallback } from "react";
import { Card } from "../../components/cards/Card.js";
import { Button } from "../../components/ui/Button.js";
import { Input } from "../../components/forms/Input.js";
import { Select } from "../../components/forms/Select.js";
import { Textarea } from "../../components/forms/Textarea.js";
import { Modal } from "../../components/dialogs/Modal.js";
import { ConfirmDialog } from "../../components/dialogs/ConfirmDialog.js";
import { Badge } from "../../components/ui/Badge.js";
import {
  listConstitutionRules,
  createConstitutionRule,
  deleteConstitutionRule,
  toggleConstitutionRule,
  validateTextAgainstConstitution,
  type ConstitutionRule,
  type ConstitutionValidationResult,
} from "../../api/constitutionApi.js";
import { useProjectStore } from "../../stores/projectStore.js";

const CATEGORIES = [
  { value: "narrative", label: "叙事规则" },
  { value: "character", label: "角色规则" },
  { value: "world", label: "世界规则" },
  { value: "style", label: "文风规则" },
  { value: "plot", label: "剧情规则" },
  { value: "other", label: "其他" },
];

const SEVERITIES = [
  { value: "warning", label: "警告" },
  { value: "blocker", label: "阻断" },
];

const RULE_TYPES = [
  { value: "must", label: "必须遵守" },
  { value: "must_not", label: "禁止" },
  { value: "should", label: "建议" },
];

const emptyForm = {
  ruleType: "must" as string,
  category: "narrative" as string,
  content: "",
  severity: "warning" as string,
};

export function ConstitutionPage() {
  const [rules, setRules] = useState<ConstitutionRule[]>([]);
  const [filter, setFilter] = useState("全部");
  const [selected, setSelected] = useState<ConstitutionRule | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [showDelete, setShowDelete] = useState(false);
  const [form, setForm] = useState(emptyForm);

  // Validation panel
  const [validateText, setValidateText] = useState("");
  const [validating, setValidating] = useState(false);
  const [validationResult, setValidationResult] =
    useState<ConstitutionValidationResult | null>(null);

  const projectRoot = useProjectStore((s) => s.currentProjectPath);

  const load = useCallback(async () => {
    if (!projectRoot) {
      setRules([]);
      return;
    }
    const data = await listConstitutionRules(projectRoot);
    setRules(data);
  }, [projectRoot]);

  useEffect(() => {
    void load();
  }, [load]);

  const categories = [
    "全部",
    ...new Set(rules.map((r) => r.category)),
  ];
  const filtered =
    filter === "全部" ? rules : rules.filter((r) => r.category === filter);

  const severityVariant = (s: string) =>
    s === "blocker" ? "error" : "warning";

  async function handleCreate() {
    if (!form.content.trim() || !projectRoot) return;
    await createConstitutionRule(projectRoot, {
      ruleType: form.ruleType,
      category: form.category,
      content: form.content.trim(),
      severity: form.severity,
    });
    setForm(emptyForm);
    setShowNew(false);
    await load();
  }

  async function handleDelete() {
    if (!selected || !projectRoot) return;
    await deleteConstitutionRule(projectRoot, selected.id);
    setShowDelete(false);
    setSelected(null);
    await load();
  }

  async function handleToggle(rule: ConstitutionRule) {
    if (!projectRoot) return;
    await toggleConstitutionRule(projectRoot, rule.id, !rule.isActive);
    await load();
    if (selected?.id === rule.id) {
      setSelected({ ...rule, isActive: !rule.isActive });
    }
  }

  async function handleValidate() {
    if (!validateText.trim() || !projectRoot) return;
    setValidating(true);
    try {
      const result = await validateTextAgainstConstitution(
        projectRoot,
        validateText
      );
      setValidationResult(result);
    } catch {
      setValidationResult(null);
    } finally {
      setValidating(false);
    }
  }

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-surface-100">故事宪法</h1>
          <p className="text-sm text-surface-400 mt-1">
            定义不可违反的叙事规则，AI 生成时自动遵守并校验
          </p>
        </div>
        <Button
          variant="primary"
          size="sm"
          onClick={() => {
            setForm(emptyForm);
            setShowNew(true);
          }}
        >
          新建规则
        </Button>
      </div>

      <div className="flex gap-6">
        {/* Left: Category filter */}
        <div className="w-40 shrink-0">
          <div className="space-y-1">
            {categories.map((cat) => (
              <button
                key={cat}
                onClick={() => setFilter(cat)}
                className={`w-full text-left px-3 py-2 text-sm rounded-lg transition-colors ${
                  filter === cat
                    ? "bg-primary/10 text-primary"
                    : "text-surface-300 hover:bg-surface-700"
                }`}
              >
                {cat === "全部"
                  ? `全部 (${rules.length})`
                  : `${CATEGORIES.find((c) => c.value === cat)?.label ?? cat} (${rules.filter((r) => r.category === cat).length})`}
              </button>
            ))}
          </div>
        </div>

        {/* Middle: Rule list */}
        <div className="w-72 shrink-0 space-y-2">
          {filtered.length === 0 ? (
            <Card padding="md" className="text-center">
              <p className="text-sm text-surface-400">暂无宪法规则</p>
            </Card>
          ) : (
            filtered.map((rule) => (
              <button
                key={rule.id}
                onClick={() => setSelected(rule)}
                className={`w-full text-left p-3 rounded-lg transition-colors border ${
                  selected?.id === rule.id
                    ? "bg-primary/10 border-primary/30"
                    : "bg-surface-800 border-surface-700 hover:border-surface-500"
                }`}
              >
                <div className="flex items-center gap-2">
                  <span
                    className={`w-2 h-2 rounded-full shrink-0 ${rule.isActive ? "bg-success" : "bg-surface-500"}`}
                  />
                  <span className="text-sm font-medium text-surface-100 truncate">
                    {rule.content.slice(0, 40)}
                    {rule.content.length > 40 ? "..." : ""}
                  </span>
                </div>
                <div className="flex items-center gap-2 mt-1.5 ml-4">
                  <Badge variant={severityVariant(rule.severity)}>
                    {rule.severity === "blocker" ? "阻断" : "警告"}
                  </Badge>
                  <span className="text-xs text-surface-400">
                    {RULE_TYPES.find((t) => t.value === rule.ruleType)?.label ??
                      rule.ruleType}
                  </span>
                </div>
              </button>
            ))
          )}
        </div>

        {/* Right: Detail panel */}
        <div className="flex-1 min-w-0 space-y-4">
          {!selected ? (
            <Card padding="lg" className="text-center">
              <p className="text-surface-400 text-sm">
                选择一条宪法规则查看详情
              </p>
            </Card>
          ) : (
            <Card padding="lg" className="space-y-4">
              <div className="flex items-center justify-between">
                <h2 className="text-lg font-semibold text-surface-100">
                  规则详情
                </h2>
                <div className="flex gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => void handleToggle(selected)}
                  >
                    {selected.isActive ? "禁用" : "启用"}
                  </Button>
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={() => setShowDelete(true)}
                  >
                    删除
                  </Button>
                </div>
              </div>
              <div className="grid grid-cols-3 gap-4">
                <Input
                  label="类型"
                  value={
                    RULE_TYPES.find((t) => t.value === selected.ruleType)
                      ?.label ?? selected.ruleType
                  }
                  readOnly
                />
                <Input
                  label="分类"
                  value={
                    CATEGORIES.find((c) => c.value === selected.category)
                      ?.label ?? selected.category
                  }
                  readOnly
                />
                <Input
                  label="严重性"
                  value={
                    SEVERITIES.find((s) => s.value === selected.severity)
                      ?.label ?? selected.severity
                  }
                  readOnly
                />
              </div>
              <Textarea
                label="规则内容"
                value={selected.content}
                readOnly
                className="min-h-[80px]"
              />
              <div className="flex items-center gap-4 text-xs text-surface-400">
                <span>
                  状态：
                  <span
                    className={
                      selected.isActive ? "text-success" : "text-surface-500"
                    }
                  >
                    {selected.isActive ? "启用" : "已禁用"}
                  </span>
                </span>
                <span>创建于 {selected.createdAt}</span>
              </div>
            </Card>
          )}

          {/* Validation panel */}
          <Card padding="lg" className="space-y-3">
            <h3 className="text-sm font-semibold text-surface-200">
              文本校验
            </h3>
            <Textarea
              label="输入待校验文本"
              value={validateText}
              onChange={(e) => setValidateText(e.target.value)}
              placeholder="粘贴章节内容或任意文本，检查是否违反宪法规则..."
              className="min-h-[80px]"
            />
            <Button
              variant="primary"
              size="sm"
              onClick={() => void handleValidate()}
              loading={validating}
              disabled={!validateText.trim()}
            >
              {validating ? "校验中..." : "运行校验"}
            </Button>
            {validationResult && (
              <div
                className={`p-3 rounded-lg border ${validationResult.violationsFound > 0 ? "bg-error/5 border-error/20" : "bg-success/5 border-success/20"}`}
              >
                <p
                  className={`text-sm font-medium ${validationResult.violationsFound > 0 ? "text-error" : "text-success"}`}
                >
                  {validationResult.violationsFound > 0
                    ? `发现 ${validationResult.violationsFound} 项违规`
                    : `已通过 ${validationResult.totalRulesChecked} 项规则检查`}
                </p>
                {validationResult.violations.map((v, idx) => (
                  <div
                    key={idx}
                    className="mt-2 p-2 bg-surface-800 rounded text-xs text-surface-300"
                  >
                    <Badge variant={v.severity === "blocker" ? "error" : "warning"} className="mb-1">
                      {v.severity}
                    </Badge>
                    <p className="mt-1">{v.violationText}</p>
                    <p className="text-surface-500 mt-0.5">
                      规则：{v.ruleContent}
                    </p>
                  </div>
                ))}
              </div>
            )}
          </Card>
        </div>
      </div>

      {/* Create modal */}
      <Modal
        open={showNew}
        onClose={() => setShowNew(false)}
        title="新建宪法规则"
        width="lg"
      >
        <div className="space-y-4">
          <div className="grid grid-cols-3 gap-4">
            <Select
              label="规则类型"
              value={form.ruleType}
              onChange={(e) => setForm({ ...form, ruleType: e.target.value })}
              options={RULE_TYPES}
            />
            <Select
              label="分类"
              value={form.category}
              onChange={(e) => setForm({ ...form, category: e.target.value })}
              options={CATEGORIES}
            />
            <Select
              label="严重性"
              value={form.severity}
              onChange={(e) => setForm({ ...form, severity: e.target.value })}
              options={SEVERITIES}
            />
          </div>
          <Textarea
            label="规则内容 *"
            value={form.content}
            onChange={(e) => setForm({ ...form, content: e.target.value })}
            placeholder="例如：主角不得在第 10 章之前知道自己的真实身份"
            className="min-h-[100px]"
          />
          <div className="pt-3 border-t border-surface-700 flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowNew(false)}>
              取消
            </Button>
            <Button
              variant="primary"
              onClick={() => void handleCreate()}
              disabled={!form.content.trim()}
            >
              创建
            </Button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        open={showDelete}
        title="删除宪法规则"
        message="确定删除此规则？删除后无法恢复。"
        variant="danger"
        confirmLabel="删除"
        onConfirm={() => void handleDelete()}
        onCancel={() => setShowDelete(false)}
      />
    </div>
  );
}
