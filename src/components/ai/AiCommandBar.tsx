import { useState } from "react";

interface AiCommand {
  id: string;
  label: string;
  taskType: string;
}

const COMMANDS: AiCommand[] = [
  { id: "draft", label: "生成草稿", taskType: "generate_chapter_draft" },
  { id: "plan", label: "章节计划", taskType: "chapter_plan" },
  { id: "continue", label: "续写", taskType: "continue_chapter" },
  { id: "rewrite", label: "改写", taskType: "rewrite_selection" },
  { id: "naturalize", label: "去 AI 味", taskType: "deai_text" },
  { id: "check", label: "检查", taskType: "scan_consistency" }
];

interface AiCommandBarProps {
  onCommand: (taskType: string, userInstruction: string) => void;
  disabled: boolean;
}

export function AiCommandBar({ onCommand, disabled }: AiCommandBarProps) {
  const [activeCommand, setActiveCommand] = useState<string | null>(null);
  const [instruction, setInstruction] = useState("");

  function handleCommand(cmd: AiCommand) {
    setActiveCommand(cmd.id);
    if (!instruction.trim()) {
      onCommand(cmd.taskType, cmd.label);
    } else {
      onCommand(cmd.taskType, instruction);
    }
    setInstruction("");
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex gap-2">
        {COMMANDS.map((cmd) => (
          <button
            key={cmd.id}
            onClick={() => handleCommand(cmd)}
            disabled={disabled}
            className={`px-3 py-1.5 text-xs rounded-lg transition-colors disabled:opacity-40 ${
              activeCommand === cmd.id
                ? "bg-primary/20 text-primary border border-primary/30"
                : "bg-surface-700 text-surface-300 border border-surface-600 hover:bg-surface-600"
            }`}
          >
            {cmd.label}
          </button>
        ))}
      </div>
      <div className="flex gap-2">
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
