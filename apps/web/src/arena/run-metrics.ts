import type { Json } from "@/lib/supabase/types";

type JsonObject = Record<string, Json | undefined>;

const isJsonObject = (value: Json | null | undefined): value is JsonObject =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const readNested = (source: JsonObject, path: readonly string[]): Json | undefined => {
  let current: Json | undefined = source;
  for (const key of path) {
    if (!isJsonObject(current)) {
      return undefined;
    }
    current = current[key];
  }
  return current;
};

const toFiniteNumber = (value: Json | undefined): number | null => {
  if (typeof value === "number" && Number.isFinite(value)) {
    return Math.trunc(value);
  }
  if (typeof value === "string") {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      return Math.trunc(parsed);
    }
  }
  return null;
};

const pickCount = (metrics: Json | null | undefined, paths: readonly (readonly string[])[]): number | null => {
  if (!isJsonObject(metrics)) {
    return null;
  }
  for (const path of paths) {
    const candidate = toFiniteNumber(readNested(metrics, path));
    if (candidate !== null) {
      return candidate;
    }
  }
  return null;
};

const STEP_COUNT_PATHS = [
  ["steps_taken"],
  ["step_count"],
  ["steps"],
  ["total_steps"],
  ["movement_steps"],
  ["player_steps"],
  ["movement", "steps"],
  ["run", "steps"],
  ["stats", "steps"],
] as const;

const COMMAND_COUNT_PATHS = [
  ["command_count"],
  ["commands_run"],
  ["commands"],
  ["total_commands"],
  ["tool_calls"],
  ["action_count"],
  ["input_count"],
  ["run", "commands"],
  ["stats", "commands"],
] as const;

const formatSpecies = (value: string): string => {
  const normalized = value.trim().replace(/[_-]+/g, " ").toLowerCase();
  if (!normalized) {
    return "";
  }
  return normalized.replace(/\b\w/g, (char) => char.toUpperCase());
};

type TeamMember = { species: string; level: number | null };

const readTeamFromArray = (value: Json | undefined): TeamMember[] => {
  if (!Array.isArray(value)) {
    return [];
  }
  const members: TeamMember[] = [];
  for (const entry of value) {
    if (!isJsonObject(entry)) {
      continue;
    }
    const speciesRaw = entry.species;
    const levelRaw = entry.level;
    if (typeof speciesRaw !== "string") {
      continue;
    }
    const species = formatSpecies(speciesRaw);
    if (!species) {
      continue;
    }
    const level = toFiniteNumber(levelRaw);
    members.push({ species, level });
  }
  return members;
};

const readPartyMembers = (metrics: JsonObject): TeamMember[] => {
  const direct = readTeamFromArray(metrics.team);
  if (direct.length) {
    return direct;
  }
  const party = metrics.party;
  if (isJsonObject(party)) {
    const fromParty = readTeamFromArray(party.pokemon);
    if (fromParty.length) {
      return fromParty;
    }
  }
  return [];
};

export const extractStepCount = (metrics: Json | null | undefined): number | null =>
  pickCount(metrics, STEP_COUNT_PATHS);

export const extractCommandCount = (metrics: Json | null | undefined): number | null =>
  pickCount(metrics, COMMAND_COUNT_PATHS);

export const extractTeamSummary = (metrics: Json | null | undefined, maxMembers = 3): string | null => {
  if (!isJsonObject(metrics)) {
    return null;
  }
  const members = readPartyMembers(metrics);
  if (members.length) {
    const safeMax = Math.max(1, maxMembers);
    const displayed = members.slice(0, safeMax).map((member) =>
      member.level !== null ? `${member.species} Lv${member.level}` : member.species
    );
    const remaining = members.length - displayed.length;
    return remaining > 0 ? `${displayed.join(", ")} +${remaining}` : displayed.join(", ");
  }

  const partySummary = metrics.party_summary;
  if (isJsonObject(partySummary)) {
    const count = toFiniteNumber(partySummary.count);
    const leadSpecies =
      typeof partySummary.lead_species === "string" ? formatSpecies(partySummary.lead_species) : "";
    if (leadSpecies) {
      if (count && count > 1) {
        return `${leadSpecies} +${count - 1}`;
      }
      return leadSpecies;
    }
    if (count && count > 0) {
      return `${count} Pokemon`;
    }
  }
  return null;
};
