import type { BlueprintCertaintyZones } from "../../../domain/types.js";

export const EMPTY_CERTAINTY_ZONES: BlueprintCertaintyZones = {
  frozen: [],
  promised: [],
  exploratory: [],
};

export function normalizeCertaintyItem(raw: string): string {
  return raw
    .trim()
    .replace(/^[-*]\s*/, "")
    .replace(/^\d+\.\s*/, "")
    .trim();
}

export function parseCertaintyZoneText(raw: string): string[] {
  const lines = raw
    .split(/\r?\n|[;；]/)
    .map((line) => normalizeCertaintyItem(line))
    .filter((line) => line.length > 0);
  return Array.from(new Set(lines)).slice(0, 24);
}

export function stringifyCertaintyZoneText(items: string[]): string {
  return items.join("\n");
}

export function hasCertaintyZones(zones: BlueprintCertaintyZones): boolean {
  return zones.frozen.length > 0 || zones.promised.length > 0 || zones.exploratory.length > 0;
}

export function validateCertaintyZones(zones: BlueprintCertaintyZones): string[] {
  const ownership = new Map<string, string>();
  const overlaps = new Set<string>();
  const register = (zoneLabel: string, entries: string[]) => {
    for (const entry of entries) {
      const normalized = normalizeCertaintyItem(entry).toLowerCase();
      if (!normalized) continue;
      const existing = ownership.get(normalized);
      if (!existing) {
        ownership.set(normalized, zoneLabel);
        continue;
      }
      if (existing !== zoneLabel) {
        overlaps.add(`${entry}（${existing} / ${zoneLabel}）`);
      }
    }
  };

  register("冻结区", zones.frozen);
  register("承诺区", zones.promised);
  register("探索区", zones.exploratory);

  return Array.from(overlaps).slice(0, 8);
}

export function parseCertaintyZonesFromLegacyContent(content: string): BlueprintCertaintyZones {
  if (!content.trim()) return { ...EMPTY_CERTAINTY_ZONES };
  try {
    const parsed = JSON.parse(content) as {
      certaintyZones?: Partial<BlueprintCertaintyZones>;
      certainty_zones?: Partial<BlueprintCertaintyZones>;
      frozen?: string[] | string;
      promised?: string[] | string;
      exploratory?: string[] | string;
    };
    const candidate = parsed.certaintyZones ?? parsed.certainty_zones ?? parsed;
    const list = (value: unknown): string[] => {
      if (typeof value === "string") return parseCertaintyZoneText(value);
      if (Array.isArray(value)) {
        return Array.from(
          new Set(
            value
              .filter((item): item is string => typeof item === "string")
              .map((item) => normalizeCertaintyItem(item))
              .filter((item) => item.length > 0),
          ),
        ).slice(0, 24);
      }
      return [];
    };
    const zones = {
      frozen: list(candidate.frozen),
      promised: list(candidate.promised),
      exploratory: list(candidate.exploratory),
    };
    if (hasCertaintyZones(zones)) return zones;
  } catch {
    // fall through
  }

  enum Zone {
    Frozen = "frozen",
    Promised = "promised",
    Exploratory = "exploratory",
  }
  const zones: BlueprintCertaintyZones = { ...EMPTY_CERTAINTY_ZONES };
  let current: Zone | null = null;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;
    if (line.includes("冻结区")) {
      current = Zone.Frozen;
      continue;
    }
    if (line.includes("承诺区")) {
      current = Zone.Promised;
      continue;
    }
    if (line.includes("探索区")) {
      current = Zone.Exploratory;
      continue;
    }
    if (!current) continue;
    const normalized = normalizeCertaintyItem(line);
    if (!normalized) continue;
    if (!zones[current].includes(normalized) && zones[current].length < 24) {
      zones[current].push(normalized);
    }
  }
  return zones;
}
