import { useEffect, useState } from "react";
import { listSkills, type SkillManifest } from "../../api/skillsApi.js";
import {
  EDITOR_AI_ACTIONS,
  type EditorAiAction,
  type EditorAiCategory,
} from "../../utils/taskRouting.js";

interface AiCommandBarProps {
  onCommand: (taskType: string, userInstruction: string) => void;
  disabled: boolean;
}

const CATEGORY_LABELS: Record<string, string> = {
  writing: "写作",
  character: "角色",
  world: "世界观",
  plot: "剧情",
  review: "审稿",
};

const CATEGORY_ORDER: EditorAiCategory[] = ["writing", "character", "world", "plot", "review"];

/** Skills whose IDs are excluded from the command bar (internal skills). */
const HIDDEN_SKILLS = new Set(["context.collect", "import.extract_assets"]);

export function AiCommandBar({ onCommand, disabled }: AiCommandBarProps) {
  const [skills, setSkills] = useState<SkillManifest[]>([]);
  const [activeTaskType, setActiveTaskType] = useState<string | null>(null);
  const [instruction, setInstruction] = useState("");

  useEffect(() => {
    listSkills().then((list) => {
      setSkills(list.filter((s) => !HIDDEN_SKILLS.has(s.id)));
    }).catch(() => {});
  }, []);

  function handleCommand(action: EditorAiAction) {
    const taskType = action.taskType;
    const userMsg = instruction.trim() || action.label;
    setActiveTaskType(taskType);
    onCommand(taskType, userMsg);
    setInstruction("");
  }

  const skillById = new Map(skills.map((skill) => [skill.id, skill] as const));

  // 固定渲染编辑器 9 按钮，按 category 分组
  const groups = CATEGORY_ORDER
    .map((cat) => ({
      category: cat,
      label: CATEGORY_LABELS[cat] || cat,
      actions: EDITOR_AI_ACTIONS.filter((action) => action.category === cat),
    }))
    .filter((g) => g.actions.length > 0);

  return (
    <div className="rounded-xl border border-surface-700 bg-surface-900/40 p-3">
      <div className="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-2">
        {groups.map((group) => (
          <div key={group.category} className="rounded-lg border border-surface-700/80 bg-surface-800/35 p-2">
            <p className="text-[10px] font-medium text-surface-500 uppercase tracking-wider mb-1.5">
              {group.label}
            </p>
            <div className="flex gap-1.5 flex-wrap">
              {group.actions.map((action) => {
                const skill = skillById.get(action.taskType);
                const buttonLabel = skill?.name || action.label;
                const icon = skill?.icon;
                return (
                <button
                  key={action.taskType}
                  onClick={() => handleCommand(action)}
                  disabled={disabled}
                  className={`inline-flex items-center gap-1 px-2.5 py-1.5 text-xs rounded-lg transition-colors disabled:opacity-40 ${
                    activeTaskType === action.taskType
                      ? "bg-primary/20 text-primary border border-primary/30"
                      : "bg-surface-700 text-surface-300 border border-surface-600 hover:bg-surface-600"
                  }`}
                >
                  {icon && <span>{icon}</span>}
                  {buttonLabel}
                </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>
      <div className="flex gap-2 mt-3">
        <input
          type="text"
          value={instruction}
          onChange={(e) => setInstruction(e.target.value)}
          placeholder="输入自定义指令，或直接点击上方按钮..."
          disabled={disabled}
          className="flex-1 px-3 py-1.5 text-xs bg-surface-800 border border-surface-600 rounded-lg text-surface-200 placeholder-surface-500 focus:outline-none focus:border-primary/50 disabled:opacity-40"
          onKeyDown={(e) => {
            if (e.key === "Enter" && instruction.trim() && !disabled) {
              onCommand("custom", instruction);
              setInstruction("");
            }
          }}
        />
        {instruction.trim() && (
          <button
            onClick={() => {
              onCommand("custom", instruction);
              setInstruction("");
            }}
            disabled={disabled}
            className="px-3 py-1.5 text-xs bg-primary/20 text-primary border border-primary/30 rounded-lg hover:bg-primary/30 transition-colors disabled:opacity-40"
          >
            发送
          </button>
        )}
      </div>
    </div>
  );
}
