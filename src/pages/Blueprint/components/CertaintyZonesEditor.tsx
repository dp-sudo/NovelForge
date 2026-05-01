import { Textarea } from "../../../components/forms/Textarea.js";
import type { BlueprintCertaintyZones } from "../../../domain/types.js";
import {
  parseCertaintyZoneText,
  stringifyCertaintyZoneText,
} from "../utils/certaintyZones.js";

interface CertaintyZoneLabels {
  frozen: string;
  promised: string;
  exploratory: string;
}

interface CertaintyZonesEditorProps {
  zones: BlueprintCertaintyZones;
  labels: CertaintyZoneLabels;
  onChange: (zones: BlueprintCertaintyZones) => void;
}

export function CertaintyZonesEditor({ zones, labels, onChange }: CertaintyZonesEditorProps) {
  const update = (key: keyof BlueprintCertaintyZones) => (value: string) =>
    onChange({ ...zones, [key]: parseCertaintyZoneText(value) });

  return (
    <div className="mt-4 rounded-xl border border-surface-700 bg-surface-800/60 p-4">
      <h3 className="text-sm font-semibold text-surface-200">确定性分区</h3>
      <p className="mt-1 text-xs text-surface-400">
        冻结区禁止改写，承诺区要求后续兑现，探索区允许继续探索和重构。
      </p>
      <div className="mt-3 grid grid-cols-1 gap-3 xl:grid-cols-3">
        <Textarea
          label={labels.frozen}
          value={stringifyCertaintyZoneText(zones.frozen)}
          onChange={(event) => update("frozen")(event.target.value)}
          placeholder="每行一条，例如：终局真相不可改写"
          helperText="命中冲突时将触发降级或审阅策略。"
        />
        <Textarea
          label={labels.promised}
          value={stringifyCertaintyZoneText(zones.promised)}
          onChange={(event) => update("promised")(event.target.value)}
          placeholder="每行一条，例如：主角将直面宗门审判"
          helperText="后续章节生成应优先兑现这些承诺。"
        />
        <Textarea
          label={labels.exploratory}
          value={stringifyCertaintyZoneText(zones.exploratory)}
          onChange={(event) => update("exploratory")(event.target.value)}
          placeholder="每行一条，例如：支线人物立场可变化"
          helperText="用于保留可变空间，避免过早锁死。"
        />
      </div>
    </div>
  );
}
