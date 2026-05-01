import { Card } from "../../../components/cards/Card.js";
import { Button } from "../../../components/ui/Button.js";
import { Textarea } from "../../../components/forms/Textarea.js";

interface BookPipelinePanelProps {
  ideaPrompt: string;
  onIdeaPromptChange: (value: string) => void;
  running: boolean;
  status: string | null;
  logs: string[];
  onRun: () => void;
  onCancel: () => void;
  runDisabled: boolean;
  projectReady: boolean;
  title: string;
}

export function BookPipelinePanel({
  ideaPrompt,
  onIdeaPromptChange,
  running,
  status,
  logs,
  onRun,
  onCancel,
  runDisabled,
  projectReady,
  title,
}: BookPipelinePanelProps) {
  return (
    <Card padding="md" className="mt-3">
      <h3 className="text-sm font-semibold text-surface-200 mb-3">{title}</h3>
      <Textarea
        label="创意提示词"
        value={ideaPrompt}
        onChange={(event) => onIdeaPromptChange(event.target.value)}
        placeholder="输入核心创意，按阶段自动生成蓝图/角色/设定/剧情"
      />
      <div className="mt-3 flex gap-2">
        <Button
          variant="primary"
          size="sm"
          className="flex-1 justify-center"
          loading={running}
          onClick={onRun}
          disabled={runDisabled}
        >
          {running ? "编排中..." : "开始编排"}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="justify-center"
          onClick={onCancel}
          disabled={!running}
        >
          取消
        </Button>
      </div>
      {!projectReady && <p className="mt-3 text-xs text-warning">请先打开项目</p>}
      {status && <p className="mt-3 text-xs text-surface-300">{status}</p>}
      {logs.length > 0 && (
        <div className="mt-3 max-h-28 overflow-y-auto rounded-lg border border-surface-700 bg-surface-800/80 p-2">
          {logs.map((log, idx) => (
            <p key={`${log}-${idx}`} className="text-[11px] text-surface-300">
              {log}
            </p>
          ))}
        </div>
      )}
    </Card>
  );
}
