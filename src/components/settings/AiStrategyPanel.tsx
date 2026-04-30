import { useEffect, useMemo, useState } from "react";
import { Card } from "../cards/Card";
import { Button } from "../ui/Button";
import { Input } from "../forms/Input";
import { Select } from "../forms/Select";
import { Textarea } from "../forms/Textarea";
import {
  getProjectAiStrategy,
  saveProjectAiStrategy,
} from "../../api/settingsApi";
import {
  defaultAiStrategyProfile,
  type AiStrategyProfile,
} from "../../types/ai";

const REVIEW_LEVEL_LABELS = ["relaxed", "standard", "strict", "pedantic", "exhaustive"];

function listToLines(items: string[]): string {
  return items.join("\n");
}

function linesToList(raw: string): string[] {
  return raw
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

interface AiStrategyPanelProps {
  projectRoot: string | null;
}

interface StrictnessControlProps {
  value: number;
  onChange: (next: number) => void;
}

function StrictnessControl({ value, onChange }: StrictnessControlProps) {
  return (
    <div>
      <label className="text-sm text-surface-200 block mb-2">审查严格度</label>
      <div className="flex items-center gap-3">
        <div className="flex gap-1.5 flex-1">
          {REVIEW_LEVEL_LABELS.map((label, index) => {
            const level = index + 1;
            return (
              <button
                key={label}
                type="button"
                onClick={() => onChange(level)}
                className={`px-3 py-2 rounded-lg text-xs border transition-colors ${
                  level === value
                    ? "bg-primary text-white border-primary"
                    : "bg-surface-800 text-surface-300 border-surface-600 hover:border-surface-500"
                }`}
              >
                {level} · {label}
              </button>
            );
          })}
        </div>
      </div>
      <p className="text-xs text-surface-400 mt-2">
        值越高，AI 对连续性与规则冲突会做更严格的自检。
      </p>
    </div>
  );
}

export function AiStrategyPanel({ projectRoot }: AiStrategyPanelProps) {
  const [profile, setProfile] = useState<AiStrategyProfile>(defaultAiStrategyProfile());
  const [loading, setLoading] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const workflowStackText = useMemo(
    () => listToLines(profile.defaultWorkflowStack),
    [profile.defaultWorkflowStack],
  );
  const capabilityBundlesText = useMemo(
    () => listToLines(profile.defaultCapabilityBundles),
    [profile.defaultCapabilityBundles],
  );
  const policySkillsText = useMemo(
    () => listToLines(profile.alwaysOnPolicySkills),
    [profile.alwaysOnPolicySkills],
  );

  function patchProfile(patch: Partial<AiStrategyProfile>) {
    setProfile((prev) => ({ ...prev, ...patch }));
    setSaved(false);
  }

  useEffect(() => {
    let canceled = false;

    if (!projectRoot) {
      setProfile(defaultAiStrategyProfile());
      setLoaded(false);
      setMessage(null);
      return () => {
        canceled = true;
      };
    }

    setLoading(true);
    setLoaded(false);
    setSaved(false);
    setMessage(null);

    (async () => {
      try {
        const next = await getProjectAiStrategy(projectRoot);
        if (!canceled) {
          setProfile(next);
        }
      } catch (err: unknown) {
        if (canceled) return;
        setProfile(defaultAiStrategyProfile());
        setMessage(
          typeof err === "object" && err && "message" in err
            ? `加载 AI 策略失败：${String((err as { message: string }).message)}`
            : "加载 AI 策略失败",
        );
      } finally {
        if (!canceled) {
          setLoading(false);
          setLoaded(true);
        }
      }
    })();

    return () => {
      canceled = true;
    };
  }, [projectRoot]);

  async function handleSave() {
    if (!projectRoot) {
      setMessage("请先打开项目后再保存 AI 策略");
      return;
    }

    setSaving(true);
    setMessage(null);
    try {
      await saveProjectAiStrategy(projectRoot, profile);
      setSaved(true);
      setMessage("AI 策略已保存");
      setTimeout(() => setSaved(false), 2000);
    } catch (err: unknown) {
      setSaved(false);
      setMessage(
        typeof err === "object" && err && "message" in err
          ? `保存 AI 策略失败：${String((err as { message: string }).message)}`
          : "保存 AI 策略失败",
      );
    } finally {
      setSaving(false);
    }
  }

  function handleReset() {
    setProfile(defaultAiStrategyProfile());
    setSaved(false);
    setMessage("已重置为默认策略，点击保存后生效");
  }

  return (
    <Card padding="lg" className="space-y-6">
      <div>
        <h2 className="text-base font-semibold text-surface-100">AI 策略配置</h2>
        <p className="text-sm text-surface-400 mt-1">
          配置当前项目的 AI 生产策略，控制自动化级别、审查深度和默认能力栈。
        </p>
      </div>

      {projectRoot && loading && (
        <p className="text-xs text-surface-500">AI 策略加载中...</p>
      )}

      <div className="space-y-2">
        <h3 className="text-sm font-semibold text-surface-200">默认工作流栈</h3>
        <p className="text-xs text-surface-400">
          每行一个任务类型，定义项目默认采用的任务推进链路。
        </p>
        <Textarea
          value={workflowStackText}
          onChange={(e) => patchProfile({ defaultWorkflowStack: linesToList(e.target.value) })}
          placeholder="chapter.plan&#10;chapter.draft"
          rows={3}
        />
      </div>

      <div className="space-y-3">
        <h3 className="text-sm font-semibold text-surface-200">审查严格度</h3>
        <StrictnessControl
          value={profile.reviewStrictness}
          onChange={(value) => patchProfile({ reviewStrictness: value })}
        />
      </div>

      <div className="space-y-3">
        <h3 className="text-sm font-semibold text-surface-200">默认能力包</h3>
        <p className="text-xs text-surface-400">
          默认能力包用于增强章节生成时的稳定能力；常驻策略技能用于持续注入约束策略。
        </p>
        <Textarea
          label="默认能力包（每行一个）"
          value={capabilityBundlesText}
          onChange={(e) => patchProfile({ defaultCapabilityBundles: linesToList(e.target.value) })}
          placeholder="character-presence&#10;scene-environment"
          rows={3}
        />
        <Textarea
          label="常驻策略技能（每行一个）"
          value={policySkillsText}
          onChange={(e) => patchProfile({ alwaysOnPolicySkills: linesToList(e.target.value) })}
          placeholder="term-lock&#10;pov-guard"
          rows={3}
        />
      </div>

      <div className="space-y-3">
        <h3 className="text-sm font-semibold text-surface-200">自动持久化策略</h3>
        <p className="text-xs text-surface-400">
          配置 AI 默认自动化档位与状态写入策略，控制哪些结果自动推进、哪些需要人工确认。
        </p>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Select
            label="默认自动化档位"
            value={profile.automationDefault}
            onChange={(e) =>
              patchProfile({
                automationDefault: e.target.value as AiStrategyProfile["automationDefault"],
              })
            }
            options={[
              { value: "auto", label: "auto（自动推进）" },
              { value: "supervised", label: "supervised（待审查）" },
              { value: "confirm", label: "confirm（需确认）" },
            ]}
          />
          <Select
            label="状态写入策略"
            value={profile.stateWritePolicy}
            onChange={(e) =>
              patchProfile({
                stateWritePolicy: e.target.value as AiStrategyProfile["stateWritePolicy"],
              })
            }
            options={[
              { value: "chapter_confirmed", label: "章节确认后写入" },
              { value: "manual_only", label: "仅手动写入" },
            ]}
          />
        </div>
      </div>

      <div className="space-y-3">
        <h3 className="text-sm font-semibold text-surface-200">连续性与生成模式</h3>
        <p className="text-xs text-surface-400">
          控制上下文编译深度、章节生成流程和窗口规划范围，平衡生成质量与吞吐。
        </p>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Select
            label="Continuity Pack 深度"
            value={profile.continuityPackDepth}
            onChange={(e) =>
              patchProfile({
                continuityPackDepth: e.target.value as AiStrategyProfile["continuityPackDepth"],
              })
            }
            options={[
              { value: "minimal", label: "minimal" },
              { value: "standard", label: "standard" },
              { value: "deep", label: "deep" },
            ]}
          />
          <Select
            label="章节生成模式"
            value={profile.chapterGenerationMode}
            onChange={(e) =>
              patchProfile({
                chapterGenerationMode: e.target.value as AiStrategyProfile["chapterGenerationMode"],
              })
            }
            options={[
              { value: "draft_only", label: "draft_only" },
              { value: "plan_draft", label: "plan_draft" },
              { value: "plan_scene_draft", label: "plan_scene_draft" },
            ]}
          />
          <Input
            label="窗口规划地平线（章）"
            type="number"
            min={1}
            max={50}
            value={String(profile.windowPlanningHorizon)}
            onChange={(e) =>
              patchProfile({
                windowPlanningHorizon: Math.max(1, Math.min(50, Number(e.target.value) || 1)),
              })
            }
          />
        </div>
      </div>

      {message && (
        <div
          className={`px-3 py-2 rounded-lg text-sm ${
            saved
              ? "bg-success/10 text-success border border-success/20"
              : "bg-info/10 text-info border border-info/20"
          }`}
        >
          {message}
        </div>
      )}

      <div className="flex items-center gap-3 pt-3 border-t border-surface-700">
        <Button
          variant="primary"
          onClick={() => void handleSave()}
          disabled={!projectRoot || !loaded || saving}
          loading={saving}
        >
          {saving ? "保存中..." : saved ? "已保存 ✓" : "保存 AI 策略"}
        </Button>
        <Button
          variant="ghost"
          onClick={handleReset}
          disabled={!loaded}
        >
          重置默认
        </Button>
        {!projectRoot && (
          <span className="text-xs text-warning">请先打开项目以配置 AI 策略</span>
        )}
      </div>
    </Card>
  );
}
