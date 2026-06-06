import { normalizeAsmSlug } from "@pokecrystal/core/audio-export/asm-programs";
import { DISASSEMBLY_MUSIC_ALIASES, DISASSEMBLY_SFX_ALIASES } from "@pokecrystal/core/engine/systems/audio-aliases";
import pokemonCries from "../../../assets/data/pokemon_cries.json";

export type AudioTestGroup = "music" | "sfx" | "cry";

export type AudioTestEntry = {
  id: string;
  group: AudioTestGroup;
  token: string;
  title: string;
  source: string;
  stem: string;
  detail: string;
  index: number;
};

export type AudioTestStats = Record<AudioTestGroup, number>;

export type AudioTestCatalog = {
  entries: AudioTestEntry[];
  stats: AudioTestStats;
  total: number;
};

type CryTableEntry = {
  cry?: string | null;
  pitch?: number;
  length?: number;
};

const AUDIO_API_BASE = "/api/audio/pcm";

const titleWord = (value: string): string => {
  if (!value) {
    return value;
  }
  if (/^\d/.test(value)) {
    return value;
  }
  if (value.length <= 3) {
    return value.toUpperCase();
  }
  return `${value.charAt(0).toUpperCase()}${value.slice(1).toLowerCase()}`;
};

const titleFromToken = (token: string): string =>
  token
    .replace(/^(MUSIC|SFX|CRY)_/, "")
    .split("_")
    .filter(Boolean)
    .map(titleWord)
    .join(" ");

const stemFromAlias = (value: string): string => {
  const parts = value.split("/").filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : value;
};

const formatSpeciesName = (value: string): string =>
  value
    .split("_")
    .filter(Boolean)
    .map(titleWord)
    .join(" ");

const numberedEntry = (
  entries: AudioTestEntry[],
  entry: Omit<AudioTestEntry, "index">,
): void => {
  entries.push({ ...entry, index: entries.length + 1 });
};

export function buildAudioTestCatalog(): AudioTestCatalog {
  const entries: AudioTestEntry[] = [];

  for (const [token, stem] of Object.entries(DISASSEMBLY_MUSIC_ALIASES)) {
    if (token === "MUSIC_NONE") {
      continue;
    }
    numberedEntry(entries, {
      id: `music:${token}`,
      group: "music",
      token,
      title: titleFromToken(token),
      source: `${AUDIO_API_BASE}/music/${stem}.json`,
      stem,
      detail: stem,
    });
  }

  for (const [token, sourceStem] of Object.entries(DISASSEMBLY_SFX_ALIASES)) {
    const stem = stemFromAlias(sourceStem);
    numberedEntry(entries, {
      id: `sfx:${token}`,
      group: "sfx",
      token,
      title: titleFromToken(token),
      source: `${AUDIO_API_BASE}/sfx/${stem}.json`,
      stem,
      detail: sourceStem,
    });
  }

  const criesByToken = new Map<string, Set<string>>();
  for (const [owner, entry] of Object.entries(pokemonCries as Record<string, CryTableEntry>)) {
    const token = entry.cry?.trim().toUpperCase();
    if (!token?.startsWith("CRY_")) {
      continue;
    }
    const owners = criesByToken.get(token) ?? new Set<string>();
    if (!/^\d+$/.test(owner)) {
      owners.add(owner);
    }
    criesByToken.set(token, owners);
  }

  for (const [token, owners] of Array.from(criesByToken.entries()).sort(([left], [right]) => left.localeCompare(right))) {
    const stem = normalizeAsmSlug(token.replace(/^CRY_/, ""));
    const namedOwners = Array.from(owners).sort((left, right) => left.localeCompare(right));
    const preview = namedOwners.slice(0, 4).map(formatSpeciesName).join(", ");
    const detail = namedOwners.length > 4
      ? `${preview}, +${namedOwners.length - 4}`
      : preview || stem;
    numberedEntry(entries, {
      id: `cry:${token}`,
      group: "cry",
      token,
      title: titleFromToken(token),
      source: `${AUDIO_API_BASE}/cries/${stem}.json`,
      stem,
      detail,
    });
  }

  const stats: AudioTestStats = {
    music: entries.filter((entry) => entry.group === "music").length,
    sfx: entries.filter((entry) => entry.group === "sfx").length,
    cry: entries.filter((entry) => entry.group === "cry").length,
  };

  return {
    entries,
    stats,
    total: entries.length,
  };
}
