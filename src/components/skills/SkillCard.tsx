interface SkillCardProps {
  id: string;
  name: string;
  description: string;
  source: "builtin" | "user" | "imported";
  icon?: string;
  category: string;
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

export function SkillCard({ id, name, description, source, icon, active, onClick }: SkillCardProps) {
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
          </div>
          <p className="text-xs text-surface-500 mt-0.5 truncate">{description}</p>
        </div>
      </div>
    </button>
  );
}
