import { useSkillStore } from "../../stores/skillStore.js";
import { SkillCard } from "./SkillCard.js";
import type { SkillManifest } from "../../api/skillsApi.js";

interface SkillListProps {
  skills: SkillManifest[];
  filter: string;
}

export function SkillList({ skills, filter }: SkillListProps) {
  const selectedId = useSkillStore((s) => s.selectedId);
  const setSelectedId = useSkillStore((s) => s.setSelectedId);

  const q = filter.toLowerCase();
  const filtered = q
    ? skills.filter(
        (s) =>
          s.name.toLowerCase().includes(q) ||
          s.id.toLowerCase().includes(q) ||
          s.description.toLowerCase().includes(q) ||
          s.tags.some((t) => t.toLowerCase().includes(q)),
      )
    : skills;

  if (filtered.length === 0) {
    return (
      <div className="flex items-center justify-center h-32 text-sm text-surface-500">
        {q ? "无匹配技能" : "暂无技能"}
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {filtered.map((skill) => (
        <SkillCard
          key={skill.id}
          id={skill.id}
          name={skill.name}
          description={skill.description}
          source={skill.source}
          icon={skill.icon}
          category={skill.category}
          active={selectedId === skill.id}
          onClick={() => setSelectedId(skill.id)}
        />
      ))}
    </div>
  );
}
