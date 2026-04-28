import { useEffect, useState } from "react";
import { listSkills, type SkillManifest } from "../../api/skillsApi.js";

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
  utility: "工具",
};

const CATEGORY_ORDER = ["writing", "character", "world", "plot", "review", "utility"];

/** Skills whose IDs are excluded from the command bar (internal skills). */
const HIDDEN_SKILLS = new Set(["context.collect", "import.extract_assets"]);

export function AiCommandBar({ onCommand, disabled }: AiCommandBarProps) {
  const [skills, setSkills] = useState<SkillManifest[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [instruction, setInstruction] = useState("");

  useEffect(() => {
    listSkills().then((list) => {
      setSkills(list.filter((s) => !HIDDEN_SKILLS.has(s.id)));
    }).catch(() => {});
  }, []);

  function handleCommand(skill: SkillManifest) {
    setActiveId(skill.id);
    const userMsg = instruction.trim() || skill.name;
    onCommand(skill.id, userMsg);
    setInstruction("");
  }

  // Group by category preserving order
  const groups = CATEGORY_ORDER
    .map((cat) => ({
      category: cat,
      label: CATEGORY_LABELS[cat] || cat,
      skills: skills.filter((s) => s.category === cat),
    }))
    .filter((g) => g.skills.length > 0);

  return (
    <div className="flex flex-col gap-2">
      {groups.map((group) => (
        <div key={group.category}>
          <p className="text-[10px] font-medium text-surface-500 uppercase tracking-wider mb-1.5">
            {group.label}
          </p>
          <div className="flex gap-1.5 flex-wrap">
            {group.skills.map((skill) => (
              <button
                key={skill.id}
                onClick={() => handleCommand(skill)}
                disabled={disabled}
                className={`inline-flex items-center gap-1 px-2.5 py-1.5 text-xs rounded-lg transition-colors disabled:opacity-40 ${
                  activeId === skill.id
                    ? "bg-primary/20 text-primary border border-primary/30"
                    : "bg-surface-700 text-surface-300 border border-surface-600 hover:bg-surface-600"
                }`}
              >
                {skill.icon && <span>{skill.icon}</span>}
                {skill.name}
              </button>
            ))}
          </div>
        </div>
      ))}
      <div className="flex gap-2 mt-1">
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
