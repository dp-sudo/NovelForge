interface SkillCardProps {
  name: string;
  description: string;
  source: "builtin" | "user" | "imported";
  icon?: string;
  skillClass?: "workflow" | "capability" | "extractor" | "review" | "policy";
  active: boolean;
  onClick: () => void;
}

const SOURCE_LABELS: Record<string, string> = {
  builtin: "内置",
  user: "自定义",
  imported: "已导入",
};

const SOURCE_COLORS: Record<string, string> = {
  builtin: "bg-info/10 text-info border-info/30",
  user: "bg-success/10 text-success border-success/30",
  imported: "bg-warning/10 text-warning border-warning/30",
};

const SKILL_CLASS_LABELS: Record<string, string> = {
  workflow: "Workflow",
  capability: "Capability",
  extractor: "Extractor",
  review: "Review",
  policy: "Policy",
};

const SKILL_CLASS_COLORS: Record<string, string> = {
  workflow: "bg-primary/15 text-primary border-primary/40",
  capability: "bg-success/15 text-success border-success/40",
  extractor: "bg-warning/15 text-warning border-warning/40",
  review: "bg-info/15 text-info border-info/40",
  policy: "bg-error/15 text-error border-error/40",
};

export function SkillCard({ name, description, source, icon, skillClass, active, onClick }: SkillCardProps) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left px-3 py-2.5 rounded-lg transition-colors border ${
        active
          ? "bg-primary/10 border-primary/30 text-primary"
          : "bg-transparent border-transparent text-surface-200 hover:bg-surface-800 hover:border-surface-700"
      }`}
    >
      <div className="flex items-center gap-2">
        {icon && <span className="text-lg shrink-0">{icon}</span>}
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium truncate">{name}</span>
            <span className={`text-[10px] px-1.5 py-0.5 rounded-full border ${SOURCE_COLORS[source] || SOURCE_COLORS.builtin}`}>
              {SOURCE_LABELS[source] || source}
            </span>
            <span
              className={`text-[10px] px-1.5 py-0.5 rounded-full border ${
                skillClass ? SKILL_CLASS_COLORS[skillClass] || "bg-surface-700 text-surface-300 border-surface-600" : "bg-surface-700 text-surface-300 border-surface-600"
              }`}
            >
              {skillClass ? SKILL_CLASS_LABELS[skillClass] || skillClass : "Unclassified"}
            </span>
          </div>
          <p className="text-xs text-surface-500 mt-0.5 truncate">{description}</p>
        </div>
      </div>
    </button>
  );
}
