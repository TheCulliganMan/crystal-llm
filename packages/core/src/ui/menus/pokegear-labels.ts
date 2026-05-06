import { LandmarkEntry, loadPokegearPayloadSync } from "@pokecrystal/assets/content/pokegear";

let landmarkLabelCache: Record<string, string> | null = null;

const formatLandmarkName = (name: string): string => String(name ?? "").toUpperCase();

function landmarkLabelMap(): Record<string, string> {
  if (landmarkLabelCache) {
    return landmarkLabelCache;
  }
  const payload = loadPokegearPayloadSync();
  landmarkLabelCache = {};
  for (const entry of payload.landmarks) {
    const text = formatLandmarkName(entry.name);
    if (entry.label) {
      landmarkLabelCache[entry.label] = text;
    }
  }
  return landmarkLabelCache;
}

export function getLandmarkLabel(label: string): string | undefined {
  const mapping = landmarkLabelMap();
  return mapping[label];
}

export function resolveLandmarkText(entry: LandmarkEntry): string {
  const labelValue = entry["label"];
  if (labelValue) {
    const text = getLandmarkLabel(String(labelValue));
    if (text) {
      return text;
    }
  }
  return String(entry["name"] ?? "");
}
